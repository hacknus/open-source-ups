//#![deny(unsafe_code)]
#![no_main]
#![no_std]
// For allocator
#![feature(lang_items)]
#![feature(alloc_error_handler)]

extern crate alloc;

use cortex_m::asm;
use cortex_m_rt::exception;
use cortex_m_rt::{entry, ExceptionFrame};
use panic_halt as _;
use stm32f4xx_hal::otg_fs::{USB, UsbBus};
use stm32f4xx_hal::{
    pac::{self, Interrupt},
    gpio::{Edge},
    prelude::*,
};
use crate::commands::{process_ups_command};
use crate::devices::led::LED;
use crate::intrpt::{G_BUTTON, G_STATE};

use freertos_rust::*;
use core::alloc::Layout;
use usb_device::bus::{UsbBusAllocator};
use usb_device::device::{UsbDeviceBuilder, UsbVidPid};

mod devices;
mod commands;
mod intrpt;
mod usb;

#[global_allocator]
static GLOBAL: FreeRtosAllocator = FreeRtosAllocator;

use usbd_hid::descriptor::{SerializedDescriptor, generator_prelude::*};
use usbd_hid::hid_class::HIDClass;
use crate::usb::{G_USB_HID, usb_init};

const HID_PD_IPRODUCT: u8 = 1;
const HID_PD_SERIAL: u8 = 2;
const HID_PD_MANUFACTURER: u8 = 3;
const HID_PD_RECHARGEABLE: u8 = 6;
const HID_PD_IDEVICECHEMISTRY: u8 = 0x04;
const HID_PD_IOEMINFORMATION: u8 = 32;
const HID_PD_CAPACITYMODE: u8 = 22;
const HID_PD_CPCTYGRANULARITY1: u8 = 16;
const HID_PD_CPCTYGRANULARITY2: u8 = 24;
const HID_PD_FULLCHRGECAPACITY: u8 = 14;
const HID_PD_DESIGNCAPACITY: u8 = 23;
const HID_PD_REMAININGCAPACITY: u8 = 12;
const HID_PD_WARNCAPACITYLIMIT: u8 = 15;
const HID_PD_REMNCAPACITYLIMIT: u8 = 17;
const HID_PD_MANUFACTUREDATE: u8 = 9;
const HID_PD_AVERAGETIME2FULL: u8 = 26;
const HID_PD_AVERAGETIME2EMPTY: u8 = 28;
const HID_PD_RUNTIMETOEMPTY: u8 = 13;
const HID_PD_REMAINTIMELIMIT: u8 = 8;
const HID_PD_DELAYBE4SHUTDOWN: u8 = 18;
const HID_PD_DELAYBE4REBOOT: u8 = 19;
const HID_PD_CONFIGVOLTAGE: u8 = 10;
const HID_PD_VOLTAGE: u8 = 11;
const HID_PD_AUDIBLEALARMCTRL: u8 = 20;
const HID_PD_PRESENTSTATUS: u8 = 7;

const IPRODUCT: u8 = 2;
const ISERIAL: u8 = 3;
const IMANUFACTURER: u8 = 1;
const IDEVICECHEMISTRY: u8 = 4;
const IOEMVENDOR: u8 = 5;

