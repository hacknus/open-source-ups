use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use stm32f4xx_hal::otg_fs::{UsbBus, USB};
use stm32f4xx_hal::pac::{interrupt};
use usb_device::prelude::*;
use usb_device::bus::UsbBusAllocator;
use usbd_serial::SerialPort;

// Make USB serial device globally available
pub static G_USB_SERIAL: Mutex<RefCell<Option<SerialPort<UsbBus<USB>>>>> =
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
    let serial_port = SerialPort::new(&usb_bus);
    let usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x051D, 0x0001))
        .manufacturer("American Power Conversion")
        .product("UPS")
        .serial_number("001")
        .device_class(usbd_serial::USB_CLASS_CDC)
        .build();
    cortex_m::interrupt::free(|cs| {
        *G_USB_SERIAL.borrow(cs).borrow_mut() = Some(serial_port);
        *G_USB_DEVICE.borrow(cs).borrow_mut() = Some(usb_dev);
    });
}

#[allow(dead_code)]
pub fn usb_read(message: &mut [u8; 1024]) -> bool {
    cortex_m::interrupt::free(|cs| {
        *message = [0; 1024];
        return match G_USB_SERIAL.borrow(cs).borrow_mut().as_mut() {
            None => { false }
            Some(serial) => {
                match serial.read(message) {
                    Ok(_) => { true }
                    Err(_) => { false }
                }
            }
        };
    })
}

#[allow(dead_code)]
pub fn usb_println(string: &str) -> bool {
    cortex_m::interrupt::free(|cs| {
        match G_USB_SERIAL.borrow(cs).borrow_mut().as_mut() {
            None => { false }
            Some(serial) => {
                match serial.write(string.as_bytes()) {
                    Ok(_) => {}
                    Err(_) => { return false; }
                };
                match serial.write(b"\r\n") {
                    Ok(_) => { true }
                    Err(_) => { false }
                }
            }
        }
    })
}

#[allow(dead_code)]
pub fn usb_print(string: &str) {
    cortex_m::interrupt::free(|cs| {
        match G_USB_SERIAL.borrow(cs).borrow_mut().as_mut() {
            None => {}
            Some(serial) => {
                serial.write(string.as_bytes()).expect("serial write failed");
            }
        }
    });
}

#[interrupt]
#[allow(non_snake_case)]
fn OTG_FS() {
    cortex_m::interrupt::free(|cs| {
        match G_USB_DEVICE.borrow(cs).borrow_mut().as_mut() {
            None => {}
            Some(usb_dev) => {
                match G_USB_SERIAL.borrow(cs).borrow_mut().as_mut() {
                    None => {}
                    Some(serial) => {
                        // do this regularly to keep connection to USB host
                        usb_dev.poll(&mut [serial]);
                    }
                }
            }
        }
    });
}