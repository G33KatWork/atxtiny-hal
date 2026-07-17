//! # Basic timer support

mod counter;
mod delay;
mod pwm;
mod timer;

pub use counter::*;
pub use delay::*;
pub use pwm::*;
pub use timer::*;

pub mod rtc;
pub mod tca;
pub mod tcb;
pub mod tcb_8bit;

use crate::time::*;

mod sealed {
    use super::{Error, TimerClock};
    use crate::time::*;
    use crate::Toggle;

    pub trait General {
        const TIMER_WIDTH_BITS: u8;
        type CounterValue: Clone + Copy + Into<u32> + TryFrom<u32>;

        cfg_if::cfg_if! {
            if #[cfg(feature = "enumset")] {
                type Interrupt: enumset::EnumSetType;
                type Event: enumset::EnumSetType;
            } else {
                type Interrupt;
                type Event;
            }
        }

        fn reset_counter_peripheral(&mut self);
        fn enable_counter(&mut self);
        fn disable_counter(&mut self);
        fn is_counter_enabled(&self) -> bool;

        fn reset_count(&mut self);
        fn read_count(&self) -> Self::CounterValue;

        fn configure_interrupt(&mut self, interrupt: Self::Interrupt, enable: impl Into<Toggle>);
        fn is_interrupt_configured(&self, interrupt: Self::Interrupt) -> bool;
        fn is_event_triggered(&self, event: Self::Event) -> bool;
        fn clear_event(&mut self, event: Self::Event);
    }

    pub trait AsClockSource: General {
        type OutputClock;

        fn use_as_clock_source(&self, timer_clk: Hertz) -> Self::OutputClock;
    }

    pub trait PeriodicMode: General {
        /// Whether period updates go through a hardware double buffer.
        ///
        /// On buffered timers (TCA) a new period written while the counter
        /// runs only takes effect at the next UPDATE condition. On
        /// unbuffered timers (TCB, RTC) the write is live: shrinking the
        /// period below the current count makes the counter run through a
        /// full counter-width wrap before matching again, so callers must
        /// stop the counter around the update instead.
        const PERIOD_DOUBLE_BUFFERED: bool;

        fn set_periodic_mode(&mut self);

        #[inline(always)]
        fn set_period(&mut self, period: Self::CounterValue) -> Result<(), Error> {
            let p: u32 = period.into();

            // A register value of 0 is a legitimate one-tick period on all
            // supported timers (TCA/TCB/RTC count from 0 through the period
            // value inclusive), so only the upper bound is checked.
            if p <= Self::max_period().into() {
                Ok(unsafe { self.set_period_unchecked(period) })
            } else {
                Err(Error::ImpossiblePeriod)
            }
        }

        unsafe fn set_period_unchecked(&mut self, period: Self::CounterValue);
        fn read_period() -> Self::CounterValue;
        fn trigger_update(&mut self);
        fn max_period() -> Self::CounterValue;

        fn calculate_period_and_prescaler<C: TimerClock>(
            &self,
            clk: C::ClockSource,
            frequency: Hertz,
        ) -> Result<(Self::CounterValue, u16), Error> {
            // Reject 0 Hz (division by zero) and frequencies above the input
            // clock (zero ticks) up front — both used to underflow/panic in
            // the math below in dev builds.
            let ticks = C::get_input_clock_rate(clk)
                .raw()
                .checked_div(frequency.raw())
                .ok_or(Error::ImpossiblePeriod)?;
            if ticks == 0 {
                return Err(Error::ImpossiblePeriod);
            }

            // Round the division up to the next integer to properly determine the
            // prescaler which is an upper bound
            let prescaler = ticks.div_ceil(1 << Self::TIMER_WIDTH_BITS);

            let prescaler = C::get_valid_prescalers(clk)
                .iter()
                .find(|e| **e as u32 >= prescaler)
                .ok_or(Error::ImpossiblePrescaler)?;
            // Round the period up as well: an integer period that is one tick
            // too long beats a timeout that expires early. The prescaler
            // choice above guarantees ticks <= prescaler << TIMER_WIDTH_BITS,
            // so the rounded-up period still fits the counter width.
            let period = ticks.div_ceil(*prescaler as u32) - 1;

            let period = period.try_into().map_err(|_| Error::ImpossiblePeriod)?;
            Ok((period, *prescaler))
        }

        fn clear_overflow(&mut self);
        fn get_overflow(&self) -> bool;
    }

    // FIXME: maybe split the pwm trait and a compare match trait and implement
    //        both for PWM-capable timers? RTC only has compare match but no PWM
    pub trait WithPwm: General + PeriodicMode {
        const CH_NUMBER: u8;
        type GenerationMode;
        type CompareValue: Clone + Copy + Into<u32> + TryFrom<u32>;

        fn set_pwm_mode(&mut self, mode: Self::GenerationMode);

        // FIXME: passing some channel object wrapping a timer pointer or similar
        //        might be the better solution here. Otherwise we always need to
        //        call ptr() and dereference it all the time in these functions
        fn enable_channel(channel: u8, b: bool);
        fn set_compare_value(channel: u8, value: Self::CompareValue);
        fn read_compare_value(channel: u8) -> Self::CompareValue;

        fn clear_compare_match(channel: u8);
        fn get_compare_match(channel: u8) -> bool;
    }

    // FIXME: we need a working event system for TCB single shot mode, I think
    //pub trait SingleShotMode: General {
    //    fn set_single_shot_mode(&mut self);
    //}
}
pub(crate) use sealed::{AsClockSource, General, PeriodicMode, WithPwm};

