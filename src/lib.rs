#![no_std]
#![feature(asm_experimental_arch)]
#![feature(min_generic_const_args)]
#![deny(rustdoc::broken_intra_doc_links)]

#[cfg(not(feature = "device-selected"))]
compile_error!(
    "This crate requires you to specify your target chip as a feature.

    Please select one of the following:

    tinyAVR 0-series:
    * attiny202
    * attiny204
    * attiny402
    * attiny404
    * attiny804
    * attiny1604
    * attiny1606

    tinyAVR 1-series:
    * attiny212
    * attiny214
    * attiny412
    * attiny414
    * attiny416
    * attiny417
    * attiny816
    * attiny817
    * attiny1614
    * attiny1617
    * attiny3217
    "
);

pub use embedded_hal;
pub use embedded_hal_bus;
pub use embedded_hal_nb;
pub use embedded_io;

mod private {
    /// Private sealed trait used crate-wide to prevent downstream crates
    /// from implementing this HAL's public traits (pin markers, peripheral
    /// extensions, ...) for their own types.
    pub trait Sealed {}
}

pub mod panic_serial;
pub mod prelude;
pub mod time;

pub use avr_device;

pub mod ac;
pub mod bod;
pub mod ccl;
pub mod clkctrl;
pub mod cpuint;
// The DAC only exists on 1-series parts.
#[cfg(feature = "periph-dac0")]
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
pub mod fuses;

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

// One re-export per supported chip; selecting more than one device feature
// yields a (deliberate) duplicate-definition error on `pac`.
macro_rules! pac_for {
    ($($feature:literal => $chip:ident,)+) => {
        $(
            #[cfg(feature = $feature)]
            pub use avr_device::$chip as pac;
        )+
    };
}

pac_for! {
    "attiny202" => attiny202,
    "attiny204" => attiny204,
    "attiny402" => attiny402,
    "attiny404" => attiny404,
    "attiny804" => attiny804,
    "attiny1604" => attiny1604,
    "attiny1606" => attiny1606,
    "attiny212" => attiny212,
    "attiny214" => attiny214,
    "attiny412" => attiny412,
    "attiny414" => attiny414,
    "attiny416" => attiny416,
    "attiny417" => attiny417,
    "attiny816" => attiny816,
    "attiny817" => attiny817,
    "attiny1614" => attiny1614,
    "attiny1617" => attiny1617,
    "attiny3217" => attiny3217,
}
