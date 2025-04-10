#![no_std]
#![no_main]

use panic_halt as _;

use atxtiny_hal::pac;
use atxtiny_hal::prelude::*;
use atxtiny_hal::timer::FTimer;

#[avr_device::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Constrain a few peripherals into our HAL types
    let clkctrl = dp.clkctrl.constrain();

    // Configure our clocks
    let clocks = clkctrl.freeze();

    // Split the PORTB peripheral into its pins
    let b = dp.portb.split();

    // Grab a pin for an LED
    let mut led = b.pb6.into_push_pull_output();

    // Create a timer with a fixed frequency using TCA0
    // If the frequency cannot be met given the constrained prescalers of the
    // passed counter in conjunction with the clock supplying the timer peripheral
    // an error is returned.
    let t = FTimer::<_, 312500>::new(dp.tca0, clocks).unwrap();

    // Use the now configured fixed frequency timer to create a delay
    let mut d = t.delay();

    loop {
        // Toggle the LED
        led.toggle().unwrap();

        // Sleep
        d.delay(500.millis());
    }
}
