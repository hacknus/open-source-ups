use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use stm32f4xx_hal::otg_fs::{UsbBus, USB};
use stm32f4xx_hal::pac::{interrupt};
use usb_device::prelude::*;
use usb_device::bus::UsbBusAllocator;
use usbd_hid::hid_class::HIDClass;


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
    let hid = HIDClass::new(&usb_bus, HID_REPORT_DESCRIPTOR, 1);
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