#![no_std]
#![no_main]

use panic_halt as _;

use atxtiny_hal::ac::{ComparatorExt, Config};
use atxtiny_hal::pac;
use atxtiny_hal::prelude::*;

#[avr_device::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Constrain a few peripherals into our HAL types
    let clkctrl = dp.CLKCTRL.constrain();

    // Configure our clocks
    let _clocks = clkctrl.freeze().expect("valid clock config");

    // Split the PORTA peripheral into its pins
    let a = dp.PORTA.split();

    // Grab AINN0 & AINP0
    let ainn0 = a.pa6.into_analog_input();
    let ainp0 = a.pa7.into_analog_input();

    // Grab the AC output pin (PA5, or PA3 on 8-pin packages) and disable
    // its pullup
    #[cfg(not(feature = "pins-8"))]
    let mut acout = a.pa5.into_stateless_push_pull_output();
    #[cfg(feature = "pins-8")]
    let mut acout = a.pa3.into_stateless_push_pull_output();
    acout.internal_pull_up(Toggle::Off);

    // Create a comparator
    let ac = dp.AC0.comparator(
        ainp0,
        ainn0,
        Config {
            hysteresis: atxtiny_hal::ac::Hysteresis::_50mV,
            ..Default::default()
        },
    );
    ac.output_pin(acout);
    let _ac = ac.enable();

    loop {}
}
