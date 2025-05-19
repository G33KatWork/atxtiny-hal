#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use panic_halt as _;

use atxtiny_hal::pac;
use atxtiny_hal::prelude::*;

use atxtiny_hal::gpio::{Gpiox, Output, Pin, Stateful, Ux};
use atxtiny_hal::timer::{tca::Event, tca::Interrupt, Counter, FTimer};

use core::cell::RefCell;
use core::mem::MaybeUninit;
use critical_section::Mutex;

struct InterruptState {
    pub counter: Counter<pac::TCA0, 312500>,
    pub led: Pin<Gpiox, Ux, Output<Stateful>>,
}

static INTERRUPT_STATE: Mutex<RefCell<MaybeUninit<InterruptState>>> = Mutex::new(RefCell::new(MaybeUninit::uninit()));

#[avr_device::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Constrain a few peripherals into our HAL types
    let clkctrl = dp.CLKCTRL.constrain();

    // Configure our clocks
    let clocks = clkctrl.freeze();

    // Split the PORTB peripheral into its pins
    let b = dp.PORTB.split();

    // Grab a pin for an LED
    let led = b.pb6.into_push_pull_output();

    // Create a timer with a fixed frequency using TCA0
    // If the frequency cannot be met given the constrained prescalers of the
    // passed counter in conjunction with the clock supplying the timer peripheral
    // an error is returned.
    let t = FTimer::<_, 312500>::new(dp.TCA0, clocks).unwrap();

    // Use the now configured fixed frequency timer to create a counter
    let mut c = t.counter();

    // Enable the overflow interrupt
    c.enable_interrupt(Interrupt::Overflow);

    // Start the counter with a timeout of 100ms
    // If the timeout cannot be met given the fixed frequency, start() returns
    // an Error
    c.start(100.millis()).unwrap();

    critical_section::with(|cs| {
        INTERRUPT_STATE.borrow(cs).borrow_mut().write(
            InterruptState {
                counter: c,
                led: led.downgrade().downgrade(),
            }
        );
    });

    // Enable the interrupts globally
    unsafe { avr_device::interrupt::enable() };

    loop {}
}

#[avr_device::interrupt(attiny817)]
fn TCA0_LUNF_OVF() {
    critical_section::with(|cs| {
        let mut cell = INTERRUPT_STATE.borrow(cs).borrow_mut();
        // SAFETY: We know this is initialized before interrupts are enabled
        let state = unsafe { cell.assume_init_mut() };

        // Clear the interrupt so it isn't triggered immediately after returning from this ISR
        state.counter.clear_event(Event::Overflow);

        // Toggle the LED
        state.led.toggle().unwrap();
    });
}
