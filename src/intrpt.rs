use core::cell::{Cell, RefCell};
use panic_halt as _;
use cortex_m::interrupt::Mutex;
use stm32f4xx_hal::{
    pac::{interrupt},
    gpio::{self, Input},
    prelude::*,
};
// Create a Global Variable for the GPIO Peripheral that I'm going to pass around.
pub static G_BUTTON: Mutex<RefCell<Option<gpio::PB8<Input>>>> = Mutex::new(RefCell::new(None));
// Create a Global Variable for the state
pub static G_STATE: Mutex<Cell<bool>> = Mutex::new(Cell::new(false));

#[interrupt]
fn EXTI9_5() {
    // Start a Critical Section
    cortex_m::interrupt::free(|cs| {
        G_STATE
            .borrow(cs)
            .set(!G_STATE.borrow(cs).get());
        // Obtain access to Global Button Peripheral and Clear Interrupt Pending Flag
        let mut button = G_BUTTON.borrow(cs).borrow_mut();
        button.as_mut().unwrap().clear_interrupt_pending_bit();
    });
}