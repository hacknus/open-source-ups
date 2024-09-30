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
use stm32f4xx_hal::otg_fs::{USB};
use stm32f4xx_hal::{
    pac::{self, Interrupt},
    gpio::{Edge},
    prelude::*,
};
use crate::devices::led::LED;
use crate::intrpt::{G_BUTTON, G_STATE};

use freertos_rust::*;
use core::alloc::Layout;
use core::borrow::BorrowMut;
use crate::report::{HID_PD_PRESENTSTATUS, HID_PD_REMAININGCAPACITY, HID_PD_RUNTIMETOEMPTY, Report, Status};
use modular_bitfield_to_value::ToValue;
use stm32f4xx_hal::adc::config::{AdcConfig, Dma, SampleTime, Scan, Sequence};
use stm32f4xx_hal::adc::{Adc, Temperature};
use stm32f4xx_hal::dma::config::DmaConfig;
use stm32f4xx_hal::dma::{StreamsTuple, Transfer};
use crate::adc::{read_current, read_v_bat, ADC_MEMORY, G_XFR};

mod devices;
mod intrpt;
mod usb;
mod report;
mod adc;

#[global_allocator]
static GLOBAL: FreeRtosAllocator = FreeRtosAllocator;


use crate::usb::{G_USB_HID, usb_init};


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
    let gpioc = dp.GPIOC.split();
    let gpioe = dp.GPIOE.split();
    let _gpiod = dp.GPIOD.split();

    // initialize dma
    let dma2 = StreamsTuple::new(dp.DMA2);

    // initialize leds
    let mut stat_led = LED::new(gpiob.pb1.into_push_pull_output());
    let mut usb_led1 = LED::new(gpioc.pc13.into_push_pull_output());

    // initialize pins
    let mut vin_pin = gpioa.pa2.into_floating_input();

    let mut adc_vbat = gpioa.pa0.into_analog();
    let mut adc_current = gpioa.pa1.into_analog();

    let dma_config = DmaConfig::default()
        .transfer_complete_interrupt(true)
        .memory_increment(true)
        .double_buffer(false);

    let adc_config = AdcConfig::default()
        .dma(Dma::Continuous)
        .scan(Scan::Enabled);
    let mut adc = Adc::adc1(dp.ADC1, true, adc_config);

    adc.configure_channel(&Temperature, Sequence::One, SampleTime::Cycles_480);
    adc.configure_channel(&adc_vbat, Sequence::Two, SampleTime::Cycles_3);
    adc.configure_channel(&adc_current, Sequence::Two, SampleTime::Cycles_3);
    adc.enable_temperature_and_vref();

    // let adc_buffer = cortex_m::singleton!(: [u16; 2] = [0; 2]).unwrap();
    let mut transfer = unsafe {
        Transfer::init_peripheral_to_memory(dma2.0, adc, &mut ADC_MEMORY, None, dma_config)
    };

    transfer.start(|adc| {
        adc.start_conversion();
    });
    cortex_m::interrupt::free(|cs| {
        G_XFR.borrow(cs).replace(Some(transfer));
    });

    // initialize switch
    let mut sw = gpioc.pc15.into_floating_input();
    // let mut syscfg = dp.SYSCFG.constrain();
    // sw.make_interrupt_source(&mut syscfg);
    // sw.trigger_on_edge(&mut dp.EXTI, Edge::Rising);
    // sw.enable_interrupt(&mut dp.EXTI);

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
        // cortex_m::peripheral::NVIC::unmask(sw.interrupt());
    }

    // Now that button is configured, move button into global context
    // cortex_m::interrupt::free(|cs| {
    //     G_BUTTON.borrow(cs).replace(Some(sw));
    // });

    stat_led.on();

    for i in 0..=3 {
        delay.delay(1000.millis());
        match i {
            0 => { stat_led.on() }
            1 => { usb_led1.on() }
            _ => {
                stat_led.off();
                usb_led1.off();
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
            let mut current = 0.0;
            let mut vbat = 0.0;
            let mut supply_present = false;
            let mut capacity = 0;
            let mut remaining_minutes = 0;

            let mut old_capacity = 100;
            let mut old_remaining_minutes = 1;

            let battery_capacity = 2.0 * 3.7 * 2100.0; // Wh

            let mut remaining_capacity_report = Report::new_u8(HID_PD_REMAININGCAPACITY, 50);
            let mut runtime_empty_report = Report::new_u16(HID_PD_RUNTIMETOEMPTY, 1);
            let mut status = Status::new();
            let mut old_status = Status::new();
            status.set_charging(1);
            status.set_ac_present(1);
            status.set_battery_present(0);

            let mut status_report = Report::new_u16(HID_PD_PRESENTSTATUS, status.to_u16_le().unwrap());
            loop {
                current = read_current();
                vbat = read_v_bat();
                supply_present = vin_pin.is_high();

                if vbat < 4.0 && supply_present {
                    status.set_charging(1);
                    status_report.update_u16_value(status.to_u16_le().unwrap());
                } else if !supply_present {
                    status.set_charging(0);
                    status.set_discharging(1);
                    status_report.update_u16_value(status.to_u16_le().unwrap());
                }

                if supply_present {
                    status.set_ac_present(0);
                    status.set_battery_present(1);
                    status_report.update_u16_value(status.to_u16_le().unwrap());
                } else {
                    status.set_ac_present(1);
                    status.set_battery_present(0);
                    status_report.update_u16_value(status.to_u16_le().unwrap());
                }

                if vbat < 3.5 {
                    status.set_remaining_time_limit_expired(1);
                    status.set_shutdown_requested(1);
                    status_report.update_u16_value(status.to_u16_le().unwrap());
                }

                if vbat < 3.2 {
                    status.set_shutdown_imminent(1);
                    status_report.update_u16_value(status.to_u16_le().unwrap());
                }

                if status.to_u16_le().unwrap() != old_status.to_u16_le().unwrap() {
                    cortex_m::interrupt::free(|cs| {
                        if let Some(hid) = G_USB_HID.borrow(cs).borrow_mut().as_mut() {
                            hid.send_report(&status_report);
                        };
                    });
                    old_status = status;
                }

                capacity = (100.0 / (4.15 - 3.3) * (vbat - 3.3)) as u8;
                remaining_minutes = (battery_capacity / vbat / current) as u16;

                if capacity != old_capacity {
                    remaining_capacity_report = Report::new_u8(HID_PD_REMAININGCAPACITY, capacity);
                    old_capacity = capacity;
                    cortex_m::interrupt::free(|cs| {
                        if let Some(hid) = G_USB_HID.borrow(cs).borrow_mut().as_mut() {
                            hid.send_report(&remaining_capacity_report);
                        };
                    });
                }
                if remaining_minutes != old_remaining_minutes {
                    runtime_empty_report = Report::new_u16(HID_PD_RUNTIMETOEMPTY, remaining_minutes);
                    old_remaining_minutes = remaining_minutes;
                    cortex_m::interrupt::free(|cs| {
                        if let Some(hid) = G_USB_HID.borrow(cs).borrow_mut().as_mut() {
                            hid.send_report(&runtime_empty_report);
                        };
                    });
                }

                cortex_m::interrupt::free(|cs| {
                    if let Some(hid) = G_USB_HID.borrow(cs).borrow_mut().as_mut() {
                        hid.send_report(&status_report);
                        stat_led.toggle();
                    };
                });

                CurrentTask::delay(Duration::ms(500));
            }
        }).unwrap();

    Task::new()
        .name("BLINK TASK")
        .stack_size(256)
        .priority(TaskPriority(2))
        .start(move || {
            loop {
                CurrentTask::delay(Duration::ms(500));
                usb_led1.toggle();
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