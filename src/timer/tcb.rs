//! # 16-bit Timer/Counter Type B

#[cfg(feature = "enumset")]
use enumset::EnumSetType;

use crate::pac::{TCA0, TCB0};
use crate::{clkctrl::Clocks, pac::tcb0::ctrla, time::*, Toggle};

use super::tcb_8bit::TCB8Bit;

/// Interrupts for TCB
#[derive(ufmt::derive::uDebug, Debug)]
#[cfg_attr(feature = "enumset", derive(EnumSetType))]
#[cfg_attr(not(feature = "enumset"), derive(Copy, Clone, PartialEq, Eq))]
pub enum Interrupt {
    CaptureCompare,
}

/// Status events for TCB
#[derive(ufmt::derive::uDebug, Debug)]
#[cfg_attr(feature = "enumset", derive(EnumSetType))]
#[cfg_attr(not(feature = "enumset"), derive(Copy, Clone, PartialEq, Eq))]
pub enum Event {
    CaptureCompare,
}

#[derive(Clone, Copy)]
pub enum TCBClockSource {
    Peripheral(Clocks),
    TCA(Hertz),
}

impl ufmt::uDebug for TCBClockSource {
    fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
    where
        W: ufmt::uWrite + ?Sized,
    {
        match self {
            TCBClockSource::Peripheral(_) => f.write_str("CLK_PER"),
            TCBClockSource::TCA(c) => f.debug_struct("CLK_TCA")?.field("Rate", &c.raw())?.finish(),
        }
    }
}

impl From<Clocks> for TCBClockSource {
    fn from(clocks: Clocks) -> Self {
        TCBClockSource::Peripheral(clocks)
    }
}

pub trait Tcb8bitPwmCapable: super::Instance + super::TimerClock {
    fn into_8bit_pwm(self) -> TCB8Bit;
}

impl super::Instance for TCB0 {}
impl Tcb8bitPwmCapable for TCB0 {
    fn into_8bit_pwm(self) -> TCB8Bit {
        TCB8Bit { tim: self }
    }
}

impl super::TimerClock for TCB0 {
    type ClockSource = TCBClockSource;

    #[inline(always)]
    fn get_input_clock_rate(clk: Self::ClockSource) -> Hertz {
        match clk {
            TCBClockSource::Peripheral(clocks) => clocks.per(),
            TCBClockSource::TCA(clk) => clk,
        }
    }

    #[inline(always)]
    fn prepare_clock_source(&mut self, clk: Self::ClockSource) {
        match clk {
            TCBClockSource::Peripheral(_) => {}
            TCBClockSource::TCA(_) => _ = self.ctrla().modify(|_, w| w.clksel().clktca()),
        }
    }

    #[inline(always)]
    fn get_valid_prescalers(clk: Self::ClockSource) -> &'static [u16] {
        match clk {
            TCBClockSource::Peripheral(_) => &[1, 2],
            TCBClockSource::TCA(_) => &[1],
        }
    }

    #[inline(always)]
    fn set_prescaler(&mut self, psc: u16) {
        if !self.ctrla().read().clksel().is_clktca() {
            self.ctrla()
                .modify(|_, w| w.clksel().variant(into_clksrc(psc)));
        }
    }

    #[inline(always)]
    fn read_prescaler(&self) -> u16 {
        use ctrla::CLKSEL_A::*;
        let prescaler = self.ctrla().read().clksel().variant().unwrap();
        match prescaler {
            CLKTCA => 1,
            _ => from_clksrc(prescaler),
        }
    }
}

impl super::AsClockSource for TCA0 {
    type OutputClock = TCBClockSource;

    #[inline(always)]
    fn use_as_clock_source(&self, timer_clk: Hertz) -> Self::OutputClock {
        TCBClockSource::TCA(timer_clk)
    }
}

impl super::General for TCB0 {
    const TIMER_WIDTH_BITS: u8 = 16;
    type CounterValue = u16;
    type Interrupt = Interrupt;
    type Event = Event;

    #[inline(always)]
    fn reset_counter_peripheral(&mut self) {}

    #[inline(always)]
    fn enable_counter(&mut self) {
        self.ctrla().modify(|_, w| w.enable().set_bit());
    }

    #[inline(always)]
    fn disable_counter(&mut self) {
        self.ctrla().modify(|_, w| w.enable().clear_bit());
    }

