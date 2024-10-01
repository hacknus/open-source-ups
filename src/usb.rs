use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use stm32f4xx_hal::otg_fs::{UsbBus, USB, UsbBusType};
use stm32f4xx_hal::pac::{interrupt};
use usb_device::bus::UsbBusAllocator;
use usb_device::device::{StringDescriptors, UsbDevice, UsbDeviceBuilder, UsbVidPid};
use usbd_hid_device::{USB_CLASS_HID, Hid};
use crate::report::Report;


// Make USB serial device globally available
pub static G_USB_HID: Mutex<RefCell<Option<Hid<Report, UsbBus<USB>>>>> =
    Mutex::new(RefCell::new(None));

// Make USB device globally available
pub static G_USB_DEVICE: Mutex<RefCell<Option<UsbDevice<UsbBus<USB>>>>> =
    Mutex::new(RefCell::new(None));

#[allow(dead_code)]
pub unsafe fn usb_init(usb: USB) {
    static mut EP_MEMORY: [u32; 1024] = [0; 1024];
    static mut USB_BUS: Option<UsbBusAllocator<UsbBusType>> = None;
    USB_BUS = Some(UsbBusType::new(usb, &mut EP_MEMORY));
    let usb_bus = USB_BUS.as_ref().unwrap();
    let hid = Hid::new(usb_bus, 10);
    let usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x03f0, 0x1f06))
        .device_class(USB_CLASS_HID)
        .strings(&[StringDescriptors::default()
            .manufacturer("hacknus")
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