static HID_REPORT_DESCRIPTOR: &[u8] = &[
    0x05, 0x84, // USAGE_PAGE (Power Device)
    0x09, 0x04, // USAGE (UPS)
    0xA1, 0x01, // COLLECTION (Application)
    0x09, 0x24, //   USAGE (Sink)
    0xA1, 0x02, //   COLLECTION (Logical)
    0x75, 0x08, //     REPORT_SIZE (8)
    0x95, 0x01, //     REPORT_COUNT (1)
    0x15, 0x00, //     LOGICAL_MINIMUM (0)
    0x26, 0xFF, 0x00, //     LOGICAL_MAXIMUM (255)
    0x85, HID_PD_IPRODUCT, //     REPORT_ID (HID_PD_IPRODUCT)
    0x09, 0xFE, //     USAGE (iProduct)
    0x79, IPRODUCT, //     STRING INDEX (IPRODUCT)
    0xB1, 0x23, //     FEATURE (Constant, Variable, Absolute, Nonvolatile)
    0x85, HID_PD_SERIAL, //     REPORT_ID (HID_PD_SERIAL)
    0x09, 0xFF, //     USAGE (iSerialNumber)
    0x79, ISERIAL, //     STRING INDEX (ISERIAL)
    0xB1, 0x23, //     FEATURE (Constant, Variable, Absolute, Nonvolatile)
    0x85, HID_PD_MANUFACTURER, //     REPORT_ID (HID_PD_MANUFACTURER)
    0x09, 0xFD, //     USAGE (iManufacturer)
    0x79, IMANUFACTURER, //     STRING INDEX (IMANUFACTURER)
    0xB1, 0x23, //     FEATURE (Constant, Variable, Absolute, Nonvolatile)
    0x05, 0x85, //     USAGE_PAGE (Battery System)
    0x85, HID_PD_RECHARGEABLE, //     REPORT_ID (HID_PD_RECHARGEABLE)
    0x09, 0x8B, //     USAGE (Rechargeable)
    0xB1, 0x23, //     FEATURE (Constant, Variable, Absolute, Nonvolatile)
    0x85, HID_PD_IDEVICECHEMISTRY, //     REPORT_ID (HID_PD_IDEVICECHEMISTRY)
    0x09, 0x89, //     USAGE (iDeviceChemistry)
    0x79, IDEVICECHEMISTRY, //     STRING INDEX (IDEVICECHEMISTRY)
    0xB1, 0x23, //     FEATURE (Constant, Variable, Absolute, Nonvolatile)
    0x85, HID_PD_IOEMINFORMATION, //     REPORT_ID (HID_PD_IOEMINFORMATION)
    0x09, 0x8F, //     USAGE (iOEMInformation)
    0x79, IOEMVENDOR, //     STRING INDEX (IOEMVENDOR)
    0xB1, 0x23, //     FEATURE (Constant, Variable, Absolute, Nonvolatile)
    0x85, HID_PD_CAPACITYMODE, //     REPORT_ID (HID_PD_CAPACITYMODE)
    0x09, 0x2C, //     USAGE (CapacityMode)
    0xB1, 0x23, //     FEATURE (Constant, Variable, Absolute, Nonvolatile)
    0x85, HID_PD_CPCTYGRANULARITY1, //     REPORT_ID (HID_PD_CPCTYGRANULARITY1)
    0x09, 0x8D, //     USAGE (CapacityGranularity1)
    0x26, 0x64, 0x00, //     LOGICAL_MAXIMUM (100)
    0xB1, 0x22, //     FEATURE (Data, Variable, Absolute, Nonvolatile)
    0x85, HID_PD_CPCTYGRANULARITY2, //     REPORT_ID (HID_PD_CPCTYGRANULARITY2)
    0x09, 0x8E, //     USAGE (CapacityGranularity2)
    0xB1, 0x23, //     FEATURE (Constant, Variable, Absolute, Nonvolatile)
    0x85, HID_PD_FULLCHRGECAPACITY, //     REPORT_ID (HID_PD_FULLCHRGECAPACITY)
    0x09, 0x67, //     USAGE (FullChargeCapacity)
    0xB1, 0x83, //     FEATURE (Constant, Variable, Absolute, Volatile)
    0x85, HID_PD_DESIGNCAPACITY, //     REPORT_ID (HID_PD_DESIGNCAPACITY)
    0x09, 0x83, //     USAGE (DesignCapacity)
    0xB1, 0x83, //     FEATURE (Constant, Variable, Absolute, Volatile)
    0x85, HID_PD_REMAININGCAPACITY, //     REPORT_ID (HID_PD_REMAININGCAPACITY)
    0x09, 0x66, //     USAGE (RemainingCapacity)
    0x81, 0xA3, //     INPUT (Constant, Variable, Absolute, Bitfield)
    0x09, 0x66, //     USAGE (RemainingCapacity)
    0xB1, 0xA3, //     FEATURE (Constant, Variable, Absolute, Volatile)
    0x85, HID_PD_WARNCAPACITYLIMIT, //     REPORT_ID (HID_PD_WARNCAPACITYLIMIT)
    0x09, 0x8C, //     USAGE (WarningCapacityLimit)
    0xB1, 0xA2, //     FEATURE (Data, Variable, Absolute, Volatile)
    0x85, HID_PD_REMNCAPACITYLIMIT, //     REPORT_ID (HID_PD_REMNCAPACITYLIMIT)
    0x09, 0x29, //     USAGE (RemainingCapacityLimit)
    0xB1, 0xA2, //     FEATURE (Data, Variable, Absolute, Volatile)
    0x85, HID_PD_MANUFACTUREDATE, //     REPORT_ID (HID_PD_MANUFACTUREDATE)
    0x09, 0x85, //     USAGE (ManufacturerDate)
    0x75, 0x10, //     REPORT_SIZE (16)
    0x27, 0xFF, 0xFF, 0x00, 0x00, //     LOGICAL_MAXIMUM (65534)
    0xB1, 0xA3, //     FEATURE (Constant, Variable, Absolute, Volatile)
    0x85, HID_PD_AVERAGETIME2FULL, //     REPORT_ID (HID_PD_AVERAGETIME2FULL)
    0x09, 0x6A, //     USAGE (AverageTimeToFull)
    0x27, 0xFF, 0xFF, 0x00, 0x00, //     LOGICAL_MAXIMUM (65535)
    0xB1, 0xA2, //     FEATURE (Data, Variable, Absolute, Volatile)
    0x85, HID_PD_AVERAGETIME2EMPTY, //     REPORT_ID (HID_PD_AVERAGETIME2EMPTY)
    0x09, 0x69, //     USAGE (AverageTimeToEmpty)
    0xB1, 0xA2, //     FEATURE (Data, Variable, Absolute, Volatile)
    0x85, HID_PD_RUNTIMETOEMPTY, //     REPORT_ID (HID_PD_RUNTIMETOEMPTY)
    0x09, 0x68, //     USAGE (RunTimeToEmpty)
    0xB1, 0xA3, //     FEATURE (Constant, Variable, Absolute, Volatile)
    0x85, HID_PD_REMAINTIMELIMIT, //     REPORT_ID (HID_PD_REMAINTIMELIMIT)
    0x09, 0x2A, //     USAGE (RemainingTimeLimit)
    0xB1, 0xA3, //     FEATURE (Constant, Variable, Absolute, Volatile)
    0x85, HID_PD_DELAYBE4SHUTDOWN, //     REPORT_ID (HID_PD_DELAYBE4SHUTDOWN)
    0x09, 0x2B, //     USAGE (DelayBeforeShutdown)
    0xB1, 0xA3, //     FEATURE (Constant, Variable, Absolute, Volatile)
    0x85, HID_PD_DELAYBE4REBOOT, //     REPORT_ID (HID_PD_DELAYBE4REBOOT)
    0x09, 0x87, //     USAGE (DelayBeforeReboot)
    0xB1, 0xA3, //     FEATURE (Constant, Variable, Absolute, Volatile)
    0x85, HID_PD_CONFIGVOLTAGE, //     REPORT_ID (HID_PD_CONFIGVOLTAGE)
    0x09, 0x6D, //     USAGE (ConfigVoltage)
    0x75, 0x10, //     REPORT_SIZE (16)
    0x27, 0xFF, 0xFF, 0x00, 0x00, //     LOGICAL_MAXIMUM (65535)
    0xB1, 0xA3, //     FEATURE (Constant, Variable, Absolute, Volatile)
    0x85, HID_PD_VOLTAGE, //     REPORT_ID (HID_PD_VOLTAGE)
    0x09, 0x30, //     USAGE (Voltage)
    0x81, 0xA2, //     INPUT (Data, Variable, Absolute, Volatile)
    0x85, HID_PD_AUDIBLEALARMCTRL, //     REPORT_ID (HID_PD_AUDIBLEALARMCTRL)
    0x09, 0x3A, //     USAGE (AudibleAlarmControl)
    0x75, 0x02, //     REPORT_SIZE (2)
    0x15, 0x01, //     LOGICAL_MINIMUM (1)
    0x25, 0x03, //     LOGICAL_MAXIMUM (3)
    0xB1, 0xA2, //     FEATURE (Data, Variable, Absolute, Volatile)
    0x75, 0x06, //     REPORT_SIZE (6)
    0x81, 0x01, //     INPUT (Constant)
    0x85, HID_PD_PRESENTSTATUS, //     REPORT_ID (HID_PD_PRESENTSTATUS)
    0x09, 0x3D, //     USAGE (PresentStatus)
    0x75, 0x10, //     REPORT_SIZE (16)
    0xB1, 0xA2, //     FEATURE (Data, Variable, Absolute, Volatile)
    0xC0, //   END_COLLECTION
    0xC0, // END_COLLECTION
];


