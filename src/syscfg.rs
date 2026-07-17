//! # System configuration

use crate::pac::SYSCFG;

/// Extension trait that constrains the [`crate::pac::SYSCFG`] peripheral
pub trait SyscfgExt {
    /// Constrains the [`pac::SYSCFG`] peripheral.
    ///
    /// Consumes the [`pac::SYSCFG`] peripheral and converts it to a [`HAL`] internal type
    /// constraining it's public access surface to fit the design of the `HAL`.
    ///
    /// [`pac::SYSCFG`]: `crate::pac::SYSCFG`
    /// [`HAL`]: `crate`
    fn constrain(self) -> Syscfg;
}

/// Constrained Syscfg peripheral
///
/// An instance of this struct is acquired by calling the [`constrain`](SyscfgExt::constrain) function
/// on the [`SYSCFG`] struct.
///
/// ```
/// let dp = pac::Peripherals::take().unwrap();
/// let syscfg = dp.SYSCFG.constrain();
/// ```
pub struct Syscfg {
    syscfg: SYSCFG,
}

impl SyscfgExt for SYSCFG {
    fn constrain(self) -> Syscfg {
        Syscfg { syscfg: self }
    }
}

impl Syscfg {
    /// Return the revision ID of the chip
    pub fn revision_id(&self) -> u8 {
        self.syscfg.revid().read().bits()
    }
}
