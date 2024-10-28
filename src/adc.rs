use core::cell::RefCell;

// use arrform::{arrform, ArrForm};
use cortex_m::interrupt::Mutex;
use freertos_rust::{CurrentTask, Duration};
use stm32f4xx_hal::adc::Adc;
use stm32f4xx_hal::dma::{PeripheralToMemory, Stream0, Transfer};
use stm32f4xx_hal::pac::{ADC1, DMA2};
use stm32f4xx_hal::pac::interrupt;

// use crate::usb::usb_println;

type DMATransfer = Transfer<Stream0<DMA2>, 0, Adc<ADC1>, PeripheralToMemory, &'static mut [u16; 4]>;

pub static G_XFR: Mutex<RefCell<Option<DMATransfer>>> = Mutex::new(RefCell::new(None));
pub static G_VBAT: Mutex<RefCell<Option<f32>>> = Mutex::new(RefCell::new(None));
pub static G_VIN: Mutex<RefCell<Option<f32>>> = Mutex::new(RefCell::new(None));
pub static G_CURRENT: Mutex<RefCell<Option<f32>>> = Mutex::new(RefCell::new(None));
pub static G_ADC_BUF: Mutex<RefCell<Option<[u16; 4]>>> = Mutex::new(RefCell::new(None));

pub static mut ADC_MEMORY: [u16; 4] = [0u16; 4];

///
///
/// measures the current in Ampere
///
/// returns: f32
///
///
pub fn read_current() -> f32 {
    let mut current = 0.0;

    cortex_m::interrupt::free(|cs| match G_CURRENT.borrow(cs).borrow_mut().as_mut() {
        None => {}
        Some(sampled_voltage) => {
            current = *sampled_voltage / 33.0; // sens = 33 mV / A
        }
    });
    // add some delay
    CurrentTask::delay(Duration::ms(2));
    // trigger a new conversion (is this really needed?)
    cortex_m::interrupt::free(|cs| {
        if let Some(transfer) = G_XFR.borrow(cs).borrow_mut().as_mut() {
            transfer.start(|adc| {
                adc.start_conversion();
            });
        };
    });
    current
}

pub fn read_v_bat() -> f32 {
    let mut voltage = 0.0;

    cortex_m::interrupt::free(|cs| match G_VBAT.borrow(cs).borrow_mut().as_mut() {
        None => {}
        Some(sampled_voltage) => {
            voltage = *sampled_voltage / 3.4 * 12.0;
        }
    });
    // add some delay
    CurrentTask::delay(Duration::ms(2));
    // trigger a new conversion (is this really needed?)
    cortex_m::interrupt::free(|cs| {
        if let Some(transfer) = G_XFR.borrow(cs).borrow_mut().as_mut() {
            transfer.start(|adc| {
                adc.start_conversion();
            });
        };
    });
    voltage
}

pub fn read_v_in() -> f32 {
    let mut voltage = 0.0;

    cortex_m::interrupt::free(|cs| match G_VIN.borrow(cs).borrow_mut().as_mut() {
        None => {}
        Some(sampled_voltage) => {
            voltage = *sampled_voltage / 3.4 * 12.0;
        }
    });
    // add some delay
    CurrentTask::delay(Duration::ms(2));
    // trigger a new conversion (is this really needed?)
    cortex_m::interrupt::free(|cs| {
        if let Some(transfer) = G_XFR.borrow(cs).borrow_mut().as_mut() {
            transfer.start(|adc| {
                adc.start_conversion();
            });
        };
    });
    voltage
}

#[interrupt]
#[allow(non_snake_case)]
fn DMA2_STREAM0() {
    cortex_m::interrupt::free(|cs| {
        if let Some(xfer) = G_XFR.borrow(cs).borrow_mut().as_mut() {
            unsafe {
                if let Ok((buffer, _)) = xfer.next_transfer(&mut ADC_MEMORY) {
                    let sample_to_millivolts = xfer.peripheral().make_sample_to_millivolts();

                    //println!("DMA1_CH1 IRQ: results: {:?}", buf).unwrap();
                    G_VBAT.borrow(cs).replace(Some(
                        sample_to_millivolts(buffer[1]) as f32 / 1000.0,
                    ));
                    G_VIN.borrow(cs).replace(Some(
                        sample_to_millivolts(buffer[2]) as f32 / 1000.0,
                    ));
                    G_CURRENT.borrow(cs).replace(Some(
                        sample_to_millivolts(buffer[3]) as f32 / 1000.0,
                    ));
                    G_ADC_BUF.borrow(cs).replace(Some(*buffer));
                }
            }
        } else {
            //println!("DMA1_CH1 IRQ: ERR: no xfer").unwrap();
        }
    });
}
