//#![deny(unsafe_code)]
#![no_main]
#![no_std]
// For allocator
#![feature(lang_items)]
#![feature(alloc_error_handler)]

extern crate alloc;

use alloc::sync::Arc;
use micromath::F32Ext;

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
use core::f32::consts::PI;
use crate::report::{HID_PD_PRESENTSTATUS, HID_PD_REMAININGCAPACITY, HID_PD_RUNTIMETOEMPTY, Report, Status};
use modular_bitfield_to_value::ToValue;
use stm32f4xx_hal::adc::config::{AdcConfig, Dma, SampleTime, Scan, Sequence};
use stm32f4xx_hal::adc::{Adc, Temperature};
use stm32f4xx_hal::dma::config::DmaConfig;
use stm32f4xx_hal::dma::{StreamsTuple, Transfer};
use stm32f4xx_hal::timer::Channel4;
use crate::adc::{read_current, read_v_bat, read_v_in, ADC_MEMORY, G_XFR};
use arrform::{arrform, ArrForm};

mod devices;
mod intrpt;
mod usb_hid;
mod report;
mod adc;
mod utils;
mod usb_serial;

#[global_allocator]
static GLOBAL: FreeRtosAllocator = FreeRtosAllocator;


use crate::usb_hid::{G_USB_HID, usb_hid_init, G_USB_DEVICE, G_USB_HID_MODE};
use crate::usb_serial::{usb_println, usb_serial_init};
use crate::utils::LEDState;

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
    let led_dim = Channel4::new(gpiob.pb1);

    let mut usb_led1 = LED::new(gpioc.pc13.into_push_pull_output());

    // initialize pwm timer 3
    let mut stat_led_pwm = dp
        .TIM3
        .pwm_hz(led_dim, 10000.Hz(), &clocks)
        .split();


    // initialize pins

    let mut adc_vbat = gpioa.pa0.into_analog();
    let mut adc_current = gpioa.pa1.into_analog();
    let mut adc_vin = gpioa.pa2.into_analog();

    // initialize switch
    let sw = gpioc.pc15.into_floating_input();
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

    let hid_mode = sw.is_high();
    cortex_m::interrupt::free(|cs| {
        *G_USB_HID_MODE.borrow(cs).borrow_mut() = hid_mode;
    });

    unsafe {
        if hid_mode {
            usb_hid_init(usb);
        } else {
            usb_serial_init(usb);
        }
        cortex_m::peripheral::NVIC::unmask(Interrupt::OTG_FS);
        cortex_m::peripheral::NVIC::unmask(Interrupt::DMA2_STREAM0);
        // Enable the external interrupt in the NVIC by passing the button interrupt number
        // cortex_m::peripheral::NVIC::unmask(sw.interrupt());
    }

    // Now that button is configured, move button into global context
    // cortex_m::interrupt::free(|cs| {
    //     G_BUTTON.borrow(cs).replace(Some(sw));
    // });

    let dma_config = DmaConfig::default()
        .transfer_complete_interrupt(true)
        .memory_increment(true)
        .double_buffer(false);

    let adc_config = AdcConfig::default()
        .dma(Dma::Continuous)
        .scan(Scan::Enabled);
    let mut adc = Adc::adc1(dp.ADC1, true, adc_config);

    adc.configure_channel(&Temperature, Sequence::One, SampleTime::Cycles_480);
    adc.configure_channel(&adc_vbat, Sequence::Two, SampleTime::Cycles_480);
    adc.configure_channel(&adc_vin, Sequence::Three, SampleTime::Cycles_480);
    adc.configure_channel(&adc_current, Sequence::Four, SampleTime::Cycles_480);
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


    for i in 0..=3 {
        delay.delay(1000.millis());
        match i {
            0 => {
                stat_led_pwm.enable();
                stat_led_pwm.set_duty(8000)
            }
            1 => { usb_led1.on() }
            _ => {
                stat_led_pwm.disable();
                usb_led1.off();
            }
        }
    }

    let led_state = LEDState::SlowBreathing;
    let led_state_container =
        Arc::new(Mutex::new(led_state).expect("Failed to create led state guard mutex"));
    let led_state_container_main = led_state_container.clone();
    let led_state_container_led = led_state_container;


    for _ in 0..=4 {
        delay.delay(200.millis());
    }


    Task::new()
        .name("USB TASK")
        .stack_size(1024)
        .priority(TaskPriority(3))
        .start(move || {
            let mut current = 0.0;
            let mut vbat = 0.0;
            let mut vin = 0.0;
            let mut supply_present = false;
            let mut capacity = 0;
            let mut remaining_seconds = 0;

            let battery_capacity = 2.0 * 3.7 * 2100.0; // Wh

            let mut remaining_capacity_report = Report::new_u8(HID_PD_REMAININGCAPACITY, 50);
            let mut runtime_empty_report = Report::new_u16(HID_PD_RUNTIMETOEMPTY, 1);
            let mut status = Status::new();
            status.set_charging(1);
            status.set_ac_present(1);
            status.set_battery_present(0);

            let mut led_state = LEDState::SlowBreathing;

            let mut status_report = Report::new_u16(HID_PD_PRESENTSTATUS, status.to_u16_le().unwrap());
            loop {
                current = read_current();
                vbat = read_v_bat();
                vin = read_v_in();
                supply_present = vin > 10.0;

                if vbat < 4.1 * 2.0 && supply_present {
                    status.set_charging(1);
                    status.set_discharging(0);
                    status_report.update_u16_value(status.to_u16_le().unwrap());
                } else if !supply_present {
                    status.set_charging(0);
                    status.set_discharging(1);
                    status_report.update_u16_value(status.to_u16_le().unwrap());
                }

                if supply_present {
                    status.set_charging(1);
                    status.set_ac_present(1);
                    status.set_battery_present(0);
                    status_report.update_u16_value(status.to_u16_le().unwrap());
                    led_state = LEDState::SlowBreathing;
                } else {
                    status.set_ac_present(0);
                    status.set_charging(0);
                    status.set_battery_present(1);
                    status_report.update_u16_value(status.to_u16_le().unwrap());
                    led_state = LEDState::FastBreathing;
                }

                if vbat < 3.5 * 2.0 {
                    status.set_remaining_time_limit_expired(1);
                    status.set_shutdown_requested(1);
                    status_report.update_u16_value(status.to_u16_le().unwrap());
                }

                if vbat < 3.2 * 2.0 {
                    status.set_shutdown_imminent(1);
                    status_report.update_u16_value(status.to_u16_le().unwrap());
                }

                if hid_mode {
                    cortex_m::interrupt::free(|cs| {
                        if let Some(hid) = G_USB_HID.borrow(cs).borrow_mut().as_mut() {
                            hid.send_report(&status_report);
                        };
                        usb_led1.toggle();
                    });
                }
                CurrentTask::delay(Duration::ms(300));


                capacity = (100.0 / (4.15 * 2.0 - 3.3 * 2.0) * (vbat - 3.3 * 2.0)) as u8;
                remaining_seconds = (battery_capacity / vbat / current) as u16;

                remaining_capacity_report = Report::new_u8(HID_PD_REMAININGCAPACITY, capacity);
                if hid_mode {
                    cortex_m::interrupt::free(|cs| {
                        if let Some(hid) = G_USB_HID.borrow(cs).borrow_mut().as_mut() {
                            hid.send_report(&remaining_capacity_report);
                        };
                    });
                }
                CurrentTask::delay(Duration::ms(300));


                runtime_empty_report = Report::new_u16(HID_PD_RUNTIMETOEMPTY, remaining_seconds);
                if hid_mode {
                    cortex_m::interrupt::free(|cs| {
                        if let Some(hid) = G_USB_HID.borrow(cs).borrow_mut().as_mut() {
                            hid.send_report(&runtime_empty_report);
                        };
                    });
                }


                if hid_mode {} else {
                    usb_println(arrform!(128, "v_bat: {}, v_in: {}, current: {}, remaining seconds: {}",vbat, vin, current, remaining_seconds ).as_str());
                    usb_led1.toggle();
                }

                if let Ok(mut guard) = led_state_container_main.lock(Duration::ms(1)) {
                    *guard = led_state.clone();
                }

                CurrentTask::delay(Duration::ms(300));
            }
        }).unwrap();

    Task::new()
        .name("BLINK TASK")
        .stack_size(256)
        .priority(TaskPriority(2))
        .start(move || {
            let max_duty = stat_led_pwm.get_max_duty();
            let mut count = 0;
            let mut led_state = LEDState::SlowBreathing;
            loop {
                if let Ok(guard) = led_state_container_led.lock(Duration::ms(1)) {
                    led_state = guard.clone();
                }
                stat_led_pwm.enable();
                let val = max_duty
                    - (max_duty as f32 * (count as f32 / 1024.0 * PI).sin()) as u16; // LED1
                stat_led_pwm.set_duty(val);
                match led_state {
                    LEDState::FastBreathing => {
                        count += 20;
                        CurrentTask::delay(Duration::ms(5));
                    }
                    LEDState::SlowBreathing => {
                        count += 10;
                        CurrentTask::delay(Duration::ms(10));
                    }
                }
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