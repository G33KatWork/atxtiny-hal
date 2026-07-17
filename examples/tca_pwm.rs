#![no_std]
#![no_main]

use panic_halt as _;

use atxtiny_hal::pac;
use atxtiny_hal::prelude::*;
use atxtiny_hal::timer::{tca::WaveformGenerationMode, Channel, FTimer};
use atxtiny_hal::traits::PwmTimer;

#[avr_device::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Constrain a few peripherals into our HAL types
    let clkctrl = dp.CLKCTRL.constrain();
    let portmux = dp.PORTMUX.constrain();

    // Configure our clocks
    let clocks = clkctrl.freeze().expect("valid clock config");

    // Create a timer with a fixed frequency using TCA0
    // If the frequency cannot be met given the constrained prescalers of the
    // passed counter in conjunction with the clock supplying the timer peripheral
    // an error is returned.
    let t = FTimer::<_, 312500>::new(dp.TCA0, clocks).unwrap();
    let tca0_clk = t.use_as_clock_source();

    // Build a set of PWM pins and multiplex them accordingly. WO0-WO2 sit
    // on PB0-PB2 on 14-pin-and-up packages and on PA3/PA1/PA2 on the
    // 8-pin ones.
    #[cfg(not(feature = "pins-8"))]
    let pwm_pins = {
        let b = dp.PORTB.split();
        (
            b.pb0.into_stateless_push_pull_output().mux(portmux.tca0_wo0),
            b.pb1.into_stateless_push_pull_output().mux(portmux.tca0_wo1),
            b.pb2.into_stateless_push_pull_output().mux(portmux.tca0_wo2),
        )
    };
    #[cfg(feature = "pins-8")]
    let pwm_pins = {
        let a = dp.PORTA.split();
        (
            // UFCS: PA3 is also TCA0's split-mode WO3, so plain `.mux()`
            // is ambiguous between the two pinset targets.
            IntoMuxedPinset::<pac::TCA0>::mux(
                a.pa3.into_stateless_push_pull_output(),
                portmux.tca0_wo0,
            ),
            a.pa1.into_stateless_push_pull_output().mux(portmux.tca0_wo1),
            a.pa2.into_stateless_push_pull_output().mux(portmux.tca0_wo2),
        )
    };

    // Use the now configured fixed frequency timer to create a PWM abstraction
    let mut pwm = t
        .pwm(pwm_pins, 10.millis(), WaveformGenerationMode::SingleSlope)
        .unwrap();

    // Enable all three channels and set a duty cycle
    pwm.set_duty_time(Channel::C1, 1.millis()).unwrap();
    pwm.enable(Channel::C1).unwrap();

    pwm.set_duty_time(Channel::C2, 1.millis()).unwrap();
    pwm.enable(Channel::C2).unwrap();

    pwm.set_duty_time(Channel::C3, 9.millis()).unwrap();
    pwm.enable(Channel::C3).unwrap();

    // Let's use TCB for an accurate delay
    let mut d = FTimer::<_, 312500>::new(dp.TCB0, tca0_clk).unwrap().delay();

    let mut i = 0;

    loop {
        pwm.set_duty_time(Channel::C1, i.millis()).unwrap();

        i += 1;

        if i > 10 {
            i = 0;
        }

        d.delay(100.millis());
    }
}
