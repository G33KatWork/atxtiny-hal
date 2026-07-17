#![no_std]
#![no_main]

use panic_halt as _;

use atxtiny_hal::dac::DacExt;
use atxtiny_hal::pac;
use atxtiny_hal::prelude::*;
use atxtiny_hal::vref::{ReferenceVoltage, VrefExt};

#[avr_device::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Constrain a few peripherals into our HAL types
    let clkctrl = dp.CLKCTRL.constrain();

    // Configure our clocks
    let _clocks = clkctrl.freeze().expect("valid clock config");

    // Split the PORTA peripheral into its pins
    let a = dp.PORTA.split();

    // Put the DAC output pin into analog mode (input buffer and output
    // driver disabled, as the datasheet recommends for DAC output)
    let dacout = a.pa6.into_analog_input();

    // Set up the reference voltage
    // Note: the DAC takes ownership of the (non-clonable) reference token,
    //       so nothing else can reconfigure the reference behind its back
    let vref_parts = dp.VREF.constrain();
    let mut vref = vref_parts.vref;
    let mut dacref = vref_parts.dac0;
    dacref.voltage(&mut vref, ReferenceVoltage::_4V34);

    let mut dac = dp.DAC0.constrain(dacref).output_pin(dacout);
    dac.dac_set_value(128);
    let _dac = dac.enable();

    loop {}
}
