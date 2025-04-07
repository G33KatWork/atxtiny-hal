#![no_std]
#![feature(asm_experimental_arch)]
#![feature(associated_type_defaults)]
#![feature(associated_const_equality)]
#![deny(rustdoc::broken_intra_doc_links)]

pub use embedded_hal;
pub use embedded_hal_bus;
pub use embedded_hal_nb;
pub use embedded_io;

mod private {
    /// Private sealed trait to seal all GPIO implementations
    /// which do implement peripheral functionalities.
    pub trait Sealed {}
}

pub mod panic_serial;
pub mod prelude;
pub mod time;

pub use avr_device;

#[cfg(feature = "attiny817")]
pub use avr_device::attiny817 as pac;

pub mod ac;
pub mod bod;
pub mod ccl;
pub mod clkctrl;
pub mod cpuint;
pub mod dac;
pub mod evout;
pub mod evsys;
pub mod gpio;
pub mod nvmctrl;
pub mod portmux;
pub mod rstctrl;
pub mod serial;
pub mod slpctrl;
pub mod spi;
pub mod syscfg;
pub mod timer;
pub mod traits;
pub mod twi;
pub mod vref;
pub mod watchdog;

/// Toggle something on or off.
///
/// Convenience enum and wrapper around a bool, which more explicit about the intention to enable
/// or disable something, in comparison to `true` or `false`.
#[derive(ufmt::derive::uDebug, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Toggle {
    /// Toggle something on / enable a thing.
    On,
    /// Toggle something off / disable a thing.
    Off,
}

impl From<Toggle> for bool {
    fn from(toggle: Toggle) -> Self {
        matches!(toggle, Toggle::On)
    }
}

impl From<bool> for Toggle {
    fn from(b: bool) -> Self {
        match b {
            true => Toggle::On,
            false => Toggle::Off,
        }
    }
}
