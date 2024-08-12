use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use stm32f4xx_hal::otg_fs::{UsbBus, USB};
use stm32f4xx_hal::pac::{interrupt};
use usb_device::prelude::*;
use usb_device::bus::UsbBusAllocator;
use usbd_hid::hid_class::HIDClass;
use usbd_hid::descriptor::{SerializedDescriptor, generator_prelude::*, MouseReport};



pub const HID_PD_IPRODUCT: u8 = 0x01;               // FEATURE ONLY
pub const HID_PD_SERIAL: u8 = 0x02;                 // FEATURE ONLY
pub const HID_PD_MANUFACTURER: u8 = 0x03;           // FEATURE ONLY
pub const IDEVICECHEMISTRY: u8 = 0x04;
pub const IOEMVENDOR: u8 = 0x05;

pub const HID_PD_RECHARGEABLE: u8 = 0x06;           // FEATURE ONLY
pub const HID_PD_PRESENTSTATUS: u8 = 0x07;          // INPUT OR FEATURE(required by Windows)
pub const HID_PD_REMAINTIMELIMIT: u8 = 0x08;
pub const HID_PD_MANUFACTUREDATE: u8 = 0x09;
pub const HID_PD_CONFIGVOLTAGE: u8 = 0x0A;          // FEATURE ONLY
pub const HID_PD_VOLTAGE: u8 = 0x0B;                // INPUT (NA) OR FEATURE(implemented)
pub const HID_PD_REMAININGCAPACITY: u8 = 0x0C;      // INPUT OR FEATURE(required by Windows)
pub const HID_PD_RUNTIMETOEMPTY: u8 = 0x0D;
pub const HID_PD_FULLCHRGECAPACITY: u8 = 0x0E;      // INPUT OR FEATURE. Last Full Charge Capacity
pub const HID_PD_WARNCAPACITYLIMIT: u8 = 0x0F;
pub const HID_PD_CPCTYGRANULARITY1: u8 = 0x10;
pub const HID_PD_REMNCAPACITYLIMIT: u8 = 0x11;
pub const HID_PD_DELAYBE4SHUTDOWN: u8 = 0x12;       // FEATURE ONLY
pub const HID_PD_DELAYBE4REBOOT: u8 = 0x13;
pub const HID_PD_AUDIBLEALARMCTRL: u8 = 0x14;       // FEATURE ONLY
pub const HID_PD_CURRENT: u8 = 0x15;                // FEATURE ONLY
pub const HID_PD_CAPACITYMODE: u8 = 0x16;
pub const HID_PD_DESIGNCAPACITY: u8 = 0x17;
pub const HID_PD_CPCTYGRANULARITY2: u8 = 0x18;
pub const HID_PD_AVERAGETIME2FULL: u8 = 0x1A;
pub const HID_PD_AVERAGECURRENT: u8 = 0x1B;
pub const HID_PD_AVERAGETIME2EMPTY: u8 = 0x1C;

pub const HID_PD_IDEVICECHEMISTRY: u8 = 0x1F;       // Feature
pub const HID_PD_IOEMINFORMATION: u8 = 0x20;        // Feature

pub const IPRODUCT: u8 = 0x02;
pub const ISERIAL: u8 = 0x03;
pub const IMANUFACTURER: u8 = 0x01;

