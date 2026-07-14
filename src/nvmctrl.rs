//! # Non-Volatile Memory Controller

use core::ptr;

use cfg_if::cfg_if;
use avr_device::ccp::ProtectedWritable;

use crate::pac::{nvmctrl::ctrla::CMD_A, NVMCTRL};

// TODO: SIGROW  = 0x1100
//       FUSES   = 0x1280
//       USERROW = 0x1300
// TODO: Parse BOOTEND and APPEND fuses and offer some API?

cfg_if! {
    if #[cfg(any(
        feature = "attiny417",
    ))] {
        /// Start address of the flash in data space
        pub const FLASH_MAP_START:  usize = 0x8000;

        /// End address of the flash in data space
        pub const FLASH_MAP_END:    usize = 0x8FFF;

        /// Total size of the flash in data space
        pub const FLASH_SIZE:       usize = FLASH_MAP_END - FLASH_MAP_START + 1;

        /// Page size of the flash in data space
        pub const FLASH_PAGE_SIZE:  usize = 64;


        /// Start address of the EEPROM in data space
        pub const EEPROM_MAP_START: usize = 0x1400;

        /// End address of the EEPROM in data space
        pub const EEPROM_MAP_END:   usize = 0x147F;

        /// Page size of the EEPROM in data space
        pub const EEPROM_PAGE_SIZE: usize = 32;


        /// Start address of the USERROW in data space
        pub const USERROW_START:    usize = 0x1300;

        /// End address of the USERROW in data space
        pub const USERROW_END:      usize = 0x131F;

        /// Total size of the USERROW in data space
        pub type const USERROW_SIZE: usize = USERROW_END - USERROW_START + 1;

    } else if #[cfg(any(
        feature = "attiny817",
    ))] {
        /// Start address of the flash in data space
        pub const FLASH_MAP_START:  usize = 0x8000;

        /// End address of the flash in data space
        pub const FLASH_MAP_END:    usize = 0x9FFF;

        /// Total size of the flash in data space
        pub const FLASH_SIZE:       usize = FLASH_MAP_END - FLASH_MAP_START + 1;

        /// Page size of the flash in data space
        pub const FLASH_PAGE_SIZE:  usize = 64;


        /// Start address of the EEPROM in data space
        pub const EEPROM_MAP_START: usize = 0x1400;

        /// End address of the EEPROM in data space
        pub const EEPROM_MAP_END:   usize = 0x147F;

        /// Page size of the EEPROM in data space
        pub const EEPROM_PAGE_SIZE: usize = 32;


        /// Start address of the USERROW in data space
        pub const USERROW_START:    usize = 0x1300;

        /// End address of the USERROW in data space
        pub const USERROW_END:      usize = 0x131F;

        /// Total size of the USERROW in data space
        pub type const USERROW_SIZE: usize = USERROW_END - USERROW_START + 1;

    } else if #[cfg(any(
        feature = "attiny1617",
    ))] {
        /// Start address of the flash in data space
        pub const FLASH_MAP_START:  usize = 0x8000;

        /// End address of the flash in data space
        pub const FLASH_MAP_END:    usize = 0xBFFF;

        /// Total size of the flash in data space
        pub const FLASH_SIZE:       usize = FLASH_MAP_END - FLASH_MAP_START + 1;

        /// Page size of the flash in data space
        pub const FLASH_PAGE_SIZE:  usize = 64;


        /// Start address of the EEPROM in data space
        pub const EEPROM_MAP_START: usize = 0x1400;

        /// End address of the EEPROM in data space
        pub const EEPROM_MAP_END:   usize = 0x14FF;

        /// Page size of the EEPROM in data space
        pub const EEPROM_PAGE_SIZE: usize = 32;


        /// Start address of the USERROW in data space
        pub const USERROW_START:    usize = 0x1300;

        /// End address of the USERROW in data space
        pub const USERROW_END:      usize = 0x131F;

        /// Total size of the USERROW in data space
        pub type const USERROW_SIZE: usize = USERROW_END - USERROW_START + 1;

    } else if #[cfg(any(
        feature = "attiny3217",
    ))] {
        /// Start address of the flash in data space
        pub const FLASH_MAP_START:  usize = 0x8000;

        /// End address of the flash in data space
        pub const FLASH_MAP_END:    usize = 0xFFFF;

        /// Total size of the flash in data space
        pub const FLASH_SIZE:       usize = FLASH_MAP_END - FLASH_MAP_START + 1;

        /// Page size of the flash in data space
        pub const FLASH_PAGE_SIZE:  usize = 128;


        /// Start address of the EEPROM in data space
        pub const EEPROM_MAP_START: usize = 0x1400;

        /// End address of the EEPROM in data space
        pub const EEPROM_MAP_END:   usize = 0x14FF;

        /// Page size of the EEPROM in data space
        pub const EEPROM_PAGE_SIZE: usize = 64;


        /// Start address of the USERROW in data space
        pub const USERROW_START:    usize = 0x1300;

        /// End address of the USERROW in data space
        pub const USERROW_END:      usize = 0x131F;

        /// Total size of the USERROW in data space
        pub type const USERROW_SIZE: usize = USERROW_END - USERROW_START + 1;
    }
}

