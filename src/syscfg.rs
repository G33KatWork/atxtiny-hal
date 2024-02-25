//! # System configuration

use crate::pac::SYSCFG;

/// Extension trait for the [`SYSCFG`] peripheral
pub trait SyscfgExt {
    /// Return the revision ID of the chip from the [`SYSCFG`] peripheral.
    ///
    /// [`pac::SYSCFG`]: `crate::pac::SYSCFG`
    fn get_revision_id(&self) -> u8;
}

impl SyscfgExt for SYSCFG {
    fn get_revision_id(&self) -> u8 {
        self.revid().read().bits()
    }
}
