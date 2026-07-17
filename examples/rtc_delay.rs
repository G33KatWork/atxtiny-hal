#![no_std]
#![no_main]

use panic_halt as _;

use atxtiny_hal::pac;
use atxtiny_hal::prelude::*;
use atxtiny_hal::timer::{rtc::RTCClockSource, FTimer};

#[avr_device::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Constrain a few peripherals into our HAL types
    let clkctrl = dp.CLKCTRL.constrain();

    // Configure our clocks
    let _clocks = clkctrl.freeze().expect("valid clock config");

    // Demo LED pin per package: PB6 matches the LED on the ATtiny817
    // Xplained boards; the smaller packages just use a free pin.
    #[cfg(feature = "pins-24")]
    let mut led = dp.PORTB.split().pb6.into_push_pull_output();
    #[cfg(feature = "pins-20")]
    let mut led = dp.PORTB.split().pb5.into_push_pull_output();
    #[cfg(feature = "pins-14")]
    let mut led = dp.PORTB.split().pb3.into_push_pull_output();
    #[cfg(feature = "pins-8")]
    let mut led = dp.PORTA.split().pa3.into_push_pull_output();

    // Create a timer with a fixed frequency using TCA0
    // If the frequency cannot be met given the constrained prescalers of the
    // passed counter in conjunction with the clock supplying the timer peripheral
    // an error is returned.
    let t = FTimer::<_, 1024>::new(dp.RTC, RTCClockSource::OSCULP32K_32K).unwrap();

    // Use the now configured fixed frequency timer to create a delay
    let mut d = t.delay();

    loop {
        // Toggle the LED
        led.toggle().unwrap();

        // Sleep
        d.delay(500.millis());
    }
}