impl crate::private::Sealed for NVMCTRL {}

pub trait NvmctrlExt: crate::private::Sealed {
    /// Create a [`FlashAccess`] instance that allows to read and write program flash pages
    fn flash(&self) -> FlashAccess<'_>;

    /// Create a [`EepromAccess`] instance that allows to read and write EEPROM pages
    fn eeprom(&self) -> EepromAccess<'_>;

    /// Create a [`UserrowAccess`] instance that allows to read and write USERROW pages
    fn userrow(&self) -> UserrowAccess<'_>;
}

impl NvmctrlExt for NVMCTRL {
    /// Get access to the Flash of the microcontroller for reading and writing
    fn flash(&self) -> FlashAccess<'_> {
        FlashAccess { nvmctrl: self }
    }

    /// Get access to the EEPROM of the microcontroller for reading and writing
    fn eeprom(&self) -> EepromAccess<'_> {
        EepromAccess { nvmctrl: self }
    }

    /// Get access to the USERROW of the microcontroller for reading and writing
    fn userrow(&self) -> UserrowAccess<'_>  {
        UserrowAccess { nvmctrl: self }
    }
}

/// Errors that can occur when reading or writing to Flash or EEPROM
#[derive(ufmt::derive::uDebug, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The hardware returned a write error condition.
    Write,

    /// The supplied offset and length would cause an out of bounds access when
    /// reading or writing Flash or EEPROM.
    OutOfBounds,
}

/// The flash access module which allows reading from and writing to flash
pub struct FlashAccess<'a> {
    nvmctrl: &'a NVMCTRL,
}

