//! # Prelude
//!
//! ```rust
//! // Import common extension traits.
//! //
//! // This includes internal extension crates,
//! // but also reexportet traits from embeded-hal or embedded time.
//! use atxtiny_hal::prelude::*;
//! ```

pub use fugit::ExtU32 as _fugit_DurationExtU32;
pub use fugit::RateExtU32 as _fugit_RateExtU32;

pub use crate::clkctrl::{ClkctrlExt as _atxtiny_hal_clkctrl_ClkctrlExt, MainClkSrc};
pub use crate::gpio::GpioExt as _atxtiny_hal_gpio_GpioExt;
pub use crate::nvmctrl::NvmctrlExt as _atxtiny_hal_nvmctrl_NvmctrlExt;
pub use crate::portmux::{IntoMuxedPinset, PortmuxExt as _atxtiny_hal_portmux_PortmuxExt};
pub use crate::watchdog::{WatchdogTimeout, WdtExt as _atxtiny_hal_watchdog_WdtExt};
pub use crate::Toggle;

pub use crate::{
    embedded_hal::digital::InputPin as _embedded_hal_digital_InputPin,
    embedded_hal::digital::OutputPin as _embedded_hal_digital_OutputPin,
    embedded_hal::digital::StatefulOutputPin as _embedded_hal_digital_StatefulOutputPin, time::*,
};
