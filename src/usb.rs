use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use stm32f4xx_hal::otg_fs::{UsbBus, USB};
use stm32f4xx_hal::pac::{interrupt};
use usb_device::prelude::*;
use usb_device::bus::UsbBusAllocator;
use usbd_hid::hid_class::HIDClass;
use usbd_hid::descriptor::{SerializedDescriptor, generator_prelude::*, MouseReport};

#[gen_hid_descriptor(
    (collection = APPLICATION, usage_page = 0x84, usage = 0x04) = { // UPS
    (usage_page = 0x84, usage = 0x85) = { // Battery System
    # [item_settings const, variable, absolute, nonvolatile] iProduct = feature;
    # [item_settings const, variable, absolute, nonvolatile] iSerialNumber = feature;
    # [item_settings const, variable, absolute, nonvolatile] iManufacturer = feature;
    (usage_page = 0x85, usage = 0x8B) = {
    # [item_settings const, variable, absolute, nonvolatile] Rechargeable = feature;
    };
    (usage_page = 0x85, usage = 0x89) = {
    # [item_settings const, variable, absolute, nonvolatile] iDeviceChemistry = feature;
    };
    (usage_page = 0x85, usage = 0x8F) = {
    # [item_settings const, variable, absolute, nonvolatile] iOEMInformation = feature;
    };
    (usage_page = 0x85, usage = 0x2C) = {
    # [item_settings const, variable, absolute, nonvolatile] CapacityMode = feature;
    };
    (usage_page = 0x85, usage = 0x8D) = {
    # [item_settings data, variable, absolute, nonvolatile] CapacityGranularity1 = feature;
    };
    (usage_page = 0x85, usage = 0x8E) = {
    # [item_settings const, variable, absolute, nonvolatile] CapacityGranularity2 = feature;
    };
    (usage_page = 0x85, usage = 0x67) = {
    # [item_settings const, variable, absolute, volatile] FullChargeCapacity = feature;
    };
    (usage_page = 0x85, usage = 0x83) = {
    # [item_settings const, variable, absolute, volatile] DesignCapacity = feature;
    };
    (usage_page = 0x85, usage = 0x66) = {
    # [item_settings const, variable, absolute, volatile] RemainingCapacity = feature;
    # [item_settings const, variable, absolute, bitfield] RemainingCapacity = input;
    };
    (usage_page = 0x85, usage = 0x8C) = {
    # [item_settings data, variable, absolute, volatile] WarningCapacityLimit = feature;
    };
    (usage_page = 0x85, usage = 0x29) = {
    # [item_settings data, variable, absolute, volatile] RemainingCapacityLimit = feature;
    };
    (usage_page = 0x85, usage = 0x85) = {
    # [item_settings const, variable, absolute, volatile] ManufactureDate = feature;
    };
    (usage_page = 0x85, usage = 0x6A) = {
    # [item_settings data, variable, absolute, volatile] AverageTimeToFull = feature;
    };
    (usage_page = 0x85, usage = 0x69) = {
    # [item_settings data, variable, absolute, volatile] AverageTimeToEmpty = feature;
    };
    (usage_page = 0x85, usage = 0x68) = {
    # [item_settings const, variable, absolute, volatile] RunTimeToEmpty = feature;
    };
    (usage_page = 0x85, usage = 0x2A) = {
    # [item_settings const, variable, absolute, volatile] RemainingTimeLimit = feature;
    };
    (usage_page = 0x85, usage = 0x2B) = {
    # [item_settings const, variable, absolute, volatile] DelayBeforeShutdown = feature;
    };
    (usage_page = 0x85, usage = 0x87) = {
    # [item_settings const, variable, absolute, volatile] DelayBeforeReboot = feature;
    };
    (usage_page = 0x85, usage = 0x6D) = {
    # [item_settings const, variable, absolute, volatile] ConfigVoltage = feature;
    };
    (usage_page = 0x85, usage = 0x30) = {
    # [item_settings data, variable, absolute, volatile] Voltage = input;
    };
    (usage_page = 0x85, usage = 0x3A) = {
    # [packed_bits 2] # [item_settings data, variable, absolute, volatile] AudibleAlarmControl = feature;
    };
    (usage_page = 0x85, usage = 0x3D) = {
    # [item_settings data, variable, absolute, volatile] PresentStatus = feature;
    };
    };
    }
)]
#[derive(Default)]
pub struct Report {
    pub iProduct: u8,
    pub iSerialNumber: u8,
    pub iManufacturer: u8,
    pub Rechargeable: u8,
    pub iDeviceChemistry: u8,
    pub iOEMInformation: u8,
    pub CapacityMode: u8,
    pub CapacityGranularity1: u8,
    pub CapacityGranularity2: u8,
    pub FullChargeCapacity: u16,
    pub DesignCapacity: u16,
    pub RemainingCapacity: u16,
    pub WarningCapacityLimit: u16,
    pub RemainingCapacityLimit: u16,
    pub ManufactureDate: u16,
    pub AverageTimeToFull: u16,
    pub AverageTimeToEmpty: u16,
    pub RunTimeToEmpty: u16,
    pub RemainingTimeLimit: u16,
    pub DelayBeforeShutdown: u16,
    pub DelayBeforeReboot: u16,
    pub ConfigVoltage: u16,
    pub Voltage: u16,
    pub AudibleAlarmControl: u8,
    pub PresentStatus: u16,
}


// Make USB serial device globally available
pub static G_USB_HID: Mutex<RefCell<Option<HIDClass<UsbBus<USB>>>>> =
    Mutex::new(RefCell::new(None));

// Make USB device globally available
pub static G_USB_DEVICE: Mutex<RefCell<Option<UsbDevice<UsbBus<USB>>>>> =
    Mutex::new(RefCell::new(None));

#[allow(dead_code)]
pub unsafe fn usb_init(usb: USB) {
    static mut EP_MEMORY: [u32; 1024] = [0; 1024];
    static mut USB_BUS: Option<UsbBusAllocator<stm32f4xx_hal::otg_fs::UsbBusType>> = None;
    USB_BUS = Some(stm32f4xx_hal::otg_fs::UsbBusType::new(usb, &mut EP_MEMORY));
    let usb_bus = USB_BUS.as_ref().unwrap();
    let hid = HIDClass::new(&usb_bus, Report::desc(), 1);
    let usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x03f0, 0x1f06))
        .device_class(0x03)
        .strings(&[StringDescriptors::default()
            .manufacturer("Linus Leo Stöckli")
            .product("UPS")
            .serial_number("UPS10")])
        .unwrap()
        .build();
    cortex_m::interrupt::free(|cs| {
        *G_USB_HID.borrow(cs).borrow_mut() = Some(hid);
        *G_USB_DEVICE.borrow(cs).borrow_mut() = Some(usb_dev);
    });
}


#[interrupt]
#[allow(non_snake_case)]
fn OTG_FS() {
    cortex_m::interrupt::free(|cs| {
        match G_USB_DEVICE.borrow(cs).borrow_mut().as_mut() {
            None => {}
            Some(usb_dev) => {
                match G_USB_HID.borrow(cs).borrow_mut().as_mut() {
                    None => {}
                    Some(hid) => {
                        // do this regularly to keep connection to USB host
                        usb_dev.poll(&mut [hid]);
                    }
                }
            }
        }
    });
}