impl FlashAccess<'_> {
    /// Erase and write flash.
    ///
    /// When calling this method, the flash is erased page-wise starting from
    /// `offset` and the data in the `bytes` slice is written to it afterwards.
    ///
    /// Non-page-aligned write accesses are handled automatically.
    ///
    /// Returns an [`Error::OutOfBounds`] in case data outside of the flash
    /// region defined by [`FLASH_MAP_START`] and [`FLASH_MAP_END`] is accessed.
    /// In case of a hardware write error [`Error::Write`] is returned.
    pub fn program(&self, offset: usize, bytes: &[u8]) -> Result<(), Error> {
        if FLASH_MAP_START + offset + bytes.len() - 1 > FLASH_MAP_END {
            return Err(Error::OutOfBounds);
        }

        let mut ptr = ((FLASH_MAP_START + offset) & !(FLASH_PAGE_SIZE - 1)) as *mut u8;

        // Clear the page buffer
        self.nvmctrl_cmd(CMD_A::PBC)?;

        // Fill the page buffer with original data that should not be overwritten
        let start_offset = offset % FLASH_PAGE_SIZE;
        for _ in 0..start_offset {
            unsafe {
                ptr::write_volatile(ptr, ptr::read_volatile(ptr));
                ptr = ptr.add(1);
            };
        }

        // Write the new data into the page buffer
        for b in bytes.iter() {
            unsafe {
                ptr::write_volatile(ptr, *b);
                ptr = ptr.add(1);

                if ptr as usize % FLASH_PAGE_SIZE == 0 {
                    self.nvmctrl_cmd(CMD_A::ERWP)?;
                }
            };
        }

        // Write the remainder of the page into the page buffer
        if (ptr as usize) % FLASH_PAGE_SIZE > 0 {
            while (ptr as usize) % FLASH_PAGE_SIZE != 0 {
                unsafe {
                    ptr::write_volatile(ptr, ptr::read_volatile(ptr));
                    ptr = ptr.add(1);
                }
            }

            self.nvmctrl_cmd(CMD_A::ERWP)?;
        }

        Ok(())
    }

    /// Erase a flash page.
    ///
    /// This method erases a flash page starting from `offset`.
    /// The offset does not need to be page-aligned.
    /// 
    /// Returns an [`Error::OutOfBounds`] in case the offset is outside of the flash
    /// region defined by [`FLASH_MAP_START`] and [`FLASH_MAP_END`].
    pub fn erase_page(&self, offset: usize) -> Result<(), Error> {
        if FLASH_MAP_START + offset + FLASH_PAGE_SIZE - 1 > FLASH_MAP_END {
            return Err(Error::OutOfBounds);
        }

        let ptr = (FLASH_MAP_START + offset) as *mut u8;
        unsafe { ptr::write_volatile(ptr, 0xFF) };

        self.nvmctrl_cmd(CMD_A::ER)?;

        Ok(())
    }

    /// Read from flash.
    ///
    /// Returns a slice that gives raw access to the data stored in flash
    /// starting from `offset` with length `len`.
    ///
    /// Returns an [`Error::OutOfBounds`] in case data outside of the flash
    /// region defined by [`FLASH_MAP_START`] and [`FLASH_MAP_END`] is accessed.
    pub fn read(&self, offset: usize, len: usize) -> Result<&[u8], Error> {
        if FLASH_MAP_START + offset + len - 1 > FLASH_MAP_END {
            return Err(Error::OutOfBounds);
        }

        let ptr = (FLASH_MAP_START + offset) as *mut u8;
        Ok(unsafe { core::slice::from_raw_parts(ptr, len) })
    }

    fn nvmctrl_cmd(&self, cmd: CMD_A) -> Result<(), Error> {
        self.nvmctrl
            .ctrla()
            .write_protected(|w| w.cmd().variant(cmd));

        while self.nvmctrl.status().read().fbusy().bit_is_set() {}

        if self.nvmctrl.status().read().wrerror().bit_is_set() {
            return Err(Error::Write);
        }

        Ok(())
    }

    /// Create a writer for incremental flash programming
    ///
    /// This allows writing data in chunks without erasing/writing on every call.
    /// Pages are only committed when full or when explicitly flushed.
    pub fn writer(&self) -> FlashWriter<'_> {
        FlashWriter {
            flash: self,
            current_page_start: None,
            next_write_addr: 0,
        }
    }
}

/// State tracker for incremental flash writing
pub struct FlashWriter<'a> {
    flash: &'a FlashAccess<'a>,
    current_page_start: Option<usize>,
    next_write_addr: usize,
}

impl FlashWriter<'_> {
    /// Write a chunk of data to flash
    ///
    /// This method accumulates data in the page buffer and only commits to flash
    /// when a page boundary is crossed. Call `flush()` at the end to ensure
    /// the last partial page is written.
    pub fn write_chunk(&mut self, offset: usize, bytes: &[u8]) -> Result<bool, Error> {
        if FLASH_MAP_START + offset + bytes.len() - 1 > FLASH_MAP_END {
            return Err(Error::OutOfBounds);
        }

        let write_addr = FLASH_MAP_START + offset;
        let page_start = write_addr & !(FLASH_PAGE_SIZE - 1);
        let mut page_committed = false;

        // Check if we're starting a new page
        if self.current_page_start != Some(page_start) {
            // Flush previous page if it exists and has data
            if let Some(prev_page) = self.current_page_start {
                if self.next_write_addr > prev_page {
                    self.flush_current_page()?;
                    page_committed = true;
                }
            }

            // Initialize new page
            self.current_page_start = Some(page_start);
            self.next_write_addr = write_addr;

            // Clear page buffer and load existing data
            self.flash.nvmctrl_cmd(CMD_A::PBC)?;

            // Fill page buffer with existing flash content
            let page_end = page_start + FLASH_PAGE_SIZE;
            for addr in page_start..page_end {
                unsafe {
                    let existing_data = ptr::read_volatile(addr as *const u8);
                    ptr::write_volatile(addr as *mut u8, existing_data);
                }
            }
        }

        // Verify we're writing sequentially within the current page
        if write_addr != self.next_write_addr {
            return Err(Error::OutOfBounds); // Non-sequential writes not supported
        }

        // Write data to page buffer
        let mut ptr = write_addr as *mut u8;
        for &byte in bytes {
            unsafe {
                ptr::write_volatile(ptr, byte);
                ptr = ptr.add(1);
            }
            self.next_write_addr += 1;

            // Check if we've filled the current page
            if self.next_write_addr % FLASH_PAGE_SIZE == 0 {
                self.flash.nvmctrl_cmd(CMD_A::ERWP)?;
                page_committed = true;
                self.current_page_start = None; // Page is complete
                break;
            }
        }

        Ok(page_committed)
    }

    /// Flush any remaining data in the page buffer to flash
    pub fn flush(&mut self) -> Result<(), Error> {
        if let Some(page_start) = self.current_page_start {
            if self.next_write_addr > page_start {
                self.flush_current_page()?;
                self.current_page_start = None;
            }
        }
        Ok(())
    }

    fn flush_current_page(&self) -> Result<(), Error> {
        self.flash.nvmctrl_cmd(CMD_A::ERWP)
    }
}

