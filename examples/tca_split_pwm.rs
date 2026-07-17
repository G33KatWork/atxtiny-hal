//! TCA0 split-mode PWM: six 8-bit channels (three on the 8-pin parts).
//!
//! Split mode turns TCA0 into two 8-bit halves driving WO0-WO2 (low) and
//! WO3-WO5 (high). This HAL runs both halves at one shared period, so
//! the result is simply "more PWM channels at the same frequency".

#![no_std]
#![no_main]

use panic_halt as _;

use atxtiny_hal::pac;
use atxtiny_hal::prelude::*;
use atxtiny_hal::timer::{
    rtc::RTCClockSource,
    tca_split::{TCASplit, TcaSplitCapable},
    Channel, FTimer, Timer,
};
use atxtiny_hal::traits::PwmTimer;

#[avr_device::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    let clkctrl = dp.CLKCTRL.constrain();
    let portmux = dp.PORTMUX.constrain();
    let clocks = clkctrl.freeze().expect("valid clock config");

    let a = dp.PORTA.split();

    // Waveform outputs per package. WO3 sits on PA3 everywhere; its mux
    // call is spelled in UFCS form because PA3 also muxes to other
    // peripherals (WO0 on the 8-pin parts, TCB1 on the 16 KB+ parts).
    //
    // 8-pin parts: WO4/WO5 are not bonded, and WO0's default shares PA3
    // with WO3 — so use WO1 (PA1), WO2 (PA2) and WO3 (PA3).
    #[cfg(feature = "pins-8")]
    let pins = (
        a.pa1.into_stateless_push_pull_output().mux(portmux.tca0_wo1),
        a.pa2.into_stateless_push_pull_output().mux(portmux.tca0_wo2),
        IntoMuxedPinset::<TCASplit>::mux(
            a.pa3.into_stateless_push_pull_output(),
            portmux.tca0_wo3,
        ),
    );

    // Everything larger: all six outputs on their defaults — WO0-WO2 on
    // PB0-PB2, WO3-WO5 on PA3-PA5. PA4/PA5 are muxed in UFCS form as
    // well (they also serve other peripherals, e.g. PA5 is TCB0's
    // default waveform output).
    #[cfg(not(feature = "pins-8"))]
    let pins = {
        let b = dp.PORTB.split();
        (
            b.pb0.into_stateless_push_pull_output().mux(portmux.tca0_wo0),
            b.pb1.into_stateless_push_pull_output().mux(portmux.tca0_wo1),
            b.pb2.into_stateless_push_pull_output().mux(portmux.tca0_wo2),
            IntoMuxedPinset::<TCASplit>::mux(
                a.pa3.into_stateless_push_pull_output(),
                portmux.tca0_wo3,
            ),
            IntoMuxedPinset::<TCASplit>::mux(
                a.pa4.into_stateless_push_pull_output(),
                portmux.tca0_wo4,
            ),
            IntoMuxedPinset::<TCASplit>::mux(
                a.pa5.into_stateless_push_pull_output(),
                portmux.tca0_wo5,
            ),
        )
    };

    // Delay timer
    let t = FTimer::<_, 1024>::new(dp.RTC, RTCClockSource::OSCULP32K_32K).unwrap();
    let mut d = t.delay();

    // 8-bit PWM over all channels: prescaler 64, full 8-bit period.
    // Split mode has no waveform generation mode to choose, hence `()`.
    let t = Timer::new(dp.TCA0.into_split(), clocks);
    let mut pwm = t.pwm_custom(pins, 64, 255, ()).unwrap();

    #[cfg(feature = "pins-8")]
    let channels = [Channel::C2, Channel::C3, Channel::C4];
    #[cfg(not(feature = "pins-8"))]
    let channels = [
        Channel::C1,
        Channel::C2,
        Channel::C3,
        Channel::C4,
        Channel::C5,
        Channel::C6,
    ];

    for (i, ch) in channels.iter().enumerate() {
        // Spread the duty cycles out so each output is distinguishable.
        pwm.set_duty(*ch, 32 * (i as u32 + 1)).unwrap();
        pwm.enable(*ch).unwrap();
    }

    let mut offset: u32 = 0;

    loop {
        // Slowly rotate all duty cycles.
        for (i, ch) in channels.iter().enumerate() {
            let duty = (32 * (i as u32 + 1) + offset) % 256;
            pwm.set_duty(*ch, duty).unwrap();
        }
        offset = (offset + 8) % 256;

        d.delay(100.millis());
    }
}