// Define a custom report structure that implements AsInputReport
#[derive(Debug)]
struct PowerStatusReport {
    status: u8,
}

impl Serialize for PowerStatusReport {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer
    {
        serializer.serialize_u8(self.status)
    }
}

impl AsInputReport for PowerStatusReport {}

static mut EP_MEMORY: [u32; 1024] = [0; 1024];

// Define a global USB bus allocator
static mut USB_BUS_ALLOCATOR: Option<UsbBusAllocator<UsbBus<USB>>> = None;


#[entry]
fn main() -> ! {
    let mut dp = pac::Peripherals::take().unwrap();

    let rcc = dp.RCC.constrain();

    let clocks = rcc
        .cfgr
        .use_hse(8.MHz())
        .sysclk(48.MHz())
        .hclk(48.MHz())
        .require_pll48clk()
        .pclk1(24.MHz())
        .pclk2(24.MHz())
        .freeze();

    let mut delay = dp.TIM1.delay_us(&clocks);
    delay.delay(100.millis());  // apparently required for USB to set up properly...

    // initialize ports
    let gpioa = dp.GPIOA.split();
    let gpiob = dp.GPIOB.split();
    let _gpioc = dp.GPIOC.split();
    let gpioe = dp.GPIOE.split();
    let _gpiod = dp.GPIOD.split();

    // initialize pins

    // initialize leds
    let mut stat_led = LED::new(gpioe.pe2.into_push_pull_output());
    let mut fault_1_led = LED::new(gpioe.pe3.into_push_pull_output());
    let mut fault_2_led = LED::new(gpioe.pe4.into_push_pull_output());

    // initialize switch
    let mut sw = gpiob.pb8.into_floating_input();
    let mut syscfg = dp.SYSCFG.constrain();
    sw.make_interrupt_source(&mut syscfg);
    sw.trigger_on_edge(&mut dp.EXTI, Edge::Rising);
    sw.enable_interrupt(&mut dp.EXTI);

    // initialize usb
    let usb = USB {
        usb_global: dp.OTG_FS_GLOBAL,
        usb_device: dp.OTG_FS_DEVICE,
        usb_pwrclk: dp.OTG_FS_PWRCLK,
        pin_dm: gpioa.pa11.into_alternate(),
        pin_dp: gpioa.pa12.into_alternate(),
        hclk: clocks.hclk(),
    };
    delay.delay(100.millis());

    unsafe {
        usb_init(usb);
        cortex_m::peripheral::NVIC::unmask(Interrupt::OTG_FS);
        // Enable the external interrupt in the NVIC by passing the button interrupt number
        cortex_m::peripheral::NVIC::unmask(sw.interrupt());
    }

    // Now that button is configured, move button into global context
    cortex_m::interrupt::free(|cs| {
        G_BUTTON.borrow(cs).replace(Some(sw));
    });

    stat_led.on();

    for i in 0..=3 {
        delay.delay(1000.millis());
        match i {
            0 => { stat_led.on() }
            1 => { fault_1_led.on() }
            2 => { fault_2_led.on() }
            _ => {
                stat_led.off();
                fault_1_led.off();
                fault_2_led.off();
            }
        }
    }


    for _ in 0..=4 {
        delay.delay(200.millis());
        stat_led.toggle();
    }

    Task::new()
        .name("USB TASK")
        .stack_size(1024)
        .priority(TaskPriority(3))
        .start(move || {
            loop {
                cortex_m::interrupt::free(|cs| {
                    if let Some(hid) = G_USB_HID.borrow(cs).borrow_mut().as_mut() {
                        // Example: Send a report
                        let power_status_report = PowerStatusReport { status: 0x01 }; // Replace with actual status data
                        hid.push_input(&power_status_report).ok();
                    };
                    CurrentTask::delay(Duration::ms(5));
                });
            }
        }).unwrap();

    Task::new()
        .name("BLINK TASK")
        .stack_size(256)
        .priority(TaskPriority(2))
        .start(move || {
            loop {
                CurrentTask::delay(Duration::ms(500));
                fault_1_led.toggle();
            }
        }).unwrap();

    FreeRtosUtils::start_scheduler();
}


#[exception]
#[allow(non_snake_case)]
unsafe fn DefaultHandler(_irqn: i16) {
    // custom default handler
    // irqn is negative for Cortex-M exceptions
    // irqn is positive for device specific (line IRQ)
    // panic!("Exception: {}", irqn);
}

#[exception]
#[allow(non_snake_case)]
unsafe fn HardFault(_ef: &ExceptionFrame) -> ! {
    loop {}
}

// define what happens in an Out Of Memory (OOM) condition
#[alloc_error_handler]
fn alloc_error(_layout: Layout) -> ! {
    asm::bkpt();
    loop {}
}

#[no_mangle]
#[allow(non_snake_case, unused_variables)]
fn vApplicationStackOverflowHook(pxTask: FreeRtosTaskHandle, pcTaskName: FreeRtosCharPtr) {
    asm::bkpt();
}