/// The EEPROM access module which allows reading from and writing to EEPROM
pub struct EepromAccess<'a> {
    nvmctrl: &'a NVMCTRL,
}

impl EepromAccess<'_> {
    /// Erase and write EEPROM.
    ///
    /// When calling this method, the EEPROM is erased byte-wise starting from
    /// `offset` and the data in the `bytes` slice is written to it afterwards.
    ///
    /// Returns an [`Error::OutOfBounds`] in case data outside of the flash
    /// region defined by [`FLASH_MAP_START`] and [`FLASH_MAP_END`] is accessed.
    /// In case of a hardware write error [`Error::Write`] is returned.
    pub fn program(&self, offset: usize, bytes: &[u8]) -> Result<(), Error> {
        if EEPROM_MAP_START + offset + bytes.len() - 1 > EEPROM_MAP_END {
            return Err(Error::OutOfBounds);
        }

        let mut ptr = (EEPROM_MAP_START + offset) as *mut u8;

        // Clear the page buffer
        self.nvmctrl_cmd(CMD_A::PBC)?;

        // Write the new data into the page buffer and flush it
        // to the EEPROM when reaching a page boundary
        for b in bytes.iter() {
            unsafe {
                ptr::write_volatile(ptr, *b);
                ptr = ptr.add(1);

                if ptr as usize % EEPROM_PAGE_SIZE == 0 {
                    self.nvmctrl_cmd(CMD_A::ERWP)?;
                }
            };
        }

        // Write the remaining bytes from the page buffer into the EEPROM
        if (ptr as usize) % FLASH_PAGE_SIZE > 0 {
            self.nvmctrl_cmd(CMD_A::ERWP)?;
        }

        Ok(())
    }

    /// Read from EEPROM.
    ///
    /// Returns a slice that gives raw access to the data stored in EEPROM
    /// starting from `offset` with length `len`.
    ///
    /// Returns an [`Error::OutOfBounds`] in case data outside of the flash
    /// region defined by [`FLASH_MAP_START`] and [`FLASH_MAP_END`] is accessed.
    pub fn read(&self, offset: usize, len: usize) -> Result<&[u8], Error> {
        if EEPROM_MAP_START + offset + len - 1 > EEPROM_MAP_END {
            return Err(Error::OutOfBounds);
        }

        let ptr = (EEPROM_MAP_START + offset) as *mut u8;
        Ok(unsafe { core::slice::from_raw_parts(ptr, len) })
    }

    fn nvmctrl_cmd(&self, cmd: CMD_A) -> Result<(), Error> {
        self.nvmctrl
            .ctrla()
            .write_protected(|w| w.cmd().variant(cmd));

        while self.nvmctrl.status().read().eebusy().bit_is_set() {}

        if self.nvmctrl.status().read().wrerror().bit_is_set() {
            return Err(Error::Write);
        }

        Ok(())
    }
}

