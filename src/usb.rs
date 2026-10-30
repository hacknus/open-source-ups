//! One USB device and one active class, shared by initialization and the ISR.
use crate::report::Telemetry;
use crate::usb_hid::UpsHid;
use crate::usb_link::{LinkAction, UsbLink};
use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use stm32f4xx_hal::otg_fs::{UsbBus, UsbBusType, USB};
use stm32f4xx_hal::pac::interrupt;
use usb_device::bus::UsbBusAllocator;
use usb_device::device::{
    StringDescriptors, UsbDevice, UsbDeviceBuilder, UsbDeviceState, UsbVidPid,
};
use usbd_serial::SerialPort;

type Bus = UsbBus<USB>;
pub enum ActiveClass {
    Hid(UpsHid<'static, Bus>),
    Serial(SerialPort<'static, Bus>),
}
pub static CLASS: Mutex<RefCell<Option<ActiveClass>>> = Mutex::new(RefCell::new(None));
static DEVICE: Mutex<RefCell<Option<UsbDevice<'static, Bus>>>> = Mutex::new(RefCell::new(None));
static LINK: Mutex<RefCell<UsbLink>> = Mutex::new(RefCell::new(UsbLink::new()));

pub fn init(usb: USB, hid_mode: bool) {
    let memory = cortex_m::singleton!(: [u32; 1024] = [0; 1024]).unwrap();
    let slot = cortex_m::singleton!(: Option<UsbBusAllocator<UsbBusType>> = None).unwrap();
    let bus = slot.insert(UsbBusType::new(usb, memory));
    let class = if hid_mode {
        ActiveClass::Hid(UpsHid::new(bus))
    } else {
        ActiveClass::Serial(SerialPort::new(bus))
    };
    // Existing HP compatibility VID, with a PID supported by older NUT mge-hid.
    let device = UsbDeviceBuilder::new(bus, UsbVidPid(0x03f0, 0x1fe7))
        .device_release(0x0102) // Visible as version 1.02 in the host USB device list.
        .self_powered(true)
        .max_power(0)
        .unwrap()
        .device_class(if hid_mode {
            0
        } else {
            usbd_serial::USB_CLASS_CDC
        })
        .strings(&[StringDescriptors::default()
            .manufacturer("hacknus")
            .product("UPS")
            .serial_number("UPS10")])
        .unwrap()
        .build();
    cortex_m::interrupt::free(|cs| {
        CLASS.borrow(cs).replace(Some(class));
        DEVICE.borrow(cs).replace(Some(device));
    });
}

pub fn update(telemetry: Telemetry, refresh: bool) {
    cortex_m::interrupt::free(|cs| {
        if let Some(ActiveClass::Hid(hid)) = CLASS.borrow(cs).borrow_mut().as_mut() {
            hid.update(telemetry, refresh);
        }
    });
    service();
}

pub fn tick(elapsed_ms: u16) {
    cortex_m::interrupt::free(|cs| {
        if let Some(device) = DEVICE.borrow(cs).borrow().as_ref() {
            // Observed SOF/reset/packet activity, not cached connection state.
            let activity = device.bus().host_activity();
            let transactions = device.bus().host_transactions();
            if let Some(action) = LINK.borrow(cs).borrow_mut().tick(
                device.state(),
                activity,
                transactions,
                elapsed_ms,
            ) {
                match action {
                    LinkAction::Disconnect => device.bus().disconnect(),
                    LinkAction::Connect => device.bus().restart(),
                }
            }
        }
        if let Some(ActiveClass::Hid(hid)) = CLASS.borrow(cs).borrow_mut().as_mut() {
            hid.tick(elapsed_ms);
        }
    });
}

pub fn service() {
    cortex_m::interrupt::free(|cs| {
        if LINK.borrow(cs).borrow().is_disconnected() {
            return;
        }
        let mut device = DEVICE.borrow(cs).borrow_mut();
        let mut class = CLASS.borrow(cs).borrow_mut();
        let (Some(device), Some(class)) = (device.as_mut(), class.as_mut()) else {
            return;
        };
        match class {
            ActiveClass::Hid(hid) => {
                device.poll(&mut [hid]);
                hid.set_configured(device.state() == UsbDeviceState::Configured);
                hid.flush_pending().ok();
            }
            ActiveClass::Serial(serial) => {
                device.poll(&mut [serial]);
            }
        }
    });
}

#[interrupt]
fn OTG_FS() {
    service();
}
