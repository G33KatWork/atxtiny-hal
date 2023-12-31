//! # Time unit utilities

pub use fugit::{
    HertzU32 as Hertz,
    KilohertzU32 as KiloHertz,
    MegahertzU32 as MegaHertz,
    NanosDurationU32 as NanosDuration,
    ExtU32 as _fugit_DurationExtU32,
    RateExtU32 as _fugit_RateExtU32,
};

/// Bits per second
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Bps(pub u32);

/// Extension trait that adds convenience methods to the `u32` type
pub trait U32Ext {
    /// Wrap in `Bps`
    fn bps(self) -> Bps;
}

impl U32Ext for u32 {
    fn bps(self) -> Bps {
        Bps(self)
    }
}
