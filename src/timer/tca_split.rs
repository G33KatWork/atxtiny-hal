//! # 16-bit Timer/Counter Type A in split mode
//!
//! Split mode reconfigures TCA0 into two 8-bit timer halves sharing one
//! clock/prescaler: the low half drives WO0-WO2 (LCMP0-2), the high half
//! drives WO3-WO5 (HCMP0-2) — six single-slope PWM outputs in total, at
//! the cost of the 16-bit range and the waveform generation modes.
//!
//! This driver runs the two halves in lockstep with one shared period
//! (every period write goes to both LPER and HPER), which is the
//! many-same-frequency-channels use case split mode exists for.
//! TODO: expose the halves' independent periods (HPER != LPER) for
//!       two-frequency setups.
//!
//! Split-mode caveats compared to the normal (single) mode:
//!
//! * Nothing is double-buffered (there are no PERBUF/CMPnBUF registers):
//!   period and duty writes take effect immediately, so changing them on
//!   a running timer can glitch the current PWM cycle.
//! * The halves count *down* from their period; the compare/duty
//!   semantics are unchanged (duty ticks out of PER + 1).
//! * The high half has no compare interrupt flags in hardware — see
//!   [`WithPwm::get_compare_match`](super::WithPwm) below.

#[cfg(feature = "enumset")]
use enumset::EnumSetType;

use crate::pac::{tca0::split_ctrla, TCA0};
use crate::{clkctrl::Clocks, time::*, Toggle};

/// Interrupts for TCA in split mode
///
/// Each half underflows through its own period, hence two underflow
/// interrupts. Only the low half's compare channels have interrupts.
#[derive(ufmt::derive::uDebug, Debug)]
#[cfg_attr(feature = "enumset", derive(EnumSetType))]
#[cfg_attr(not(feature = "enumset"), derive(Copy, Clone, PartialEq, Eq))]
pub enum Interrupt {
    /// Low half underflow interrupt
    LowUnderflow,

    /// High half underflow interrupt
    HighUnderflow,

    /// Compare match interrupt for low channel 0 (WO0)
    CompareChannel0,

    /// Compare match interrupt for low channel 1 (WO1)
    CompareChannel1,

    /// Compare match interrupt for low channel 2 (WO2)
    CompareChannel2,
}

/// Status events for TCA in split mode
#[derive(ufmt::derive::uDebug, Debug)]
#[cfg_attr(feature = "enumset", derive(EnumSetType))]
#[cfg_attr(not(feature = "enumset"), derive(Copy, Clone, PartialEq, Eq))]
pub enum Event {
    /// Low half underflow interrupt
    LowUnderflow,

    /// High half underflow interrupt
    HighUnderflow,

    /// Compare match interrupt for low channel 0 (WO0)
    CompareChannel0,

    /// Compare match interrupt for low channel 1 (WO1)
    CompareChannel1,

    /// Compare match interrupt for low channel 2 (WO2)
    CompareChannel2,
}

/// TCA0 reconfigured into split mode (CTRLD.SPLITM set).
///
/// Obtained via [`TcaSplitCapable::into_split`]; [`free`](TCASplit::free)
/// returns to the normal 16-bit mode.
pub struct TCASplit {
    tim: TCA0,
}

/// TCA instances that can be reconfigured into split mode.
pub trait TcaSplitCapable: super::Instance + super::TimerClock + Sized {
    fn into_split(self) -> TCASplit;
}

impl TcaSplitCapable for TCA0 {
    fn into_split(self) -> TCASplit {
        // SPLITM may only be changed while the peripheral is disabled.
        self.single_ctrla().modify(|_, w| w.enable().clear_bit());
        self.split_ctrld().modify(|_, w| w.splitm().set_bit());
        TCASplit { tim: self }
    }
}

impl TCASplit {
    /// Leave split mode and hand the peripheral back in its normal
    /// (single, 16-bit) configuration.
    pub fn free(self) -> TCA0 {
        self.tim.split_ctrla().modify(|_, w| w.enable().clear_bit());
        self.tim.split_ctrld().modify(|_, w| w.splitm().clear_bit());
        self.tim
    }
}

