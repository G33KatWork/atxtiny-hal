//! Types for configuring a serial interface.

use crate::clkctrl::Clocks;
use crate::pac::usart0::ctrlc::{CHSIZE_A, PMODE_A, SBMODE_A};
use crate::time::*;

/// The requested baud rate is not reachable at the given peripheral clock.
#[derive(ufmt::derive::uDebug, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidBaudRate;

/// A precomputed BAUD register value plus receiver mode selection.
///
/// The baud rate arithmetic is 32-bit division, which costs both flash
/// (libgcc helpers) and cycles on an AVR. Because the peripheral clock and
/// the baud rate are compile-time constants in virtually every firmware,
/// this type moves that arithmetic into `const` evaluation: assign the
/// result of [`BaudRate::new`] to a `const` and the binary only ever
/// contains the finished register value. An unreachable rate then fails the
/// *build* instead of the device.
///
/// ```
/// use atxtiny_hal::serial::BaudRate;
///
/// // Evaluated by the compiler; a bad combination is a compile error.
/// const BAUD: BaudRate = BaudRate::new(20_000_000, 115_200);
/// ```
///
/// For rates only known at runtime use [`BaudRate::from_clocks`], which
/// returns an error instead of panicking - and pulls the 32-bit division
/// into flash, which is exactly why it is not the default path.
#[derive(ufmt::derive::uDebug, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaudRate {
    pub(crate) reg: u16,
    pub(crate) clk2x: bool,
}

impl BaudRate {
    /// Compute the BAUD register value for a peripheral clock and baud rate.
    ///
    /// Returns [`InvalidBaudRate`] when the rate is not reachable: faster
    /// than `f_per / 8`, slower than the 16-bit register can divide to,
    /// zero, or a peripheral clock outside the hardware's 20MHz limit.
    pub const fn try_new(f_per: u32, baudrate: u32) -> Result<Self, InvalidBaudRate> {
        // The tinyAVR 0/1-series tops out at 20MHz CLK_PER. Rejecting
        // higher inputs up front keeps every multiplication below safely
        // inside u32.
        if f_per == 0 || f_per > 20_000_000 || baudrate == 0 {
            return Err(InvalidBaudRate);
        }

        // The fractional baud generator requires a register value of at
        // least 64. The fastest reachable rate is therefore f_per/8, in
        // CLK2X mode: 8*f/b >= 64 <=> b <= f/8.
        if baudrate > f_per / 8 {
            return Err(InvalidBaudRate);
        }

        // BAUD = 64*f_per / (S*f_baud), rounded to nearest. S=16 in normal
        // mode (reg = 4f/b), S=8 with the CLK2X doubler (reg = 8f/b).
        // Prefer normal mode - it samples 16x per bit and is more tolerant
        // of clock error - and fall back to CLK2X only when the divisor
        // would drop below the hardware minimum of 64.
        let reg = (4 * f_per + baudrate / 2) / baudrate;
        if reg >= 64 {
            if reg > 0xFFFF {
                // Slower than the 16-bit register can divide down to.
                return Err(InvalidBaudRate);
            }
            Ok(BaudRate {
                reg: reg as u16,
                clk2x: false,
            })
        } else {
            // In range 64..=127 by construction: baudrate <= f_per/8 was
            // checked above (lower bound) and the normal-mode value was
            // below 64 (upper bound).
            let reg = (8 * f_per + baudrate / 2) / baudrate;
            Ok(BaudRate {
                reg: reg as u16,
                clk2x: true,
            })
        }
    }

    /// Like [`BaudRate::try_new`], but panics on an unreachable rate.
    ///
    /// Intended for `const` contexts, where the panic is a *compile error*:
    ///
    /// ```
    /// # use atxtiny_hal::serial::BaudRate;
    /// const BAUD: BaudRate = BaudRate::new(20_000_000, 115_200);
    /// ```
    ///
    /// Calling this with runtime values panics at runtime instead - use
    /// [`BaudRate::try_new`] or [`BaudRate::from_clocks`] there.
    pub const fn new(f_per: u32, baudrate: u32) -> Self {
        match Self::try_new(f_per, baudrate) {
            Ok(b) => b,
            Err(_) => panic!("baud rate not reachable at this peripheral clock"),
        }
    }

    /// Compute the BAUD register value from the configured [`Clocks`].
    ///
    /// Runtime fallback for rates that are not compile-time constants. This
    /// pulls 32-bit division helpers into flash; prefer a
    /// `const` [`BaudRate::new`] whenever the rate is known at build time.
    pub fn from_clocks(clocks: &Clocks, baudrate: Bps) -> Result<Self, InvalidBaudRate> {
        Self::try_new(clocks.per().raw(), baudrate.0)
    }
}

