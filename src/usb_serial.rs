use core::cell::RefCell;

// use arrform::{arrform, ArrForm};
use cortex_m::interrupt::Mutex;
use stm32f4xx_hal::otg_fs::{UsbBus, USB};
use stm32f4xx_hal::pac::interrupt;
use usb_device::bus::UsbBusAllocator;
use usb_device::prelude::*;
use usbd_serial::SerialPort;

// Make USB serial device globally available
pub static G_USB_SERIAL: Mutex<RefCell<Option<SerialPort<UsbBus<USB>>>>> =
    Mutex::new(RefCell::new(None));

// Make USB device globally available
pub static G_USB_DEVICE: Mutex<RefCell<Option<UsbDevice<UsbBus<USB>>>>> =
    Mutex::new(RefCell::new(None));

pub static mut EP_MEMORY: [u32; 1024] = [0; 1024];
static mut USB_BUS: Option<UsbBusAllocator<stm32f4xx_hal::otg_fs::UsbBusType>> = None;

#[allow(dead_code)]
pub unsafe fn usb_serial_init(usb: USB) {
    USB_BUS = Some(stm32f4xx_hal::otg_fs::UsbBusType::new(usb, &mut EP_MEMORY));
    let usb_bus = USB_BUS.as_ref().unwrap();
    let serial_port = SerialPort::new(&usb_bus);
    let usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x03f0, 0x1f06))
        .device_class(usbd_serial::USB_CLASS_CDC)
        .strings(&[StringDescriptors::default()
            .manufacturer("hacknus")
            .product("UPS")
            .serial_number("UPS10")])
        .unwrap()
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
            None => false,
            Some(serial) => match serial.read(message) {
                Ok(a) => {
                    if a < 1024 {
                        true
                    } else {
                        false
                    }
                }
                Err(_err) => {
                    // usb_println(arrform!(128, "Serial read Error: {:?}", err).as_str());
                    // let _ = serial.flush();
                    false
                }
            },
        };
    })
}

#[allow(dead_code)]
pub fn usb_println(string: &str) {
    cortex_m::interrupt::free(|cs| match G_USB_SERIAL.borrow(cs).borrow_mut().as_mut() {
        None => {}
        Some(serial) => {
            let string_bytes = string.as_bytes();
            let mut index = 0;
            let length = 32;
            loop {
                if string_bytes.len() > index + length {
                    let bytes_to_send = &string_bytes[index..index + length];
                    serial.write(bytes_to_send).unwrap_or(0);
                    serial.flush().unwrap_or(());
                } else {
                    let bytes_to_send = &string_bytes[index..];
                    serial.write(bytes_to_send).unwrap_or(0);
                    serial.flush().unwrap_or(());
                    break;
                }
                index += length;
            }
            serial.write(b"\r\n").unwrap_or(0);
            serial.flush().unwrap_or(());
        }
    })
}

#[allow(dead_code)]
pub fn usb_print(string: &str) {
    cortex_m::interrupt::free(|cs| match G_USB_SERIAL.borrow(cs).borrow_mut().as_mut() {
        None => {}
        Some(serial) => {
            serial.write(string.as_bytes()).unwrap_or(0);
            serial.flush().unwrap_or(());
        }
    })
}
