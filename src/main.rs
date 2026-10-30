//#![deny(unsafe_code)]
#![no_main]
#![no_std]
// For allocator
#![feature(lang_items)]
#![feature(alloc_error_handler)]

extern crate alloc;

use alloc::sync::Arc;

use crate::devices::led::LED;
use cortex_m::asm;
use cortex_m_rt::exception;
use cortex_m_rt::{entry, ExceptionFrame};
use panic_halt as _;
use stm32f4xx_hal::otg_fs::USB;
use stm32f4xx_hal::{
    pac::{self, Interrupt},
    prelude::*,
};

use crate::battery::BatteryState;
use arrform::{arrform, ArrForm};
use core::alloc::Layout;
use freertos_rust::*;
use stm32f4xx_hal::adc::config::{AdcConfig, Dma, SampleTime, Scan, Sequence};
use stm32f4xx_hal::adc::{Adc, Temperature};
use stm32f4xx_hal::dma::config::DmaConfig;
use stm32f4xx_hal::dma::{StreamsTuple, Transfer};
use stm32f4xx_hal::timer::Channel4;

mod adc;
mod battery;
mod devices;
mod intrpt;
mod power;
mod report;
mod usb;
mod usb_hid;
mod usb_link;
mod usb_serial;
mod utils;

#[global_allocator]
static GLOBAL: FreeRtosAllocator = FreeRtosAllocator;

