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
    let clocks = clkctrl.freeze().expect("valid clock config");

    // Demo LED pin per package: PB6 matches the LED on the ATtiny817
    // Xplained boards; the smaller packages just use a free pin.
    #[cfg(feature = "pins-24")]
    let led = dp.PORTB.split().pb6.into_push_pull_output();
    #[cfg(feature = "pins-20")]
    let led = dp.PORTB.split().pb5.into_push_pull_output();
    #[cfg(feature = "pins-14")]
    let led = dp.PORTB.split().pb3.into_push_pull_output();
    #[cfg(feature = "pins-8")]
    let led = dp.PORTA.split().pa3.into_push_pull_output();

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

// The interrupt attribute needs the concrete chip name to resolve the
// vector, so the selected device feature picks the matching variant.
// The TCA0_LUNF_OVF vector itself exists on every supported chip.
#[cfg_attr(feature = "attiny202", avr_device::interrupt(attiny202))]
#[cfg_attr(feature = "attiny204", avr_device::interrupt(attiny204))]
#[cfg_attr(feature = "attiny402", avr_device::interrupt(attiny402))]
#[cfg_attr(feature = "attiny404", avr_device::interrupt(attiny404))]
#[cfg_attr(feature = "attiny804", avr_device::interrupt(attiny804))]
#[cfg_attr(feature = "attiny1604", avr_device::interrupt(attiny1604))]
#[cfg_attr(feature = "attiny1606", avr_device::interrupt(attiny1606))]
#[cfg_attr(feature = "attiny212", avr_device::interrupt(attiny212))]
#[cfg_attr(feature = "attiny214", avr_device::interrupt(attiny214))]
#[cfg_attr(feature = "attiny412", avr_device::interrupt(attiny412))]
#[cfg_attr(feature = "attiny414", avr_device::interrupt(attiny414))]
#[cfg_attr(feature = "attiny416", avr_device::interrupt(attiny416))]
#[cfg_attr(feature = "attiny417", avr_device::interrupt(attiny417))]
#[cfg_attr(feature = "attiny816", avr_device::interrupt(attiny816))]
#[cfg_attr(feature = "attiny817", avr_device::interrupt(attiny817))]
#[cfg_attr(feature = "attiny1614", avr_device::interrupt(attiny1614))]
#[cfg_attr(feature = "attiny1617", avr_device::interrupt(attiny1617))]
#[cfg_attr(feature = "attiny3217", avr_device::interrupt(attiny3217))]
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
