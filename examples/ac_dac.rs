// FIXME: TODO

#![no_std]
#![no_main]

use panic_halt as _;

use atxtiny_hal::ac::{ComparatorExt, Config};
use atxtiny_hal::dac::DacExt;
use atxtiny_hal::pac;
use atxtiny_hal::prelude::*;
use atxtiny_hal::timer::FTimer;
use atxtiny_hal::vref::{ReferenceVoltage, VrefExt};

#[avr_device::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Constrain a few peripherals into our HAL types
    let clkctrl = dp.CLKCTRL.constrain();

    // Configure our clocks
    let clocks = clkctrl.freeze().expect("valid clock config");

    // Split the PORTA peripheral into its pins
    let a = dp.PORTA.split();

    // Blinky things. PB6 matches the LED on the ATtiny817 Xplained boards;
    // the smaller packages just use a free pin.
    #[cfg(feature = "pins-24")]
    let mut led = dp.PORTB.split().pb6.into_push_pull_output();
    #[cfg(feature = "pins-20")]
    let mut led = dp.PORTB.split().pb5.into_push_pull_output();
    #[cfg(feature = "pins-14")]
    let mut led = dp.PORTB.split().pb3.into_push_pull_output();
    #[cfg(feature = "pins-8")]
    let mut led = a.pa1.into_push_pull_output();

    // Setup VREF for DAC to 2.5V
    let vref_parts = dp.VREF.constrain();
    let mut vref = vref_parts.vref;
    let mut dacref = vref_parts.dac0;
    dacref.voltage(&mut vref, ReferenceVoltage::_2V50);

    // Setup the DAC
    let mut dac = dp.DAC0.constrain(dacref);
    dac.dac_set_value(0);
    let dac = dac.enable();

    // Lock the DAC into an enabled state, now it cannot be disabled anymore
    // but we can also get output objects that can be passed into other
    // peripherals like the negative AC0 pin
    let mut dac = dac.lock_enable();

    // Grab AINP0
    let ainp0 = a.pa7.into_analog_input();

    // Grab the DAC as AINN0
    let ainn0 = dac.dac_get_ac0_input();

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

    // Create a delay timer
    let t = FTimer::<_, 312500>::new(dp.TCA0, clocks).unwrap();
    let mut d = t.delay();

    let mut i: u8 = 0;

    loop {
        led.toggle().unwrap();
        dac.dac_set_value(i);

        i = i.wrapping_add(4);
        d.delay(50.millis());
    }
}