impl super::Instance for TCASplit {}
impl crate::private::Sealed for TCASplit {}

impl super::TimerClock for TCASplit {
    type ClockSource = Clocks;

    #[inline(always)]
    fn get_input_clock_rate(clocks: Clocks) -> Hertz {
        clocks.per()
    }

    fn prepare_clock_source(&mut self, _clk: Self::ClockSource) {}

    #[inline(always)]
    fn set_prescaler(&mut self, psc: u16) {
        self.tim
            .split_ctrla()
            .modify(|_, w| w.clksel().variant(into_clksrc(psc)));
    }

    #[inline(always)]
    fn read_prescaler(&self) -> u16 {
        from_clksrc(self.tim.split_ctrla().read().clksel().variant())
    }

    #[inline(always)]
    fn get_valid_prescalers(_clk: Self::ClockSource) -> &'static [u16] {
        &[1, 2, 4, 8, 16, 64, 256, 1024]
    }
}

impl super::General for TCASplit {
    const TIMER_WIDTH_BITS: u8 = 8;
    type CounterValue = u8;
    type Interrupt = Interrupt;
    type Event = Event;

    #[inline(always)]
    fn reset_counter_peripheral(&mut self) {
        // Same disabled-only guarantee as in normal mode. RESET reverts
        // every register of the peripheral — including CTRLD — so split
        // mode has to be re-selected afterwards.
        self.tim.split_ctrla().modify(|_, w| w.enable().clear_bit());
        self.tim.split_ctrleset().write(|w| w.cmd().reset());
        self.tim.split_ctrld().modify(|_, w| w.splitm().set_bit());
    }

    #[inline(always)]
    fn enable_counter(&mut self) {
        self.tim.split_ctrla().modify(|_, w| w.enable().set_bit());
    }

    #[inline(always)]
    fn disable_counter(&mut self) {
        self.tim.split_ctrla().modify(|_, w| w.enable().clear_bit());
    }

    #[inline(always)]
    fn is_counter_enabled(&self) -> bool {
        self.tim.split_ctrla().read().enable().bit_is_set()
    }

    // All split-mode registers are single bytes, so unlike the 16-bit
    // modes there is no shared TEMP register to protect and no need for
    // critical sections around counter/compare accesses.

    #[inline(always)]
    fn reset_count(&mut self) {
        self.tim.split_lcnt().reset();
        self.tim.split_hcnt().reset();
    }

    #[inline(always)]
    fn read_count(&self) -> Self::CounterValue {
        // The halves run in lockstep (shared clock, same period, counters
        // reset together), so the low half's count stands in for both.
        self.tim.split_lcnt().read().bits()
    }

    #[inline(always)]
    fn configure_interrupt(&mut self, interrupt: Self::Interrupt, enable: impl Into<Toggle>) {
        let enable: Toggle = enable.into();
        let enable: bool = enable.into();
        match interrupt {
            Interrupt::LowUnderflow => {
                self.tim.split_intctrl().modify(|_, w| w.lunf().bit(enable))
            }
            Interrupt::HighUnderflow => {
                self.tim.split_intctrl().modify(|_, w| w.hunf().bit(enable))
            }
            Interrupt::CompareChannel0 => self
                .tim
                .split_intctrl()
                .modify(|_, w| w.lcmp0().bit(enable)),
            Interrupt::CompareChannel1 => self
                .tim
                .split_intctrl()
                .modify(|_, w| w.lcmp1().bit(enable)),
            Interrupt::CompareChannel2 => self
                .tim
                .split_intctrl()
                .modify(|_, w| w.lcmp2().bit(enable)),
        };
    }

    #[inline(always)]
    fn is_interrupt_configured(&self, interrupt: Self::Interrupt) -> bool {
        let intctrl = self.tim.split_intctrl().read();
        match interrupt {
            Interrupt::LowUnderflow => intctrl.lunf().bit(),
            Interrupt::HighUnderflow => intctrl.hunf().bit(),
            Interrupt::CompareChannel0 => intctrl.lcmp0().bit(),
            Interrupt::CompareChannel1 => intctrl.lcmp1().bit(),
            Interrupt::CompareChannel2 => intctrl.lcmp2().bit(),
        }
    }

