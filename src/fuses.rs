//! # Fuse declaration for downstream firmware
//!
//! [`Fuses`] mirrors the device's FUSE memory layout so a firmware crate can
//! declare its fuse configuration *programmatically* — as a static in a
//! dedicated linker section that the programming tool extracts from the ELF
//! and burns alongside the code:
//!
//! ```ignore
//! #[used]
//! #[link_section = ".fuse"]
//! static FUSES: atxtiny_hal::fuses::Fuses = Fuses {
//!     WDTCFG: 0x00,
//!     BODCFG: 0x00,
//!     OSCCFG: 0x02, // 20 MHz oscillator
//!     reserved_0x03: 0xFF,
//!     TCD0CFG: 0x00,
//!     SYSCFG0: 0xF6,
//!     SYSCFG1: 0x07,
//!     APPEND: 0x00,
//!     BOOTEND: 0x00,
//! };
//! ```
//!
//! This type is intentionally *not* referenced anywhere in the HAL and has no
//! runtime access path: fuse writes are UPDI-only (a running program cannot
//! change them), and runtime reads are served by the PAC's `FUSE` register
//! block at 0x1280.

#[repr(C)]
#[allow(non_snake_case)]
pub struct Fuses {
    pub WDTCFG: u8,
    pub BODCFG: u8,
    pub OSCCFG: u8,
    pub reserved_0x03: u8,
    pub TCD0CFG: u8,
    pub SYSCFG0: u8,
    pub SYSCFG1: u8,
    pub APPEND: u8,
    pub BOOTEND: u8,
}