/// Stop Bit configuration parameter for serial.
///
/// Wrapper around [`SBMODE_A`]
#[derive(ufmt::derive::uDebug, Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopBits {
    /// 1 stop bit
    Stop1,
    /// 2 stop bit
    Stop2,
}

impl From<StopBits> for SBMODE_A {
    fn from(stopbit: StopBits) -> Self {
        match stopbit {
            StopBits::Stop1 => SBMODE_A::_1BIT,
            StopBits::Stop2 => SBMODE_A::_2BIT,
        }
    }
}

impl From<SBMODE_A> for StopBits {
    fn from(stopbit: SBMODE_A) -> Self {
        match stopbit {
            SBMODE_A::_1BIT => StopBits::Stop1,
            SBMODE_A::_2BIT => StopBits::Stop2,
        }
    }
}

/// Parity generation and checking. If odd or even parity is selected, the
/// underlying USART will be configured to send/receive the parity bit in
/// addtion to the data bits.
#[derive(ufmt::derive::uDebug, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parity {
    /// No parity bit will be added/checked.
    None,
    /// The MSB transmitted/received will be generated/checked to have a
    /// even number of bits set.
    Even,
    /// The MSB transmitted/received will be generated/checked to have a
    /// odd number of bits set.
    Odd,
}

impl From<Parity> for PMODE_A {
    fn from(stopbit: Parity) -> Self {
        match stopbit {
            Parity::None => PMODE_A::DISABLED,
            Parity::Even => PMODE_A::EVEN,
            Parity::Odd => PMODE_A::ODD,
        }
    }
}

impl From<PMODE_A> for Parity {
    fn from(stopbit: PMODE_A) -> Self {
        match stopbit {
            PMODE_A::DISABLED => Parity::None,
            PMODE_A::EVEN => Parity::Even,
            PMODE_A::ODD => Parity::Odd,
        }
    }
}

/// Character size that the UART hardware sends and receives
#[derive(ufmt::derive::uDebug, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterSize {
    Size5,
    Size6,
    Size7,
    Size8,
    // TODO: Add support
    //Size9_LSB,
    //Size9_MSB,
}

impl From<CharacterSize> for CHSIZE_A {
    fn from(chsize: CharacterSize) -> Self {
        match chsize {
            CharacterSize::Size5 => CHSIZE_A::_5BIT,
            CharacterSize::Size6 => CHSIZE_A::_6BIT,
            CharacterSize::Size7 => CHSIZE_A::_7BIT,
            CharacterSize::Size8 => CHSIZE_A::_8BIT,
        }
    }
}

impl From<CHSIZE_A> for CharacterSize {
    fn from(chsize: CHSIZE_A) -> Self {
        match chsize {
            CHSIZE_A::_5BIT => CharacterSize::Size5,
            CHSIZE_A::_6BIT => CharacterSize::Size6,
            CHSIZE_A::_7BIT => CharacterSize::Size7,
            CHSIZE_A::_8BIT => CharacterSize::Size8,
            _ => unimplemented!(),
        }
    }
}

/// Frame format configuration for [`Serial`](super::Serial): character
/// size, parity and stop bits. The baud rate is passed to
/// [`Serial::new`](super::Serial::new) separately as a precomputed
/// [`BaudRate`].
///
/// Create a configuration by using `default` (8N1) in combination with the
/// builder methods:
/// ```
/// # use atxtiny_hal::serial::config::*;
/// let config = Config::default().parity(Parity::Even);
///
/// assert!(config.parity == Parity::Even);
/// assert!(config.stopbits == StopBits::Stop1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    /// The number of data bits in a frame
    pub character_size: CharacterSize,
    /// Whether and how to generate/check a parity bit
    pub parity: Parity,
    /// The number of stop bits to follow the last data bit or the parity bit
    pub stopbits: StopBits,
}

impl Config {
    /// Sets the given character size.
    pub fn character_size(mut self, character_size: CharacterSize) -> Self {
        self.character_size = character_size;
        self
    }

    /// Sets the given parity.
    pub fn parity(mut self, parity: Parity) -> Self {
        self.parity = parity;
        self
    }

    /// Sets the stop bits to `stopbits`.
    pub fn stopbits(mut self, stopbits: StopBits) -> Self {
        self.stopbits = stopbits;
        self
    }
}

impl Default for Config {
    /// Creates a new configuration with the typically used 8N1 frame format.
    fn default() -> Config {
        Config {
            character_size: CharacterSize::Size8,
            parity: Parity::None,
            stopbits: StopBits::Stop1,
        }
    }
}
