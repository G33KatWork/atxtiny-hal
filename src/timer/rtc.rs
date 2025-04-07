//! # Real Time Counter

#[cfg(feature = "enumset")]
use enumset::EnumSetType;

use crate::{
    pac::{rtc::ctrla, Rtc},
    time::*,
    Toggle,
};

use super::{General, Instance, PeriodicMode, TimerClock};

/// Interrupts for RTC
#[derive(ufmt::derive::uDebug, Debug)]
#[cfg_attr(feature = "enumset", derive(EnumSetType))]
#[cfg_attr(not(feature = "enumset"), derive(Copy, Clone, PartialEq, Eq))]
pub enum Interrupt {
    CompareMatch,
    Overflow,
}

/// Status events for RTC
#[derive(ufmt::derive::uDebug, Debug)]
#[cfg_attr(feature = "enumset", derive(EnumSetType))]
#[cfg_attr(not(feature = "enumset"), derive(Copy, Clone, PartialEq, Eq))]
pub enum Event {
    CompareMatch,
    Overflow,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub enum RTCClockSource {
    OSCULP32K_32K,
    OSCULP32K_1K,
    //XOSC32K,          // FIXME: retrieve an object for this from CLKCTRL and enable it when doing so
    TOSC1(Hertz),
}

impl Instance for Rtc {}

impl TimerClock for Rtc {
    type ClockSource = RTCClockSource;

    #[inline(always)]
    fn get_input_clock_rate(clk: Self::ClockSource) -> Hertz {
        match clk {
            RTCClockSource::OSCULP32K_32K => 32_768.Hz(),
            RTCClockSource::OSCULP32K_1K => 1_024.Hz(),
            //RTCClockSource::XOSC32K => 32_768.Hz(),
            RTCClockSource::TOSC1(h) => h,
        }
    }

    #[inline(always)]
    fn prepare_clock_source(&mut self, clk: Self::ClockSource) {
        match clk {
            RTCClockSource::OSCULP32K_32K => self.clksel().write(|w| w.clksel().int32k()),
            RTCClockSource::OSCULP32K_1K => self.clksel().write(|w| w.clksel().int1k()),
            //RTCClockSource::XOSC32K => self.clksel().write(|w| w.clksel().tosc32k()),
            RTCClockSource::TOSC1(_) => self.clksel().write(|w| w.clksel().extclk()),
        };
    }

    #[inline(always)]
    fn get_valid_prescalers(_clk: Self::ClockSource) -> &'static [u16] {
        &[
            1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768,
        ]
    }

    #[inline(always)]
    fn set_prescaler(&mut self, psc: u16) {
        while self.status().read().ctrlabusy().bit_is_set() {}
        self.ctrla()
            .modify(|_, w| w.prescaler().variant(into_prescaler(psc)));
    }

    #[inline(always)]
    fn read_prescaler(&self) -> u16 {
        from_prescaler(self.ctrla().read().prescaler().variant())
    }
}

impl General for Rtc {
    const TIMER_WIDTH_BITS: u8 = 16;
    type CounterValue = u16;
    type Interrupt = Interrupt;
    type Event = Event;

    #[inline(always)]
    fn reset_counter_peripheral(&mut self) {}

    #[inline(always)]
    fn enable_counter(&mut self) {
        while self.status().read().ctrlabusy().bit_is_set() {}
        self.ctrla().modify(|_, w| w.rtcen().set_bit());
    }

    #[inline(always)]
    fn disable_counter(&mut self) {
        while self.status().read().ctrlabusy().bit_is_set() {}
        self.ctrla().modify(|_, w| w.rtcen().clear_bit());
    }

    #[inline(always)]
    fn is_counter_enabled(&self) -> bool {
        self.ctrla().read().rtcen().bit_is_set()
    }

    #[inline(always)]
    fn reset_count(&mut self) {
        while self.status().read().cntbusy().bit_is_set() {}
        self.cnt().reset();
    }