static HID_REPORT_DESCRIPTOR: [u8; 532] = [
    0x05, 0x84, // USAGE_PAGE (Power Device)
    0x09, 0x04, // USAGE (UPS)
    0xA1, 0x00, // COLLECTION (Physical)
    0x09, 0x16, //   USAGE (PowerConverter)
    0xA1, 0x00, //   COLLECTION (Physical)
    0x09, 0x17, //     USAGE (iProduct)
    0x85, 0x0b, //     REPORT_ID
    0x75, 0x08, //     REPORT_SIZE (8)
    0x95, 0x01, //     REPORT_COUNT (1)
    0x15, 0x00, //     LOGICAL_MINIMUM (0)
    0x26, 0xFF, 0x00, //     LOGICAL_MAXIMUM (255)
    0x65, 0x00, // Unit(None)
    0xb1, 0x03, // FATUTRE (Cnst,Var,Abs)
    0x09, 0x1d, // USAGE (Output)
    0xA1, 0x00, //   COLLECTION (Physical)
    0x09, 0x1d, // USAGE (OutputID)
    0xb1, 0x03, // FATUTRE (Cnst,Var,Abs)
    0x09, 0x30, // USAGE (Output)
    0x85, 0x0e, // REPORT_ID (14)
    0x67, 0x21, 0xd1, 0xf0, 0x00, // UNIT (SI Lin:Volts)
    0x55, 0x07, // UNIT_EXPONENT (7)
    0xb1, 0x83, // FEATURE (Cnst,Var,Abs,Vol)
    0x09, 0x53, //USAGE (LowVoltageTransfer)
    0x85, 0x13, // REPORT_ID (19)
    0xb1, 0x82, // FEATURE (Data,Var,Abs,Vol)
    0x09, 0x54, // USAGE (HighVoltageTransfer)
    0x75, 0x10, // REPORT_SIZE (16)
    0x26, 0xff, 0x7f, // LOGICAL_MAXIMUM (32767)
    0xb1, 0x82, // FEATURE (Data,Var,Abs,Vol)
    0xc0, // END_COLLECTION
    0xc0, // END_COLLECTION
    0x09, 0x1e, // USAGE (Flow)
    0xa1, 0x84, // COLLECTION (VendorDefined)
    0x09, 0x1f, // USAGE (FlowID)
    0x85, 0x0b, // REPORT_ID (11)
    0x75, 0x08, // REPORT_SIZE (8)
    0x95, 0x01, // REPORT_COUNT (1)
    0x65, 0x00, // UNIT (None)
    0x55, 0x00,
    0x26, 0xff, 0x00,
    0xb1, 0x03,
    0x09, 0x40,
    0x85, 0x12,
    0x67, 0x21, 0xd1, 0xf0, 0x00,
    0x55, 0x07,
    0xb1, 0x82,
    0x09, 0x42,
    0x85, 0x0d,
    0x66, 0x01, 0xf0,
    0x55, 0x00,
    0xb1, 0x83,
    0x09, 0x43,
    0x75, 0x10,
    0x26, 0xff, 0x7f,
    0x66, 0x21, 0xd1,
    0x55, 0x07,
    0xb1, 0x83,
    0xc0,
    0x09, 0x24,
    0xa1, 0x00,
    0x09, 0x25,
    0x09, 0x1f,
    0x85, 0x0b,
    0x75, 0x08,
    0x95, 0x02,
    0x26, 0xff, 0x00,
    0x65, 0x00,
    0x55, 0x00,
    0xb1, 0x03,
    0x05, 0x85,
    0x09, 0x2c,
    0x85, 0x0c,
    0x75, 0x08,
    0x95, 0x01,
    0xb1, 0x03,
    0x09, 0x29,
    0x09, 0x8d,
    0x95, 0x02,
    0x25, 0x64,
    0xb1, 0x03,
    0x09, 0x89,
    0x26, 0xff, 0x00,
    0x85, 0x10,
    0x95, 0x01,
    0xb1, 0x03,
    0x05, 0x84,
    0x09, 0xfd,
    0x09, 0xfe,
    0x09, 0xff,
    0x95, 0x03,
    0xb1, 0x03,
    0x09, 0x35,
    0x85, 0x0e,
    0x95, 0x01,
    0x65, 0x00,
    0x55, 0x00,
    0xb1, 0x83,
    0x05, 0x85,
    0x09, 0x83,
    0x09, 0x67,
    0x85, 0x0c,
    0x95, 0x02,
    0x75, 0x08,
    0x25, 0x64,
    0xb1, 0x03,
    0x09, 0x66,
    0x85, 0x16,
    0x95, 0x01,
    0xb1, 0x83,
    0x09, 0x66,
    0x81, 0x83,
    0x09, 0x68,
    0x66, 0x01, 0x10,
    0x75, 0x10,
    0x26, 0x08, 0x08,
    0xb1, 0x83,
    0x09, 0x68,
    0x81, 0x83,
    0x05, 0x84,
    0x09, 0x02,
    0xa1, 0x02,
    0x09, 0x73,
    0x85, 0x01,
    0x95, 0x01,
    0x75, 0x01,
    0x65, 0x00,
    0x25, 0x01,
    0x45, 0x00,
    0x81, 0x83,
    0x09, 0x73,
    0xb1, 0x83,
    0x09, 0x00,
    0x75, 0x07,
    0x81, 0x03,
    0x09, 0x00,
    0xb1, 0x03,
    0x05, 0x85,
    0x09, 0xd0,
    0x09, 0x44,
    0x09, 0x45,
    0x09, 0x42,
    0x09, 0x4b,
    0x0b, 0x61, 0x00, 0x84, 0x00,
    0x0b, 0x69, 0x00, 0x84, 0x00,
    0x0b, 0x65, 0x00, 0x84, 0x00,
    0x0b, 0x62, 0x00, 0x84, 0x00,
    0x85, 0x02,
    0x75, 0x01,
    0x95, 0x09,
    0x25, 0x01,
    0x81, 0x83,
    0x09, 0x00,
    0x75, 0x07,
    0x95, 0x01,
    0x81, 0x03,
    0x09, 0xd0,
    0x09, 0x44,
    0x09, 0x45,
    0x09, 0x42,
    0x09, 0x4b,
    0x0b, 0x61, 0x00, 0x84, 0x00,
    0x0b, 0x69, 0x00, 0x84, 0x00,
    0x0b, 0x65, 0x00, 0x84, 0x00,
    0x0b, 0x62, 0x00, 0x84, 0x00,
    0x95, 0x09,
    0x75, 0x01,
    0xb1, 0x83,
    0x09, 0x00,
    0x95, 0x01,
    0x75, 0x07,
    0xb1, 0x03,
    0xc0,
    0x05, 0x84,
    0x09, 0x57,
    0x85, 0x0f,
    0x75, 0x18,
    0x95, 0x01,
    0x66, 0x01, 0x10,
    0x15, 0xff,
    0x27, 0xfe, 0xff, 0x00,0x00,
    0xb1, 0x82,
    0x09, 0x56,
    0x85, 0x11,
    0x55, 0x01,
    0xb1, 0x82,
    0xc0,
    0x09, 0x18,
    0xa1, 0x00,
    0x09, 0x19,
    0x85, 0x0b,
    0x75, 0x08,
    0x95, 0x01,
    0x65, 0x00,
    0x55, 0x00,
    0x15, 0x00,
    0x26, 0xff, 0x00,
    0xb1, 0x03,
    0x09, 0x20,
    0xa1, 0x81,
    0x09, 0x21,
    0xb1, 0x03,
    0x09, 0x1f,
    0xb1, 0x03,
    0x09, 0x02,
    0xa1, 0x02,
    0x09, 0x6c,
    0x85, 0x0c,
    0x25, 0x01,
    0xb1, 0x03,
    0xc0,
    0xc0,
    0x09, 0x20,
    0xa1, 0x82,
    0x09, 0x21,
    0x85, 0x0b,
    0x26, 0xff, 0x00,
    0xb1, 0x03,
    0x09, 0x1f,
    0x85, 0x0d,
    0xb1, 0x83,
    0x09, 0x02,
    0xa1, 0x02,
    0x09, 0x6c,
    0x25, 0x01,
    0xb1, 0x83,
    0x09, 0x6b,
    0x85, 0x03,
    0x75, 0x01,
    0x81, 0x83,
    0x09, 0x6b,
    0xb1, 0x83,
    0x09, 0x00,
    0x75, 0x07,
    0x81, 0x03,
    0x09, 0x00,
    0xb1, 0x03,
    0xc0,
    0x0b, 0x29, 0x00, 0x85, 0x00,
    0x85, 0x14,
    0x75, 0x08,
    0x95, 0x01,
    0x65, 0x00,
    0x25, 0x64,
    0xb1, 0x82,
    0xc0,
    0xc0,
    0xc0,
];
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
    let hid = HIDClass::new(&usb_bus, &HID_REPORT_DESCRIPTOR, 1);
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