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