/// A trait describing one or multiple clock inputs for a timer
pub trait TimerClock {
    /// An enum type that describes all possible clock sources for this timer
    type ClockSource: Copy;

    /// Get the tick rate of a possible clock source before dividing it
    fn get_input_clock_rate(clk: Self::ClockSource) -> Hertz;

    /// Activate the selected clock source for the timer peripheral
    fn prepare_clock_source(&mut self, clk: Self::ClockSource);

    /// Set a prescaler for the selected clock
    fn set_prescaler(&mut self, psc: u16);

    /// Retrieve the current prescaler for the selected clock
    fn read_prescaler(&self) -> u16;

    /// Retrieve a list of available prescalers for the passed clock source
    fn get_valid_prescalers(clk: Self::ClockSource) -> &'static [u16];

    /// Check whether a prescaler is valid for the passed clock source
    #[inline(always)]
    fn is_prescaler_valid(psc: u16, clk: Self::ClockSource) -> bool {
        Self::get_valid_prescalers(clk).contains(&psc)
    }
}

/// A timer instance
pub trait Instance: TimerClock + General + crate::private::Sealed {}

/// A timer instance with mandotory PWM
pub trait InstanceWithPwm: Instance + WithPwm {}

#[derive(ufmt::derive::uDebug, Debug, Eq, PartialEq, Copy, Clone)]
pub enum Error {
    /// Timer is disabled
    Disabled,
    /// Impossible prescaler
    ImpossiblePrescaler,
    /// Impossible Period
    ImpossiblePeriod,
}

pub trait TimerExt<TIM: Instance>: Sized {
    /// Non-blocking [Counter] with custom fixed precision
    fn counter<const FREQ: u32>(self, clk: TIM::ClockSource) -> Result<Counter<Self, FREQ>, Error>;

    /// Non-blocking [Counter] with fixed precision of 1 ms (1 kHz sampling)
    ///
    /// Can wait from 2 ms to 65 sec for 16-bit timer and from 2 ms to 49 days for 32-bit timer.
    fn counter_ms(self, clk: TIM::ClockSource) -> Result<CounterMs<Self>, Error> {
        self.counter::<1_000>(clk)
    }

    /// Non-blocking [Counter] with fixed precision of 1 μs (1 MHz sampling)
    ///
    /// Can wait from 2 μs to 65 ms for 16-bit timer and from 2 μs to 71 min for 32-bit timer.
    fn counter_us(self, clk: TIM::ClockSource) -> Result<CounterUs<Self>, Error> {
        self.counter::<1_000_000>(clk)
    }

    /// Non-blocking [Counter] with dynamic precision which uses `Hertz` as Duration units
    fn counter_hz(self, clk: TIM::ClockSource) -> CounterHz<Self>
    where
        Self: Instance;

    /// Blocking [Delay] with custom fixed precision
    fn delay<const FREQ: u32>(self, clk: TIM::ClockSource) -> Result<Delay<Self, FREQ>, Error>;
}

impl<TIM: Instance + PeriodicMode> TimerExt<TIM> for TIM {
    fn counter<const FREQ: u32>(self, clk: TIM::ClockSource) -> Result<Counter<Self, FREQ>, Error> {
        Ok(FTimer::new(self, clk)?.counter())
    }

    fn counter_hz(self, clk: TIM::ClockSource) -> CounterHz<Self> {
        Timer::new(self, clk).counter_hz()
    }

    fn delay<const FREQ: u32>(self, clk: TIM::ClockSource) -> Result<Delay<Self, FREQ>, Error> {
        Ok(FTimer::new(self, clk)?.delay())
    }
}