    #[inline(always)]
    fn is_event_triggered(&self, event: Self::Event) -> bool {
        let intflags = self.tim.split_intflags().read();
        match event {
            Event::LowUnderflow => intflags.lunf().bit(),
            Event::HighUnderflow => intflags.hunf().bit(),
            Event::CompareChannel0 => intflags.lcmp0().bit(),
            Event::CompareChannel1 => intflags.lcmp1().bit(),
            Event::CompareChannel2 => intflags.lcmp2().bit(),
        }
    }

    #[inline(always)]
    fn clear_event(&mut self, event: Self::Event) {
        match event {
            Event::LowUnderflow => self.tim.split_intflags().write(|w| w.lunf().set_bit()),
            Event::HighUnderflow => self.tim.split_intflags().write(|w| w.hunf().set_bit()),
            Event::CompareChannel0 => self.tim.split_intflags().write(|w| w.lcmp0().set_bit()),
            Event::CompareChannel1 => self.tim.split_intflags().write(|w| w.lcmp1().set_bit()),
            Event::CompareChannel2 => self.tim.split_intflags().write(|w| w.lcmp2().set_bit()),
        };
    }
}

impl super::PeriodicMode for TCASplit {
    // Split mode has no buffered registers at all; period writes are live.
    const PERIOD_DOUBLE_BUFFERED: bool = false;

    #[inline(always)]
    fn set_periodic_mode(&mut self) {
        // Split mode is inherently periodic (each half counts down from
        // its period register); just make sure it is actually selected.
        self.tim.split_ctrld().modify(|_, w| w.splitm().set_bit());
    }

    #[inline(always)]
    unsafe fn set_period_unchecked(&mut self, period: u8) {
        // Both halves get the same period — the lockstep invariant this
        // driver is built on (see the module docs).
        self.tim.split_lper().write(|w| w.set(period));
        self.tim.split_hper().write(|w| w.set(period));
    }

    #[inline(always)]
    fn read_period() -> Self::CounterValue {
        let tim = unsafe { &*TCA0::ptr() };
        tim.split_lper().read().bits()
    }

    #[inline(always)]
    fn trigger_update(&mut self) {
        // no double buffering, no updating...
    }

    #[inline(always)]
    fn max_period() -> u8 {
        u8::MAX
    }

    #[inline(always)]
    fn clear_overflow(&mut self) {
        self.tim.split_intflags().write(|w| w.lunf().set_bit());
    }

    #[inline(always)]
    fn get_overflow(&self) -> bool {
        self.tim.split_intflags().read().lunf().bit_is_set()
    }
}

impl super::WithPwm for TCASplit {
    const CH_NUMBER: u8 = 6;
    // Split mode is always single-slope 8-bit PWM; there is nothing to
    // choose, unlike the normal mode's waveform generation modes.
    type GenerationMode = ();
    type CompareValue = u8;

    #[inline(always)]
    fn max_compare_value() -> Self::CompareValue {
        u8::MAX
    }

    fn set_pwm_mode(&mut self, _mode: Self::GenerationMode) {
        self.tim.split_ctrld().modify(|_, w| w.splitm().set_bit());
    }

    #[inline(always)]
    fn is_period_driven(_mode: &Self::GenerationMode) -> bool {
        // TOP is always the (shared) period register pair.
        true
    }

    fn enable_channel(channel: u8, b: bool) {
        let tim = unsafe { &*TCA0::ptr() };
        match channel {
            0 => tim.split_ctrlb().modify(|_, w| w.lcmp0en().bit(b)),
            1 => tim.split_ctrlb().modify(|_, w| w.lcmp1en().bit(b)),
            2 => tim.split_ctrlb().modify(|_, w| w.lcmp2en().bit(b)),
            3 => tim.split_ctrlb().modify(|_, w| w.hcmp0en().bit(b)),
            4 => tim.split_ctrlb().modify(|_, w| w.hcmp1en().bit(b)),
            5 => tim.split_ctrlb().modify(|_, w| w.hcmp2en().bit(b)),
            _ => panic!("invalid channel number"),
        };
    }

