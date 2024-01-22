#![no_std]
#![no_main]

use panic_halt as _;

use atxtiny_hal::prelude::*;
use atxtiny_hal::pac;
use atxtiny_hal::timer::{FTimer, Timer, Channel, tcb::{TCBClockSource, Tcb8bitPwmCapable}, rtc::RTCClockSource};

#[avr_device::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Constrain a few peripherals into our HAL types
    let clkctrl = dp.CLKCTRL.constrain();
    let portmux = dp.PORTMUX.constrain();

    // Configure our clocks
    let clocks = clkctrl.freeze();

    // Split the PORTB peripheral into its pins
    let (a, b) = (dp.PORTA.split(), dp.PORTB.split());

    // Grab a pin for an LED
    let mut led = b.pb6.into_push_pull_output();

    // PWM output
    let pwm_wo = a.pa5.into_stateless_push_pull_output().mux(&portmux);

    // Delay timer
    let t = FTimer::<_, 1024>::new(dp.RTC, RTCClockSource::OSCULP32K_32K).unwrap();
    let mut d = t.delay();

    // Create a timer with a variable frequency using TCB0 in 8 Bit PWM mode
    let tcb0_8bit_pwm = dp.TCB0.into_8bit_pwm();
    let t = Timer::new(tcb0_8bit_pwm, TCBClockSource::Peripheral(clocks));
    
    // Build a PWM timer. Didive it down as much as possible. We should end up at about 39KHz
    let mut pwm = t.pwm_custom(pwm_wo, 2, 255, ()).unwrap();

    // Set the initial duty cycle and enable the channel
    pwm.set_duty(Channel::C1, 0);
    pwm.enable(Channel::C1);

    let mut i = 0;

    loop {
        // Play around with the duty cycle
        pwm.set_duty(Channel::C1, i);
        i = i.wrapping_add(10);

        // Toggle the LED
        led.toggle().unwrap();
        
        // Sleep
        d.delay(100.millis());
    }
}
