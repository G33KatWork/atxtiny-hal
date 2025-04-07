//! # System configuration

use crate::pac::Syscfg;

/// Extension trait for the [`Syscfg`] peripheral
pub trait SyscfgExt {
    /// Return the revision ID of the chip from the [`Syscfg`] peripheral.
    ///
    /// [`pac::Syscfg`]: `crate::pac::Syscfg`
    fn get_revision_id(&self) -> u8;
}

impl SyscfgExt for Syscfg {
    fn get_revision_id(&self) -> u8 {
        self.revid().read().bits()
    }
}
