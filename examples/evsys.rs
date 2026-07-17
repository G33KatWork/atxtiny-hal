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

    // Split the ports that exist on the selected package
    let a = dp.PORTA.split();
    #[cfg(feature = "port-b")]
    let b = dp.PORTB.split();
    #[cfg(feature = "port-c")]
    let c = dp.PORTC.split();

    // Grab AINN0 & AINP0
    let ainn0 = a.pa6.into_analog_input();
    let ainp0 = a.pa7.into_analog_input();

    // Grab EVOUT0 at PA2
    let pa2 = a.pa2.into_peripheral();
    let evout0 = pa2.mux(portmux.evout0);

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

    // AC event -> EVOUT1 (PB2). Needs both a PORTB and a third async
    // channel, which only the 1-series has.
    #[cfg(all(feature = "series-1", feature = "port-b"))]
    {
        let evout1 = b.pb2.into_peripheral().mux(portmux.evout1);
        let async_ch2 = evsys.channel_async2;
        let async_ch2 = ac.connect_event_generator(async_ch2, ());
        let _async_ch2 = async_ch2.connect_event_user(evout1);
    }

    let _ac = ac.enable();

    // PB0 event -> event output. PORTB pins route to ASYNCCH1; the output
    // goes to EVOUT2 (PC2) where a PORTC exists, otherwise to EVOUT1 (PB2)
    // on the 0-series 14-pin parts (whose EVOUT1 the AC demo above does
    // not occupy). On 14-pin 1-series parts every bonded event output is
    // already taken, so this part of the demo is skipped there.
    #[cfg(feature = "port-c")]
    {
        let evout2 = c.pc2.into_peripheral().mux(portmux.evout2);
        let mut b0 = b.pb0.into_pull_up_input();
        let async_ch1 = evsys.channel_async1;
        let async_ch1 = b0.connect_event_generator(async_ch1, ());
        let _async_ch1 = async_ch1.connect_event_user(evout2);
    }
    #[cfg(all(feature = "series-0", feature = "port-b", not(feature = "port-c")))]
    {
        let evout1 = b.pb2.into_peripheral().mux(portmux.evout1);
        let mut b0 = b.pb0.into_pull_up_input();
        let async_ch1 = evsys.channel_async1;
        let async_ch1 = b0.connect_event_generator(async_ch1, ());
        let _async_ch1 = async_ch1.connect_event_user(evout1);
    }

    loop {}
}
