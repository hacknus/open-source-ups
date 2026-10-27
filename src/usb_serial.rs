use crate::usb::{ActiveClass, CLASS};

#[allow(dead_code)]
pub fn usb_read(message: &mut [u8; 1024]) -> bool {
    cortex_m::interrupt::free(|cs| {
        *message = [0; 1024];
        return match CLASS.borrow(cs).borrow_mut().as_mut() {
            Some(ActiveClass::Serial(serial)) => match serial.read(message) {
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
            _ => false,
        };
    })
}

#[allow(dead_code)]
pub fn usb_println(string: &str) {
    cortex_m::interrupt::free(|cs| match CLASS.borrow(cs).borrow_mut().as_mut() {
        Some(ActiveClass::Serial(serial)) => {
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
        _ => {}
    })
}

#[allow(dead_code)]
pub fn usb_print(string: &str) {
    cortex_m::interrupt::free(|cs| match CLASS.borrow(cs).borrow_mut().as_mut() {
        Some(ActiveClass::Serial(serial)) => {
            serial.write(string.as_bytes()).unwrap_or(0);
            serial.flush().unwrap_or(());
        }
        _ => {}
    })
}
