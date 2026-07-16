use core::fmt;

use crate::clkctrl::Clocks;
use crate::embedded_hal::spi::{self, Mode};
use crate::pac::spi0::ctrla::{DORD_A, PRESC_A};
use crate::time::*;

/// The requested SCK rate cannot be reached without exceeding it.
#[derive(ufmt::derive::uDebug, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidSpiClock;

/// A precomputed SPI clock divider (prescaler plus CLK2X doubler).
///
/// Like [`serial::BaudRate`](crate::serial::BaudRate), this moves the
/// divider arithmetic into `const` evaluation: assign the result of
/// [`SpiClock::new`] to a `const` and the binary carries only the finished
/// register fields, while an unreachable rate fails the build.
///
/// ```
/// use atxtiny_hal::spi::SpiClock;
///
/// // Evaluated by the compiler; an unreachable rate is a compile error.
/// const SPI_CLK: SpiClock = SpiClock::new(20_000_000, 625_000);
/// ```
///
/// The requested rate is a *maximum*: the hardware only divides by powers
/// of two (2..=128), so the selected SCK is the fastest available rate
/// that does not exceed the request. SPI has no minimum-speed requirement,
/// which is why rounding down is always safe - the only unreachable
/// requests are ones slower than `f_per / 128`, where even the largest
/// divider would overshoot.
// NOTE: no uDebug derive - the PAC's PRESC_A field type only implements Debug.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpiClock {
    pub(crate) clk2x: bool,
    pub(crate) presc: PRESC_A,
}

impl SpiClock {
    /// Select the largest SCK rate that does not exceed `max_sck`.
    ///
    /// Returns [`InvalidSpiClock`] when `max_sck` is slower than
    /// `f_per / 128` (nothing the hardware could do without exceeding the
    /// requested maximum), zero, or the peripheral clock is outside the
    /// hardware's 20MHz limit.
    pub const fn try_new(f_per: u32, max_sck: u32) -> Result<Self, InvalidSpiClock> {
        if f_per == 0 || f_per > 20_000_000 || max_sck == 0 {
            return Err(InvalidSpiClock);
        }

        // Fast path doubling as an overflow guard: anything at or above
        // f_per/2 gets the fastest setting (rounding down is fine, faster
        // than /2 does not exist). Below this point max_sck < f_per/2, so
        // the ceiling addition cannot overflow u32.
        if max_sck >= f_per / 2 {
            return Ok(SpiClock {
                clk2x: true,
                presc: PRESC_A::CLK_PER_4_2,
            });
        }

        // Smallest power-of-two division that stays at or below max_sck.
        let ratio = (f_per + max_sck - 1) / max_sck;
        let (clk2x, presc) = if ratio <= 4 {
            (false, PRESC_A::CLK_PER_4_2)
        } else if ratio <= 8 {
            (true, PRESC_A::CLK_PER_16_8)
        } else if ratio <= 16 {
            (false, PRESC_A::CLK_PER_16_8)
        } else if ratio <= 32 {
            (true, PRESC_A::CLK_PER_64_32)
        } else if ratio <= 64 {
            (false, PRESC_A::CLK_PER_64_32)
        } else if ratio <= 128 {
            (false, PRESC_A::CLK_PER_128_64)
        } else {
            // Even /128 would exceed the requested maximum.
            return Err(InvalidSpiClock);
        };

        Ok(SpiClock { clk2x, presc })
    }

    /// Like [`SpiClock::try_new`], but panics on an unreachable rate.
    ///
    /// Intended for `const` contexts, where the panic is a *compile error*.
    /// Calling this with runtime values panics at runtime instead - use
    /// [`SpiClock::try_new`] or [`SpiClock::from_clocks`] there.
    pub const fn new(f_per: u32, max_sck: u32) -> Self {
        match Self::try_new(f_per, max_sck) {
            Ok(c) => c,
            Err(_) => panic!("SCK rate not reachable at this peripheral clock"),
        }
    }

    /// Compute the divider from the configured [`Clocks`].
    ///
    /// Runtime fallback for rates that are not compile-time constants; this
    /// pulls 32-bit division into flash, prefer a `const` [`SpiClock::new`].
    pub fn from_clocks(clocks: &Clocks, max_sck: Hertz) -> Result<Self, InvalidSpiClock> {
        Self::try_new(clocks.per().raw(), max_sck.raw())
    }
}

/// Frame configuration for [`Spi`](super::Spi): operation mode and data
/// order. The bus clock is passed to
/// [`Spi::new_unbuffered`](super::Spi::new_unbuffered) separately as a
/// precomputed [`SpiClock`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Config {
    /// Operation Mode as defined by [`embedded_hal::spi::Mode`]
    pub mode: Mode,
    /// The data order of transmissions
    pub order: DataOrder,
}

#[derive(ufmt::derive::uDebug, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataOrder {
    /// Transmit the most significant bit first
    MsbFirst,
    /// Transmit the least significant bit first
    LsbFirst,
}

impl From<DataOrder> for DORD_A {
    fn from(order: DataOrder) -> Self {
        match order {
            DataOrder::MsbFirst => DORD_A::MSB_FIRST,
            DataOrder::LsbFirst => DORD_A::LSB_FIRST,
        }
    }
}

impl From<DORD_A> for DataOrder {
    fn from(order: DORD_A) -> Self {
        match order {
            DORD_A::MSB_FIRST => DataOrder::MsbFirst,
            DORD_A::LSB_FIRST => DataOrder::LsbFirst,
        }
    }
}

impl Config {
    /// Set the Operation Mode
    pub fn mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    /// Set the data order
    pub fn data_order(mut self, order: DataOrder) -> Self {
        self.order = order;
        self
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mode = if self.mode == spi::MODE_0 {
            0
        } else if self.mode == spi::MODE_1 {
            1
        } else if self.mode == spi::MODE_2 {
            2
        } else {
            3
        };

        f.debug_struct("Config")
            .field("mode", &format_args!("MODE_{}", mode))
            .field("order", &format_args!("{:?}", self.order))
            .finish()
    }
}

impl Default for Config {
    /// Creates a new configuration with the typically used parameters:
    /// MODE_0, MSB first.
    fn default() -> Self {
        Self {
            mode: spi::MODE_0,
            order: DataOrder::MsbFirst,
        }
    }
}