    fn set_compare_value(channel: u8, value: Self::CompareValue) {
        let tim = unsafe { &*TCA0::ptr() };
        match channel {
            0 => tim.split_lcmp0().write(|w| w.set(value)),
            1 => tim.split_lcmp1().write(|w| w.set(value)),
            2 => tim.split_lcmp2().write(|w| w.set(value)),
            3 => tim.split_hcmp0().write(|w| w.set(value)),
            4 => tim.split_hcmp1().write(|w| w.set(value)),
            5 => tim.split_hcmp2().write(|w| w.set(value)),
            _ => panic!("invalid channel number"),
        };
    }

    fn read_compare_value(channel: u8) -> Self::CompareValue {
        let tim = unsafe { &*TCA0::ptr() };
        match channel {
            0 => tim.split_lcmp0().read().bits(),
            1 => tim.split_lcmp1().read().bits(),
            2 => tim.split_lcmp2().read().bits(),
            3 => tim.split_hcmp0().read().bits(),
            4 => tim.split_hcmp1().read().bits(),
            5 => tim.split_hcmp2().read().bits(),
            _ => panic!("invalid channel number"),
        }
    }

    // The high half has no compare interrupt flags in hardware (split
    // INTFLAGS only provides LUNF/HUNF/LCMP0-2), so for channels 3-5 a
    // compare match is simply not observable: get_compare_match reports
    // false and clear_compare_match does nothing. The channels themselves
    // work fine as PWM outputs.

    #[inline(always)]
    fn clear_compare_match(channel: u8) {
        let tim = unsafe { &*TCA0::ptr() };
        match channel {
            0 => _ = tim.split_intflags().write(|w| w.lcmp0().set_bit()),
            1 => _ = tim.split_intflags().write(|w| w.lcmp1().set_bit()),
            2 => _ = tim.split_intflags().write(|w| w.lcmp2().set_bit()),
            3..=5 => (),
            _ => panic!("invalid channel number"),
        };
    }

    #[inline(always)]
    fn get_compare_match(channel: u8) -> bool {
        let tim = unsafe { &*TCA0::ptr() };
        match channel {
            0 => tim.split_intflags().read().lcmp0().bit_is_set(),
            1 => tim.split_intflags().read().lcmp1().bit_is_set(),
            2 => tim.split_intflags().read().lcmp2().bit_is_set(),
            3..=5 => false,
            _ => panic!("invalid channel number"),
        }
    }
}

// CLKTCA (the TCB clock option derived from TCA's prescaled clock) works
// the same regardless of TCA's mode, so a split-mode TCA can feed TCBs
// exactly like the normal mode does.
impl super::AsClockSource for TCASplit {
    type OutputClock = super::tcb::TCBClockSource;

    #[inline(always)]
    fn use_as_clock_source(&self, timer_clk: Hertz) -> Self::OutputClock {
        super::tcb::TCBClockSource::TCA(timer_clk)
    }
}

fn into_clksrc(prescaler: u16) -> split_ctrla::CLKSEL_A {
    use split_ctrla::CLKSEL_A::*;
    match prescaler {
        1 => DIV1,
        2 => DIV2,
        4 => DIV4,
        8 => DIV8,
        16 => DIV16,
        64 => DIV64,
        256 => DIV256,
        1024 => DIV1024,
        _ => panic!("Invalid prescaler"),
    }
}

fn from_clksrc(prescaler: split_ctrla::CLKSEL_A) -> u16 {
    use split_ctrla::CLKSEL_A::*;
    match prescaler {
        DIV1 => 1,
        DIV2 => 2,
        DIV4 => 4,
        DIV8 => 8,
        DIV16 => 16,
        DIV64 => 64,
        DIV256 => 256,
        DIV1024 => 1024,
    }
}
