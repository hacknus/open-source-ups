use crate::battery::Measurements;
use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use stm32f4xx_hal::adc::Adc;
use stm32f4xx_hal::dma::{PeripheralToMemory, Stream0, Transfer};
use stm32f4xx_hal::pac::{interrupt, ADC1, DMA2};

pub type Buffer = &'static mut [u16; 4];
pub type DmaTransfer = Transfer<Stream0<DMA2>, 0, Adc<ADC1>, PeripheralToMemory, Buffer>;
pub static TRANSFER: Mutex<RefCell<Option<DmaTransfer>>> = Mutex::new(RefCell::new(None));
pub static SPARE: Mutex<RefCell<Option<Buffer>>> = Mutex::new(RefCell::new(None));

#[derive(Clone, Copy)]
struct Scan {
    raw: [u16; 4],
    sequence: u32,
}
static SCAN: Mutex<RefCell<Option<Scan>>> = Mutex::new(RefCell::new(None));
struct Averager {
    sums: [u32; 4],
    count: u32,
}
static AVERAGE: Mutex<RefCell<Averager>> = Mutex::new(RefCell::new(Averager {
    sums: [0; 4],
    count: 0,
}));
static SCALE_V: Mutex<RefCell<f32>> = Mutex::new(RefCell::new(0.0));

pub fn set_scale(volts_per_count: f32) {
    cortex_m::interrupt::free(|cs| *SCALE_V.borrow(cs).borrow_mut() = volts_per_count);
}

/// Return a complete scan once; a missing completion is never converted to zero volts.
pub fn measurements(last_sequence: &mut u32) -> Option<Measurements> {
    let (scan, scale) =
        cortex_m::interrupt::free(|cs| (*SCAN.borrow(cs).borrow(), *SCALE_V.borrow(cs).borrow()));
    let scan = scan?;
    if scan.sequence == *last_sequence {
        return None;
    }
    *last_sequence = scan.sequence;
    Measurements::from_adc_volts(
        scan.raw[1] as f32 * scale,
        scan.raw[2] as f32 * scale,
        scan.raw[3] as f32 * scale,
    )
}

pub fn start_scan() {
    cortex_m::interrupt::free(|cs| {
        if let Some(transfer) = TRANSFER.borrow(cs).borrow_mut().as_mut() {
            transfer.start(|adc| adc.start_conversion());
        }
    });
}

#[interrupt]
fn DMA2_STREAM0() {
    cortex_m::interrupt::free(|cs| {
        let mut transfer = TRANSFER.borrow(cs).borrow_mut();
        let Some(transfer) = transfer.as_mut() else {
            return;
        };
        let Some(spare) = SPARE.borrow(cs).borrow_mut().take() else {
            return;
        };
        match transfer.next_transfer(spare) {
            Ok((completed, _)) => {
                let mut average = AVERAGE.borrow(cs).borrow_mut();
                for (sum, value) in average.sums.iter_mut().zip(completed.iter()) {
                    *sum += *value as u32;
                }
                average.count += 1;
                if average.count == 16 {
                    let mut snapshot = SCAN.borrow(cs).borrow_mut();
                    let sequence = snapshot.map_or(1, |s| s.sequence.wrapping_add(1));
                    *snapshot = Some(Scan {
                        raw: average.sums.map(|sum| (sum / 16) as u16),
                        sequence,
                    });
                    average.sums = [0; 4];
                    average.count = 0;
                }
                SPARE.borrow(cs).replace(Some(completed));
            }
            Err(error) => {
                use stm32f4xx_hal::dma::DMAError;
                let (DMAError::NotReady(buffer)
                | DMAError::SmallBuffer(buffer)
                | DMAError::Overrun(buffer)) = error;
                SPARE.borrow(cs).replace(Some(buffer));
                SCAN.borrow(cs).replace(None);
            }
        }
    });
}