    #[inline(always)]
    fn is_counter_enabled(&self) -> bool {
        self.ctrla().read().enable().bit_is_set()
    }

    #[inline(always)]
    fn reset_count(&mut self) {
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
            Interrupt::CaptureCompare => _ = self.intctrl().modify(|_, w| w.capt().bit(enable)),
        }
    }

    #[inline(always)]
    fn is_interrupt_configured(&self, interrupt: Self::Interrupt) -> bool {
        let intctrl = self.intctrl().read();
        match interrupt {
            Interrupt::CaptureCompare => intctrl.capt().bit(),
        }
    }

    #[inline(always)]
    fn is_event_triggered(&self, event: Self::Event) -> bool {
        let intflags = self.intflags().read();
        match event {
            Event::CaptureCompare => intflags.capt().bit(),
        }
    }

    #[inline(always)]
    fn clear_event(&mut self, event: Self::Event) {
        match event {
            Event::CaptureCompare => _ = self.intflags().modify(|_, w| w.capt().set_bit()),
        }
    }
}

impl super::PeriodicMode for TCB0 {
    #[inline(always)]
    fn set_periodic_mode(&mut self) {
        self.ctrlb().modify(|_, w| w.cntmode().int());
    }

    #[inline(always)]
    fn read_period() -> Self::CounterValue {
        // FIXME: function needs to be called from PwmChannel where we don't
        //        have a reference to the Timer, hence this stuff
        //        When the split pwm channels get a ref to the timer, we can
        //        get rid of this again
        let tim = unsafe { &*TCB0::ptr() };
        tim.ccmp().read().bits()
    }

    #[inline(always)]
    fn trigger_update(&mut self) {
        // no double buffering, no updating...
    }

    #[inline(always)]
    unsafe fn set_period_unchecked(&mut self, period: Self::CounterValue) {
        self.ccmp().write(|w| w.bits(period));
    }

    #[inline(always)]
    fn max_period() -> Self::CounterValue {
        u16::MAX
    }

    #[inline(always)]
    fn clear_overflow(&mut self) {
        self.intflags().modify(|_, w| w.capt().set_bit());
    }

    #[inline(always)]
    fn get_overflow(&self) -> bool {
        self.intflags().read().capt().bit_is_set()
    }
}

fn into_clksrc(prescaler: u16) -> ctrla::CLKSEL_A {
    use ctrla::CLKSEL_A::*;
    match prescaler {
        1 => CLKDIV1,
        2 => CLKDIV2,
        _ => panic!("Invalid prescaler"),
    }
}

fn from_clksrc(prescaler: ctrla::CLKSEL_A) -> u16 {
    use ctrla::CLKSEL_A::*;
    match prescaler {
        CLKDIV1 => 1,
        CLKDIV2 => 2,
        _ => panic!("Invalid prescaler"),
    }
}

impl crate::private::Sealed for crate::pac::TCB0 {}

use super::pwm::{WaveformOutputPinset, C1};
use crate::gpio::{Output, Stateless};
use core::marker::PhantomData;

/// A pin can be marked with this when it can be used as a waveform output pin
pub trait WaveformOutputPin<TCB, const CHAN: u8> {}

/// Pin set for the port multiplexer
pub struct TcbPinset<TIM, WaveformOutput: WaveformOutputPin<TIM, CHAN>, const CHAN: u8> {
    _tim: PhantomData<TIM>,
    output: WaveformOutput,
}

impl<TIM, WaveformOutput, const CHAN: u8> TcbPinset<TIM, WaveformOutput, CHAN>
where
    WaveformOutput: WaveformOutputPin<TIM, CHAN>,
{
    pub(crate) fn new(output: WaveformOutput) -> Self {
        TcbPinset {
            _tim: PhantomData,
            output,
        }
    }

    pub fn free(self) -> WaveformOutput {
        self.output
    }
}

// TCB 8 Bit PWM mode outputs
impl<WaveformOutput: WaveformOutputPin<TCB8Bit, CHAN>, const CHAN: u8>
    WaveformOutputPinset<TCB8Bit, CHAN> for TcbPinset<TCB8Bit, WaveformOutput, CHAN>
{
}

impl WaveformOutputPin<TCB8Bit, C1> for crate::gpio::porta::PA5<Output<Stateless>> {}
impl WaveformOutputPin<TCB8Bit, C1> for crate::gpio::portc::PC0<Output<Stateless>> {}
