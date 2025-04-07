//! Types for configuring a serial interface.

use crate::pac::usart0::ctrlc::{Chsize, Pmode, Sbmode};
use crate::time::*;

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

impl From<StopBits> for Sbmode {
    fn from(stopbit: StopBits) -> Self {
        match stopbit {
            StopBits::Stop1 => Sbmode::_1bit,
            StopBits::Stop2 => Sbmode::_2bit,
        }
    }
}

impl From<Sbmode> for StopBits {
    fn from(stopbit: Sbmode) -> Self {
        match stopbit {
            Sbmode::_1bit => StopBits::Stop1,
            Sbmode::_2bit => StopBits::Stop2,
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

impl From<Parity> for Pmode {
    fn from(stopbit: Parity) -> Self {
        match stopbit {
            Parity::None => Pmode::Disabled,
            Parity::Even => Pmode::Even,
            Parity::Odd => Pmode::Odd,
        }
    }
}

impl From<Pmode> for Parity {
    fn from(stopbit: Pmode) -> Self {
        match stopbit {
            Pmode::Disabled => Parity::None,
            Pmode::Even => Parity::Even,
            Pmode::Odd => Parity::Odd,
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

impl From<CharacterSize> for Chsize {
    fn from(chsize: CharacterSize) -> Self {
        match chsize {
            CharacterSize::Size5 => Chsize::_5bit,
            CharacterSize::Size6 => Chsize::_6bit,
            CharacterSize::Size7 => Chsize::_7bit,
            CharacterSize::Size8 => Chsize::_8bit,
        }
    }
}

impl From<Chsize> for CharacterSize {
    fn from(chsize: Chsize) -> Self {
        match chsize {
            Chsize::_5bit => CharacterSize::Size5,
            Chsize::_6bit => CharacterSize::Size6,
            Chsize::_7bit => CharacterSize::Size7,
            Chsize::_8bit => CharacterSize::Size8,
            _ => unimplemented!(),
        }
    }
}

/// Configuration struct for [`Serial`](super::Serial) providing all
/// communication-related / parameters. [`Serial`](super::Serial) always uses eight data
/// bits plus the parity bit - if selected.
///
/// Create a configuration by using `default` in combination with the
/// builder methods. The following snippet shows creating a configuration
/// for 19,200 Baud, 8N1 by deriving it from the default value:
/// ```
/// # use crate::serial::config::*;
/// # use crate::time::Bps;
/// let config = Config::default().baudrate(19_200.bps());
///
/// assert!(config.baudrate == 19_200.bps());
/// assert!(config.parity == Parity::None);
/// assert!(config.stopbits == StopBits::STOP1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    /// Serial interface baud rate
    pub baudrate: Bps,
    /// The number of data bits in a frame
    pub character_size: CharacterSize,
    /// Whether and how to generate/check a parity bit
    pub parity: Parity,
    /// The number of stop bits to follow the last data bit or the parity bit
    pub stopbits: StopBits,
}

impl Config {
    /// Sets the given baudrate.
    pub fn baudrate(mut self, baudrate: Bps) -> Self {
        self.baudrate = baudrate;
        self
    }

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
    /// Creates a new configuration with typically used parameters: 115,200
    /// Baud 8N1.
    fn default() -> Config {
        Config {
            baudrate: 115_200u32.bps(),
            character_size: CharacterSize::Size8,
            parity: Parity::None,
            stopbits: StopBits::Stop1,
        }
    }
}

impl From<Bps> for Config {
    fn from(b: Bps) -> Config {
        Config {
            baudrate: b,
            ..Default::default()
        }
    }
}
