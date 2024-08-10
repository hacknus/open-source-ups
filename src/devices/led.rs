use stm32f4xx_hal::gpio::{Output, Pin};

pub struct LED<const P: char, const N: u8> {
    pub pin: Pin<P, N, Output>,
    state: bool,
}

impl<const P: char, const N: u8> LED<P, N> {
    pub fn new(
        pin: Pin<P, N, Output>) -> Self {
        LED {
            pin,
            state: false,
        }
    }

    pub fn toggle(&mut self) {
        if self.state {
            self.pin.set_low();
            self.state = false;
        } else {
            self.pin.set_high();
            self.state = true;
        }
    }

    pub fn on(&mut self) {
        self.pin.set_high();
        self.state = true;
    }

    pub fn off(&mut self) {
        self.pin.set_low();
        self.state = false;
    }
}