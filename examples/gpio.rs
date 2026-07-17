#![no_std]
#![no_main]

use panic_halt as _;

use atxtiny_hal::pac;
use atxtiny_hal::prelude::*;

#[avr_device::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let clkctrl = dp.CLKCTRL.constrain();

    let _clocks = clkctrl.freeze().expect("valid clock config");

    // Demo pins per package: on the ATtiny817 Xplained boards PB7 is the
    // button and PB6 the LED; the smaller packages just use free pins.
    #[cfg(feature = "pins-24")]
    let (mut btn, mut led, mut led2) = {
        let b = dp.PORTB.split();
        (
            b.pb7.into_pull_up_input(),
            b.pb6.into_push_pull_output(),
            b.pb5.into_push_pull_output(),
        )
    };
    #[cfg(feature = "pins-20")]
    let (mut btn, mut led, mut led2) = {
        let b = dp.PORTB.split();
        (
            b.pb4.into_pull_up_input(),
            b.pb5.into_push_pull_output(),
            b.pb3.into_push_pull_output(),
        )
    };
    #[cfg(feature = "pins-14")]
    let (mut btn, mut led, mut led2) = {
        let b = dp.PORTB.split();
        (
            b.pb0.into_pull_up_input(),
            b.pb1.into_push_pull_output(),
            b.pb2.into_push_pull_output(),
        )
    };
    #[cfg(feature = "pins-8")]
    let (mut btn, mut led, mut led2) = {
        let a = dp.PORTA.split();
        (
            a.pa1.into_pull_up_input(),
            a.pa2.into_push_pull_output(),
            a.pa3.into_push_pull_output(),
        )
    };

    let mut i = 0;

    loop {
        if btn.is_low().unwrap() {
            led2.set_high().unwrap();
        } else {
            led2.set_low().unwrap();
        }

        if i % 10000 == 0 {
            led.toggle().unwrap();
        }

        i += 1;
    }
}
