use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use stm32f4xx_hal::otg_fs::{UsbBus, USB};
use stm32f4xx_hal::pac::{interrupt};
use usb_device::prelude::*;
use usb_device::bus::UsbBusAllocator;
use usbd_hid::hid_class::HIDClass;
use crate::HID_REPORT_DESCRIPTOR;

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
    let mut hid = HIDClass::new(&usb_bus, HID_REPORT_DESCRIPTOR, 1);
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x051D, 0x0001))
        .manufacturer("Linus Leo Stöckli")
        .product("UPS")
        .serial_number("UPS10")
        .device_class(0)
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