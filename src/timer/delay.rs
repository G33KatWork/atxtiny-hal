use super::{FTimer, Instance, PeriodicMode};

use core::ops::{Deref, DerefMut};

use fugit::TimerDurationU32;

use crate::embedded_hal::delay::DelayNs;

/// Periodic non-blocking timer that implements the [embedded_hal::delay::DelayNs] trait
pub struct Delay<TIM, const FREQ: u32>(pub(super) FTimer<TIM, FREQ>);

impl<T, const FREQ: u32> Deref for Delay<T, FREQ> {
    type Target = FTimer<T, FREQ>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, const FREQ: u32> DerefMut for Delay<T, FREQ> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// FIXME: implement the delay for OneShot timers like in STM32F4 HAL
impl<TIM: Instance + PeriodicMode, const FREQ: u32> Delay<TIM, FREQ> {
    // Sleep for given time
    pub fn delay(&mut self, time: TimerDurationU32<FREQ>) {
        self.tim.disable_counter();
        self.tim.set_periodic_mode();
        self.tim.clear_overflow();

        // A period register value of N makes the counter elapse N + 1 ticks
        // before overflowing (it counts 0..=N), so the remaining tick budget
        // is accounted in whole chunks of `period + 1`. The old code
        // subtracted only `period`, over-delaying one tick per chunk — and
        // its `ticks().max(1) - 1` start value made one-tick delays return
        // immediately.
        let mut ticks = time.ticks();
        while ticks != 0 {
            let chunk = ticks.min(TIM::max_period().into() + 1);

            unsafe {
                // FIXME: add TimerDurationU16 to fugit, then do everything with 16 bits
                self.tim
                    .set_period_unchecked((chunk - 1).try_into().unwrap_or(TIM::max_period()));
            }

            ticks -= chunk;

            self.tim.reset_count();
            self.tim.enable_counter();
            while !self.tim.get_overflow() { /* wait */ }
            self.tim.disable_counter();
            self.tim.clear_overflow();
        }
    }

    /// Largest duration [`delay`](Self::delay) completes within a single
    /// hardware timer period.
    ///
    /// This is a granularity bound, not a usage limit: `delay` accepts
    /// arbitrary `u32`-tick durations and transparently splits longer ones
    /// into multiple timer periods.
    pub fn max_delay(&self) -> TimerDurationU32<FREQ> {
        // FIXME: add TimerDurationU16 to fugit, then do everything with 16 bits
        TimerDurationU32::from_ticks(TIM::max_period().into())
    }

    /// Releases the TIM peripheral
    pub fn release(mut self) -> FTimer<TIM, FREQ> {
        self.tim.disable_counter();
        self.0
    }
}

impl<TIM: Instance + PeriodicMode, const FREQ: u32> fugit_timer::Delay<FREQ> for Delay<TIM, FREQ> {
    type Error = core::convert::Infallible;

    fn delay(&mut self, duration: TimerDurationU32<FREQ>) -> Result<(), Self::Error> {
        self.delay(duration);
        Ok(())
    }
}

impl<TIM: Instance + PeriodicMode, const FREQ: u32> DelayNs for Delay<TIM, FREQ> {
    fn delay_ns(&mut self, ns: u32) {
        // Convert in u64 and round up. fugit's `nanos()` shorthand both
        // truncates toward zero (violating the embedded-hal "at least"
        // contract, e.g. delay_ns(999) at FREQ = 1 MHz returned immediately)
        // and computes `(FREQ / gcd) * ns` in u32, which wraps for FREQ
        // values that do not divide 1e9. `ns * FREQ` always fits u64, and
        // the resulting tick count only exceeds u32 for FREQ > 1 GHz — far
        // beyond anything an AVR timer can clock at.
        let ticks = (u64::from(ns) * u64::from(FREQ)).div_ceil(1_000_000_000) as u32;
        self.delay(TimerDurationU32::from_ticks(ticks));
    }
}
