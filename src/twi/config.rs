//! TWI clock configuration.

use crate::clkctrl::Clocks;
use crate::time::*;

/// The requested SCL frequency is not reachable at the given peripheral
/// clock.
#[derive(ufmt::derive::uDebug, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidTwiClock;

/// A precomputed MBAUD register value plus Fast-mode-plus selection.
///
/// Like [`serial::BaudRate`](crate::serial::BaudRate), this moves the
/// divider arithmetic into `const` evaluation: assign the result of
/// [`TwiClock::new`] to a `const` and the binary carries only the finished
/// register value, while an unreachable rate fails the build.
///
/// ```
/// use atxtiny_hal::twi::TwiClock;
///
/// // Evaluated by the compiler; an unreachable rate is a compile error.
/// const TWI_CLK: TwiClock = TwiClock::new(20_000_000, 100_000);
/// ```
///
/// [`TwiClock::new`] assumes the maximum bus rise time the I2C
/// specification allows for the requested speed class (1000 ns up to
/// 100 kHz, 300 ns up to 400 kHz, 120 ns up to 1 MHz) - the conservative
/// choice, since assuming a slower rise than the bus actually has only
/// makes SCL slower, never faster. For a measured rise time use
/// [`TwiClock::try_new_with_rise_time`]. Fast-mode-plus drive is selected
/// automatically above 400 kHz.
#[derive(ufmt::derive::uDebug, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TwiClock {
    pub(crate) mbaud: u8,
    pub(crate) fast_mode_plus: bool,
}

impl TwiClock {
    /// Compute the MBAUD register value for a peripheral clock, SCL
    /// frequency, and bus rise time.
    ///
    /// The generated SCL frequency never exceeds the requested one (the
    /// division is rounded toward slower). Returns [`InvalidTwiClock`] when
    /// the combination is unreachable: SCL of zero or above the 1 MHz the
    /// hardware supports, a peripheral clock outside the 20MHz hardware
    /// limit, a rise time above 10 µs, an SCL too fast for the peripheral
    /// clock (MBAUD would be negative), or too slow for the 8-bit register.
    pub const fn try_new_with_rise_time(
        f_per: u32,
        f_scl: u32,
        rise_time_ns: u32,
    ) -> Result<Self, InvalidTwiClock> {
        // Bounds first: they keep every multiplication below inside u32.
        if f_per == 0 || f_per > 20_000_000 {
            return Err(InvalidTwiClock);
        }
        if f_scl == 0 || f_scl > 1_000_000 || rise_time_ns > 10_000 {
            return Err(InvalidTwiClock);
        }

        // Datasheet: f_SCL = f_per / (10 + 2*MBAUD + f_per*t_rise), so
        //   MBAUD = (f_per/f_SCL - 10 - f_per*t_rise) / 2.
        //
        // The rise-time term is the number of peripheral clock cycles that
        // fit into t_rise. Computing it as f_per * t_rise / 1e9 would
        // overflow u32 for every clock above ~4.3 MHz, so factor the
        // division: f_per/1000 <= 20_000 times t_rise <= 10_000 stays
        // below 2*10^8. The error from truncating f_per/1000 is under one
        // cycle - irrelevant next to the rise-time estimate itself.
        let rise_cycles = (f_per / 1000) * rise_time_ns / 1_000_000;

        // Round the cycles-per-SCL-period budget up so the resulting SCL
        // is never faster than requested.
        let cycles = (f_per + f_scl - 1) / f_scl;

        // A budget below 10 + rise means the requested SCL is faster than
        // the peripheral clock can generate at all.
        let Some(budget) = cycles.checked_sub(10 + rise_cycles) else {
            return Err(InvalidTwiClock);
        };

        // Round MBAUD up: again err toward a slower SCL.
        let mbaud = budget.div_ceil(2);
        if mbaud > 255 {
            // Slower than the 8-bit register can divide down to.
            return Err(InvalidTwiClock);
        }

        Ok(TwiClock {
            mbaud: mbaud as u8,
            // The datasheet specifies Fast-mode-plus drive for the
            // 400 kHz..1 MHz range.
            fast_mode_plus: f_scl > 400_000,
        })
    }

    /// Like [`TwiClock::try_new_with_rise_time`], with the maximum rise
    /// time the I2C specification allows for the requested speed class.
    pub const fn try_new(f_per: u32, f_scl: u32) -> Result<Self, InvalidTwiClock> {
        // Maximum rise times per I2C speed class:
        //   Standard-mode  (<= 100 kHz): 1000 ns
        //   Fast-mode      (<= 400 kHz):  300 ns
        //   Fast-mode-plus (<=   1 MHz):  120 ns
        let rise_time_ns = if f_scl <= 100_000 {
            1000
        } else if f_scl <= 400_000 {
            300
        } else {
            120
        };

        Self::try_new_with_rise_time(f_per, f_scl, rise_time_ns)
    }

    /// Like [`TwiClock::try_new`], but panics on an unreachable rate.
    ///
    /// Intended for `const` contexts, where the panic is a *compile error*.
    /// Calling this with runtime values panics at runtime instead - use
    /// [`TwiClock::try_new`] or [`TwiClock::from_clocks`] there.
    pub const fn new(f_per: u32, f_scl: u32) -> Self {
        match Self::try_new(f_per, f_scl) {
            Ok(c) => c,
            Err(_) => panic!("SCL frequency not reachable at this peripheral clock"),
        }
    }

    /// Compute the MBAUD value from the configured [`Clocks`].
    ///
    /// Runtime fallback for rates that are not compile-time constants; this
    /// pulls 32-bit division into flash, prefer a `const` [`TwiClock::new`].
    pub fn from_clocks(clocks: &Clocks, f_scl: Hertz) -> Result<Self, InvalidTwiClock> {
        Self::try_new(clocks.per().raw(), f_scl.raw())
    }
}
