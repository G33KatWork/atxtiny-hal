#![no_std]
#![no_main]

use panic_halt as _;

use atxtiny_hal::ac::{ComparatorExt, Config};
use atxtiny_hal::evsys::EvsysExt;
use atxtiny_hal::pac;
use atxtiny_hal::prelude::*;

use atxtiny_hal::evsys::EventGenerator;

#[avr_device::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Constrain a few peripherals into our HAL types
    let clkctrl = dp.CLKCTRL.constrain();
    let portmux = dp.PORTMUX.constrain();

    // Configure our clocks
    let _clocks = clkctrl.freeze().expect("valid clock config");

    // Split the PORTA/B/C peripheral into their pins
    let (a, b, c) = (dp.PORTA.split(), dp.PORTB.split(), dp.PORTC.split());

    // Grab AINN0 & AINP0
    let ainn0 = a.pa6.into_analog_input();
    let ainp0 = a.pa7.into_analog_input();

    // Grab EVOUT0 at PA2
    let pa2 = a.pa2.into_peripheral();
    let evout0 = pa2.mux(portmux.evout0);

    // Grab EVOUT1 at PB2
    let pb2 = b.pb2.into_peripheral();
    let evout1 = pb2.mux(portmux.evout1);

    // Grab EVOUT2 at PC2
    let pc2 = c.pc2.into_peripheral();
    let evout2 = pc2.mux(portmux.evout2);

    // Create a comparator
    let mut ac = dp.AC0.comparator(
        ainp0,
        ainn0,
        Config {
            hysteresis: atxtiny_hal::ac::Hysteresis::_50mV,
            ..Default::default()
        },
    );

    // Grab the event system channels
    let evsys = dp.EVSYS.split();

    // AC event -> EVOUT0 (PA2)
    //
    // `connect_event_user` consumes the user token; one channel drives one
    // user in this API. To fan one generator out to several users, assign
    // a second channel to the same generator (hardware allows it), or poke
    // the user registers directly.
    let async_ch0 = evsys.channel_async0;
    let async_ch0 = ac.connect_event_generator(async_ch0, ());
    let _async_ch0_evout0 = async_ch0.connect_event_user(evout0);

    // AC event -> EVOUT1 (PB2)
    let async_ch2 = evsys.channel_async2;
    let async_ch2 = ac.connect_event_generator(async_ch2, ());
    let _async_ch2 = async_ch2.connect_event_user(evout1);

    let _ac = ac.enable();

    // PB0 event -> EVOUT2 (PC2)
    let mut b0 = b.pb0.into_pull_up_input();
    let async_ch1 = evsys.channel_async1;
    let async_ch1 = b0.connect_event_generator(async_ch1, ());
    let _async_ch1 = async_ch1.connect_event_user(evout2);

    loop {}
}