/// The USERROW access module which allows reading from and writing to USERROW
pub struct UserrowAccess<'a> {
    nvmctrl: &'a NVMCTRL,
}

impl UserrowAccess<'_> {
    /// Write to USERROW.
    ///
    /// The USERROW is written byte-wise starting from `offset`.
    /// Since the page buffer is 32 bytes (same as USERROW size), 
    /// no intermediate flushing is needed for sequential writes.
    ///
    /// Returns an [`Error::OutOfBounds`] in case data outside of the USERROW
    /// region is accessed. In case of a hardware write error [`Error::Write`] is returned.
    pub fn program(&self, offset: usize, bytes: &[u8]) -> Result<(), Error> {
        if USERROW_START + offset + bytes.len() - 1 > USERROW_END {
            return Err(Error::OutOfBounds);
        }

        let mut ptr = (USERROW_START + offset) as *mut u8;

        // Clear the page buffer
        self.nvmctrl_cmd(CMD_A::PBC)?;

        // Write the new data into the page buffer
        for b in bytes.iter() {
            unsafe {
                ptr::write_volatile(ptr, *b);
                ptr = ptr.add(1);
            }
        }

        // Flush the page buffer to USERROW
        self.nvmctrl_cmd(CMD_A::ERWP)?;

        Ok(())
    }

    /// Write a single byte to USERROW.
    ///
    /// This is a convenience function for single-byte writes to save program space.
    /// For multiple bytes, use [`program`] for better efficiency.
    pub fn write_byte(&self, offset: usize, byte: u8) -> Result<(), Error> {
        if USERROW_START + offset > USERROW_END {
            return Err(Error::OutOfBounds);
        }

        let ptr = (USERROW_START + offset) as *mut u8;

        // Clear the page buffer
        self.nvmctrl_cmd(CMD_A::PBC)?;

        // Write the single byte
        unsafe {
            ptr::write_volatile(ptr, byte);
        }

        // Flush to USERROW
        self.nvmctrl_cmd(CMD_A::ERWP)?;

        Ok(())
    }

    /// Read from USERROW.
    ///
    /// Returns a slice that gives raw access to the data stored in USERROW
    /// starting from `offset` with length `len`.
    ///
    /// Returns an [`Error::OutOfBounds`] in case data outside of the USERROW
    /// region is accessed.
    pub fn read(&self, offset: usize, len: usize) -> Result<&[u8], Error> {
        if USERROW_START + offset + len - 1 > USERROW_END {
            return Err(Error::OutOfBounds);
        }

        let ptr = (USERROW_START + offset) as *mut u8;
        Ok(unsafe { core::slice::from_raw_parts(ptr, len) })
    }

    /// Read a single byte from USERROW.
    ///
    /// This is a convenience function for single-byte reads to save program space.
    /// For multiple bytes, use [`read`] for better efficiency.
    pub fn read_byte(&self, offset: usize) -> Result<u8, Error> {
        if USERROW_START + offset > USERROW_END {
            return Err(Error::OutOfBounds);
        }

        let ptr = (USERROW_START + offset) as *const u8;
        Ok(unsafe { ptr::read_volatile(ptr) })
    }

    /// Read the entire USERROW as a 32-byte array.
    ///
    /// This is a convenience function to read all USERROW data at once.
    pub fn read_all(&self) -> [u8; USERROW_SIZE] {
        let mut data = [0u8; USERROW_SIZE];
        let ptr = USERROW_START as *const u8;
        
        for i in 0..USERROW_SIZE {
            data[i] = unsafe { ptr::read_volatile(ptr.add(i)) };
        }
        
        data
    }

    /// Write the entire USERROW from a 32-byte array.
    ///
    /// This is a convenience function to write all USERROW data at once.
    pub fn program_all(&self, data: &[u8; USERROW_SIZE]) -> Result<(), Error> {
        self.program(0, data)
    }

    fn nvmctrl_cmd(&self, cmd: CMD_A) -> Result<(), Error> {
        self.nvmctrl
            .ctrla()
            .write_protected(|w| w.cmd().variant(cmd));

        while self.nvmctrl.status().read().eebusy().bit_is_set() {}

        if self.nvmctrl.status().read().wrerror().bit_is_set() {
            return Err(Error::Write);
        }

        Ok(())
    }
}
