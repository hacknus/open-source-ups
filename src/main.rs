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

use usbd_hid::descriptor::{SerializedDescriptor, generator_prelude::*, MouseReport};
use usbd_hid::hid_class::HIDClass;
use crate::usb::{G_USB_DEVICE, G_USB_HID, usb_init};
#[gen_hid_descriptor(
    (collection = APPLICATION, usage_page = POWER_DEVICE, usage = UPS) = {
        (collection = LOGICAL, usage = UPS) = {
            (usage = 0x0C,) = {
                #[item_settings data,variable,absolute] remaining_capacity=input;
            };
            (usage = 0x0C,) = {
                #[item_settings data,variable,absolute] voltage=input;
            };
            (usage = 0x0C,) = {
                #[item_settings data,variable,absolute] current=input;
            };
            (usage = 0x0C,) = {
                #[item_settings data,variable,absolute] runtime_to_empty=input;
            };
        };
    }
)]
#[allow(dead_code)]
pub struct PowerStatusReport {
    pub remaining_capacity: u16,
    pub voltage: u16,
    pub current: i16,
    pub runtime_to_empty: u16,
}

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
        pin_dm: stm32f4xx_hal::gpio::alt::otg_fs::Dm::PA11(gpioa.pa11.into_alternate()),
        pin_dp: stm32f4xx_hal::gpio::alt::otg_fs::Dp::PA12(gpioa.pa12.into_alternate()),
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
                        let report = PowerStatusReport {
                            remaining_capacity: 50,
                            voltage: 2,
                            current: 3,
                            runtime_to_empty: 4,
                        };
                        stat_led.toggle();
                        hid.push_input(&report).ok();
                    };
                });
                CurrentTask::delay(Duration::ms(5));
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