use crate::power::PowerSource;
use crate::usb_serial::usb_println;
use crate::utils::{LEDState, LedAnimation};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

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
    delay.delay(100.millis()); // apparently required for USB to set up properly...

    // initialize ports
    let gpioa = dp.GPIOA.split();
    let gpiob = dp.GPIOB.split();
    let gpioc = dp.GPIOC.split();

    // initialize dma
    let dma2 = StreamsTuple::new(dp.DMA2);

    // U7 PB12/PB13 (pins 33/34): active-low LTC4418 VALID1/VALID2.
    // R46/R45 provide external pull-ups to 3.3 V.
    let valid1 = gpiob.pb12.into_floating_input();
    let valid2 = gpiob.pb13.into_floating_input();

    // U7 PB1 (pin 27): STAT, TIM3 channel 4.
    let led_dim = Channel4::new(gpiob.pb1);

    // U7 PC13 (pin 2): LED1.
    let mut usb_led1 = LED::new(gpioc.pc13.into_push_pull_output());

    // initialize pwm timer 3
    let mut stat_led_pwm = dp.TIM3.pwm_hz(led_dim, 10000.Hz(), &clocks).split();

    // U7 PA0/PA1/PA2 (pins 14/15/16): ADC_VBAT/ADC_CURRENT/ADC_VIN.

    let adc_vbat = gpioa.pa0.into_analog();
    let adc_current = gpioa.pa1.into_analog();
    let adc_vin = gpioa.pa2.into_analog();

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

    // Singleton allocations are unique; the transfer and spare never alias.
    let buffer = cortex_m::singleton!(: [u16; 4] = [0; 4]).unwrap();
    let spare = cortex_m::singleton!(: [u16; 4] = [0; 4]).unwrap();
    adc::set_scale(adc.reference_voltage() as f32 / 1000.0 / 4096.0);
    let transfer = Transfer::init_peripheral_to_memory(dma2.0, adc, buffer, None, dma_config);
    cortex_m::interrupt::free(|cs| {
        adc::TRANSFER.borrow(cs).replace(Some(transfer));
        adc::SPARE.borrow(cs).replace(Some(spare));
    });
    cortex_m::peripheral::NVIC::unpend(Interrupt::DMA2_STREAM0);
    let mut cp = cortex_m::Peripherals::take().unwrap();
    unsafe {
        cp.NVIC.set_priority(Interrupt::DMA2_STREAM0, 0x60);
        cp.NVIC.set_priority(Interrupt::OTG_FS, 0x70);
        cortex_m::peripheral::NVIC::unmask(Interrupt::DMA2_STREAM0);
    }
    adc::start_scan();

    // U7 PC15 (pin 4): SW; the board provides the external pull-up.
    let sw = gpioc.pc15.into_floating_input();
    // U7 PA11/PA12 (pins 44/45): USB_DM/USB_DP.
    let usb = USB {
        usb_global: dp.OTG_FS_GLOBAL,
        usb_device: dp.OTG_FS_DEVICE,
        usb_pwrclk: dp.OTG_FS_PWRCLK,
        pin_dm: stm32f4xx_hal::gpio::alt::otg_fs::Dm::PA11(gpioa.pa11.into_alternate()),
        pin_dp: stm32f4xx_hal::gpio::alt::otg_fs::Dp::PA12(gpioa.pa12.into_alternate()),
        hclk: clocks.hclk(),
    };
    delay.delay(100.millis());

    // Collect a first averaged scan before exposing measurement feature reports.
    for _ in 0..16 {
        delay.delay(20.millis());
        adc::start_scan();
    }
    let mut battery = BatteryState::new();
    let mut last_sequence = 0;
    let initial_sample = adc::measurements(&mut last_sequence);
    let hid_mode = sw.is_high();
    usb::init(usb, hid_mode);
    let initial_source = PowerSource::from_pin_levels(valid1.is_high(), valid2.is_high());
    usb::update(battery.update(initial_source, initial_sample), true);
    unsafe {
        cortex_m::peripheral::NVIC::unmask(Interrupt::OTG_FS);
    }

    let led_state = LEDState::for_power(
        initial_source,
        initial_sample.map_or(true, |m| m.has_battery_fault()),
    );
    let led_state_container =
        Arc::new(Mutex::new(led_state).expect("Failed to create led state guard mutex"));
    let led_state_container_main = led_state_container.clone();
    let led_state_container_led = led_state_container;

    Task::new()
        .name("USB TASK")
        .stack_size(1024)
        .priority(TaskPriority(3))
        .start(move || {
            let mut tick = 0u16;
            let mut latest = initial_sample;
            let mut age = if latest.is_some() { 0u16 } else { 100u16 };
            loop {
                if let Some(sample) = adc::measurements(&mut last_sequence) {
                    latest = Some(sample);
                    age = 0;
                } else {
                    age = age.saturating_add(1);
                }
                adc::start_scan();
                if tick % 25 == 0 {
                    let source = PowerSource::from_pin_levels(valid1.is_high(), valid2.is_high());
                    let sample = if age < 50 { latest } else { None };
                    let telemetry = battery.update(source, sample);
                    usb::update(telemetry, tick == 0);
                    if hid_mode {
                        usb_led1.toggle();
                    } else if let Some(m) = latest.filter(|_| age < 50) {
                        usb_println(
                            arrform!(
                                128,
                                "source: {:?}, v_bat: {:?}, v_in: {:?}, boost current: {:?}",
                                source,
                                m.pack_v,
                                m.input_v,
                                m.boost_current_a
                            )
                            .as_str(),
                        );
                    } else {
                        usb_println(
                            arrform!(128, "source: {:?}, ADC measurements unavailable", source)
                                .as_str(),
                        );
                    }
                    if let Ok(mut guard) = led_state_container_main.lock(Duration::ms(1)) {
                        *guard = LEDState::for_power(
                            source,
                            sample.map_or(true, |m| m.has_battery_fault()),
                        );
                    }
                }
                usb::tick(20);
                usb::service();
                tick = (tick + 1) % 250; // refresh every 5 seconds
                CurrentTask::delay(Duration::ms(20));
            }
        })
        .unwrap();

    Task::new()
        .name("BLINK TASK")
        .stack_size(256)
        .priority(TaskPriority(2))
        .start(move || {
            let max_duty = stat_led_pwm.get_max_duty();
            let mut animation = LedAnimation::new();
            let mut led_state = led_state;
            loop {
                if let Ok(guard) = led_state_container_led.lock(Duration::ms(1)) {
                    led_state = guard.clone();
                }
                stat_led_pwm.enable();
                stat_led_pwm.set_duty(animation.step(led_state, max_duty, 5));
                CurrentTask::delay(Duration::ms(5));
            }
        })
        .unwrap();

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