    #[inline(always)]
    fn read_count(&self) -> Self::CounterValue {
        self.cnt().read().bits()
    }

    #[inline(always)]
    fn configure_interrupt(&mut self, interrupt: Self::Interrupt, enable: impl Into<Toggle>) {
        let enable: Toggle = enable.into();
        let enable: bool = enable.into();
        match interrupt {
            Interrupt::CompareMatch => self.intctrl().modify(|_, w| w.cmp().bit(enable)),
            Interrupt::Overflow => self.intctrl().modify(|_, w| w.ovf().bit(enable)),
        };
    }

    #[inline(always)]
    fn is_interrupt_configured(&self, interrupt: Self::Interrupt) -> bool {
        let intctrl = self.intctrl().read();
        match interrupt {
            Interrupt::CompareMatch => intctrl.cmp().bit(),
            Interrupt::Overflow => intctrl.ovf().bit(),
        }
    }

    #[inline(always)]
    fn is_event_triggered(&self, event: Self::Event) -> bool {
        let intflags = self.intflags().read();
        match event {
            Event::CompareMatch => intflags.cmp().bit(),
            Event::Overflow => intflags.ovf().bit(),
        }
    }

    #[inline(always)]
    fn clear_event(&mut self, event: Self::Event) {
        match event {
            Event::CompareMatch => self.intflags().modify(|_, w| w.cmp().set_bit()),
            Event::Overflow => self.intflags().modify(|_, w| w.ovf().set_bit()),
        };
    }
}

impl PeriodicMode for Rtc {
    #[inline(always)]
    fn set_periodic_mode(&mut self) {}

    #[inline(always)]
    fn read_period() -> Self::CounterValue {
        // FIXME: function needs to be called from PwmChannel where we don't
        //        have a reference to the Timer, hence this stuff
        //        When the split pwm channels get a ref to the timer, we can
        //        get rid of this again
        let rtc = unsafe { &*Rtc::ptr() };
        rtc.per().read().bits()
    }

    #[inline(always)]
    fn trigger_update(&mut self) {
        // no double buffering, no updating...
    }

    #[inline(always)]
    unsafe fn set_period_unchecked(&mut self, period: Self::CounterValue) {
        while self.status().read().perbusy().bit_is_set() {}
        self.per().write(|w| w.bits(period));
    }

    #[inline(always)]
    fn max_period() -> Self::CounterValue {
        u16::MAX
    }

    #[inline(always)]
    fn clear_overflow(&mut self) {
        self.intflags().modify(|_, w| w.ovf().set_bit());
    }

    #[inline(always)]
    fn get_overflow(&self) -> bool {
        self.intflags().read().ovf().bit_is_set()
    }
}

// FIXME: implement compare mode for RTC
// FIXME: implement PIT in RTC

fn into_prescaler(prescaler: u16) -> ctrla::Prescaler {
    use ctrla::Prescaler::*;
    match prescaler {
        1 => Div1,
        2 => Div2,
        4 => Div4,
        8 => Div8,
        16 => Div16,
        32 => Div32,
        64 => Div64,
        128 => Div128,
        256 => Div256,
        512 => Div512,
        1024 => Div1024,
        2048 => Div2048,
        4096 => Div4096,
        8192 => Div8192,
        16384 => Div16384,
        32768 => Div32768,
        _ => panic!("Invalid prescaler"),
    }
}

fn from_prescaler(prescaler: ctrla::Prescaler) -> u16 {
    use ctrla::Prescaler::*;
    match prescaler {
        Div1 => 1,
        Div2 => 2,
        Div4 => 4,
        Div8 => 8,
        Div16 => 16,
        Div32 => 32,
        Div64 => 64,
        Div128 => 128,
        Div256 => 256,
        Div512 => 512,
        Div1024 => 1024,
        Div2048 => 2048,
        Div4096 => 4096,
        Div8192 => 8192,
        Div16384 => 16384,
        Div32768 => 32768,
    }
}

impl crate::private::Sealed for crate::pac::Rtc {}
