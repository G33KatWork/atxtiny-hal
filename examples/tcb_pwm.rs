#![no_std]
#![no_main]

use panic_halt as _;

use atxtiny_hal::pac;
use atxtiny_hal::prelude::*;
use atxtiny_hal::timer::{
    rtc::RTCClockSource,
    tcb::{TCBClockSource, Tcb8bitPwmCapable},
    Channel, FTimer, Timer,
};
use atxtiny_hal::traits::PwmTimer;

#[avr_device::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Constrain a few peripherals into our HAL types
    let clkctrl = dp.CLKCTRL.constrain();
    let portmux = dp.PORTMUX.constrain();

    // Configure our clocks
    let clocks = clkctrl.freeze().expect("valid clock config");

    // Split the porta peripheral into its pins
    let a = dp.PORTA.split();

    // Demo LED pin per package: PB6 matches the LED on the ATtiny817
    // Xplained boards; the smaller packages just use a free pin.
    #[cfg(feature = "pins-24")]
    let mut led = dp.PORTB.split().pb6.into_push_pull_output();
    #[cfg(feature = "pins-20")]
    let mut led = dp.PORTB.split().pb5.into_push_pull_output();
    #[cfg(feature = "pins-14")]
    let mut led = dp.PORTB.split().pb3.into_push_pull_output();
    #[cfg(feature = "pins-8")]
    let mut led = a.pa3.into_push_pull_output();

    // PWM output: TCB0's waveform output is PA5, except on the 8-pin
    // packages where it is PA6. The UFCS call disambiguates on 8-pin
    // packages, where PA6 is also the CCL LUT0 output pin.
    #[cfg(not(feature = "pins-8"))]
    let pwm_wo = a.pa5.into_stateless_push_pull_output().mux(portmux.tcb0);
    #[cfg(feature = "pins-8")]
    let pwm_wo = IntoMuxedPinset::<pac::TCB0>::mux(
        a.pa6.into_stateless_push_pull_output(),
        portmux.tcb0,
    );

    // Delay timer
    let t = FTimer::<_, 1024>::new(dp.RTC, RTCClockSource::OSCULP32K_32K).unwrap();
    let mut d = t.delay();

    // The 16 KB+ 1-series parts have a second TCB whose waveform output is
    // PA3, with a PC4 alternate on the 24-pin packages. Drive a fixed-duty
    // PWM from TCB1 alongside the sweeping TCB0 one below, using the
    // alternate pin position where it exists so both routings are covered.
    #[cfg(all(feature = "periph-tcb1", feature = "pins-14"))]
    let pwm1_wo = a.pa3.into_stateless_push_pull_output().mux(portmux.tcb1);
    #[cfg(all(feature = "periph-tcb1", feature = "pins-24"))]
    let pwm1_wo = dp
        .PORTC
        .split()
        .pc4
        .into_stateless_push_pull_output()
        .mux(portmux.tcb1);

    #[cfg(feature = "periph-tcb1")]
    let mut pwm1 = {
        let t = Timer::new(dp.TCB1.into_8bit_pwm(), TCBClockSource::Peripheral(clocks));
        t.pwm_custom(pwm1_wo, 2, 255, ()).unwrap()
    };
    #[cfg(feature = "periph-tcb1")]
    {
        pwm1.set_duty(Channel::C1, 128).unwrap();
        pwm1.enable(Channel::C1).unwrap();
    }

    // Create a timer with a variable frequency using TCB0 in 8 Bit PWM mode
    let tcb0_8bit_pwm = dp.TCB0.into_8bit_pwm();
    let t = Timer::new(tcb0_8bit_pwm, TCBClockSource::Peripheral(clocks));

    // Build a PWM timer. Didive it down as much as possible. We should end up at about 39KHz
    let mut pwm = t.pwm_custom(pwm_wo, 2, 255, ()).unwrap();

    // Set the initial duty cycle and enable the channel
    pwm.set_duty(Channel::C1, 0).unwrap();
    pwm.enable(Channel::C1).unwrap();

    let mut i: u8 = 0;

    loop {
        // Play around with the duty cycle
        pwm.set_duty(Channel::C1, i.into()).unwrap();
        i = i.wrapping_add(10);

        // Toggle the LED
        led.toggle().unwrap();

        // Sleep
        d.delay(100.millis());
    }
}
