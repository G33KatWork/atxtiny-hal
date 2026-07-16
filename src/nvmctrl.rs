//! # Non-Volatile Memory Controller

use core::ptr;

use cfg_if::cfg_if;
use avr_device::generic::ProtectedWritable;

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
        pub const USERROW_SIZE:    usize = USERROW_END - USERROW_START + 1;

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
        pub const USERROW_SIZE:    usize = USERROW_END - USERROW_START + 1;

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
        pub const USERROW_SIZE:    usize = USERROW_END - USERROW_START + 1;

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
        ///
        /// The attiny3217 USERROW is 64 bytes, twice the size of the smaller
        /// devices (ATDF: `USER_SIGNATURES` start 0x1300, size 0x40).
        pub const USERROW_END:      usize = 0x133F;

        /// Total size of the USERROW in data space
        pub const USERROW_SIZE:    usize = USERROW_END - USERROW_START + 1;
    }
}

/// Total size of the EEPROM
pub const EEPROM_SIZE: usize = EEPROM_MAP_END - EEPROM_MAP_START + 1;

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

// All bounds checks are done in *offset space* (`0..region_size`) rather than
// on absolute data-space addresses: `usize` is 16 bits on AVR, so a check like
// `FLASH_MAP_START + offset + len - 1 > FLASH_MAP_END` silently wraps in
// release builds (overflow checks are off) and would accept wild offsets. On
// the attiny3217 the flash map moreover ends at 0xFFFF == `usize::MAX`, making
// such a comparison dead code. Validating `offset + len <= region_size` with
// checked arithmetic before ever adding the map start makes the later address
// computation provably wrap-free.
fn check_bounds(offset: usize, len: usize, region_size: usize) -> Result<(), Error> {
    match offset.checked_add(len) {
        Some(end) if end <= region_size => Ok(()),
        _ => Err(Error::OutOfBounds),
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
        check_bounds(offset, bytes.len(), FLASH_SIZE)?;

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

        // Write the new data into the page buffer.
        //
        // The pointer increments use `wrapping_add`: on the attiny3217 the
        // flash map ends at 0xFFFF, so stepping past the last byte wraps to
        // address 0. The wrapped pointer is never dereferenced — it only
        // feeds the page-boundary checks below, where 0 % FLASH_PAGE_SIZE
        // correctly reads as "page complete".
        for b in bytes.iter() {
            unsafe {
                ptr::write_volatile(ptr, *b);
                ptr = ptr.wrapping_add(1);

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
                    ptr = ptr.wrapping_add(1);
                }
            }

            self.nvmctrl_cmd(CMD_A::ERWP)?;
        }

        Ok(())
    }

    /// Erase a flash page.
    ///
    /// Erasing is page-granular: the whole [`FLASH_PAGE_SIZE`]-sized page
    /// *containing* `offset` is erased to 0xFF. The offset does not need to
    /// be page-aligned.
    ///
    /// Returns an [`Error::OutOfBounds`] in case `offset` lies outside the
    /// flash ([`FLASH_SIZE`]).
    pub fn erase_page(&self, offset: usize) -> Result<(), Error> {
        check_bounds(offset, 1, FLASH_SIZE)?;

        // The ER command erases the page addressed by the last page-buffer
        // write, so point the dummy write at the page start. Clear the
        // buffer first: the buffer auto-clears after every *completed*
        // command, but stale bytes from an abandoned fill (e.g. a dropped
        // FlashWriter) would both AND into the dummy write and, worse,
        // could address a different page than the one requested here.
        self.nvmctrl_cmd(CMD_A::PBC)?;

        let ptr = ((FLASH_MAP_START + offset) & !(FLASH_PAGE_SIZE - 1)) as *mut u8;
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
        check_bounds(offset, len, FLASH_SIZE)?;

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
    /// Data accumulates in the NVM page buffer and is committed (erased and
    /// written) whenever a page fills up; a single chunk may span any number
    /// of pages. Call [`flush`](Self::flush) at the end to commit the last
    /// partial page.
    ///
    /// Chunks must be contiguous: each call continues at the address where
    /// the previous one ended. A chunk starting elsewhere is accepted when
    /// no partial page is open, or when it starts outside the open page (the
    /// open page is flushed first, preserving its unwritten tail). A
    /// non-sequential write into the *currently open* page returns
    /// [`Error::OutOfBounds`].
    ///
    /// Returns `true` when this call committed at least one page to flash.
    pub fn write_chunk(&mut self, offset: usize, bytes: &[u8]) -> Result<bool, Error> {
        check_bounds(offset, bytes.len(), FLASH_SIZE)?;

        if bytes.is_empty() {
            return Ok(false);
        }

        let write_addr = FLASH_MAP_START + offset;
        let mut committed = false;

        // Establish where this chunk starts relative to the writer state.
        match self.current_page_start {
            Some(page_start) => {
                if write_addr != self.next_write_addr {
                    if (write_addr & !(FLASH_PAGE_SIZE - 1)) == page_start {
                        // Rewriting within the open page would AND the new
                        // bytes into already-written buffer locations.
                        return Err(Error::OutOfBounds);
                    }

                    // Jump to a different page: commit the open partial page
                    // (tail preserved by flush) and restart there.
                    self.flush()?;
                    committed = true;
                    self.next_write_addr = write_addr;
                }
            }

            // No page open — the chunk may start anywhere.
            None => self.next_write_addr = write_addr,
        }

        for &byte in bytes {
            // Lazily open the page containing the next write address. Pages
            // are opened with only their *prefix* preserved; see open_page.
            if self.current_page_start.is_none() {
                self.open_page(self.next_write_addr)?;
            }

            unsafe { ptr::write_volatile(self.next_write_addr as *mut u8, byte) };

            // `wrapping_add`: the attiny3217 flash map ends at 0xFFFF, so
            // stepping past its last byte wraps to 0 — which the modulo
            // check below correctly reads as "page complete".
            self.next_write_addr = self.next_write_addr.wrapping_add(1);

            if self.next_write_addr % FLASH_PAGE_SIZE == 0 {
                self.flash.nvmctrl_cmd(CMD_A::ERWP)?;
                committed = true;
                self.current_page_start = None;
            }
        }

        Ok(committed)
    }

    /// Commit the current partial page to flash
    ///
    /// The flash content after the last written byte is preserved by copying
    /// it into the page buffer before committing. Does nothing when no page
    /// is open.
    pub fn flush(&mut self) -> Result<(), Error> {
        if let Some(page_start) = self.current_page_start.take() {
            if self.next_write_addr == page_start {
                // Page was opened but nothing written — buffer is clean,
                // committing would needlessly erase the page.
                return Ok(());
            }

            // Preserve the tail: fill the rest of the page buffer with the
            // existing flash content (each location written exactly once,
            // avoiding the AND-combining hazard). `wrapping_add` for the
            // 0xFFFF-ending last page of the attiny3217.
            let mut addr = self.next_write_addr;
            while addr % FLASH_PAGE_SIZE != 0 {
                unsafe {
                    let existing = ptr::read_volatile(addr as *const u8);
                    ptr::write_volatile(addr as *mut u8, existing);
                }
                addr = addr.wrapping_add(1);
            }

            self.flash.nvmctrl_cmd(CMD_A::ERWP)?;
        }
        Ok(())
    }

    // Open the page containing `write_addr`: clear the page buffer and copy
    // the existing flash content *in front of* the write position into it.
    //
    // Only the prefix is copied. Prefilling the whole page and then writing
    // the new data over it — as this writer used to do — corrupts the data:
    // per AN1983, writing a page-buffer location that was already written
    // since the last PBC/commit combines the values with bitwise AND. Every
    // location is therefore written at most once between commits; the tail
    // after the last data byte is preserved lazily by `flush`.
    fn open_page(&mut self, write_addr: usize) -> Result<(), Error> {
        let page_start = write_addr & !(FLASH_PAGE_SIZE - 1);

        // The page buffer auto-clears after every completed NVM command, but
        // an abandoned writer may have left stale bytes behind — clear it
        // defensively before reuse.
        self.flash.nvmctrl_cmd(CMD_A::PBC)?;

        for addr in page_start..write_addr {
            unsafe {
                let existing = ptr::read_volatile(addr as *const u8);
                ptr::write_volatile(addr as *mut u8, existing);
            }
        }

        self.current_page_start = Some(page_start);
        Ok(())
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
        check_bounds(offset, bytes.len(), EEPROM_SIZE)?;

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

        // Commit the remaining bytes — exactly "the loop above did not just
        // flush at a page boundary".
        if (ptr as usize) % EEPROM_PAGE_SIZE > 0 {
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
        check_bounds(offset, len, EEPROM_SIZE)?;

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
    /// The whole USERROW fits into the NVM page buffer (the flash page
    /// buffer is at least twice the USERROW size on every supported
    /// device), so a single commit at the end suffices.
    ///
    /// Returns an [`Error::OutOfBounds`] in case data outside of the USERROW
    /// region is accessed. In case of a hardware write error [`Error::Write`] is returned.
    pub fn program(&self, offset: usize, bytes: &[u8]) -> Result<(), Error> {
        check_bounds(offset, bytes.len(), USERROW_SIZE)?;

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
        check_bounds(offset, 1, USERROW_SIZE)?;

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
        check_bounds(offset, len, USERROW_SIZE)?;

        let ptr = (USERROW_START + offset) as *mut u8;
        Ok(unsafe { core::slice::from_raw_parts(ptr, len) })
    }

    /// Read a single byte from USERROW.
    ///
    /// This is a convenience function for single-byte reads to save program space.
    /// For multiple bytes, use [`read`] for better efficiency.
    pub fn read_byte(&self, offset: usize) -> Result<u8, Error> {
        check_bounds(offset, 1, USERROW_SIZE)?;

        let ptr = (USERROW_START + offset) as *const u8;
        Ok(unsafe { ptr::read_volatile(ptr) })
    }

    // The `0 +` in the array lengths below is a workaround for the
    // `min_generic_const_args` feature (enabled crate-wide for the evsys
    // channel-index bounds): with that feature active, a *bare* named const
    // in type position must be declared `type const`, but a non-trivial
    // expression still goes through the stable anonymous-const path.

    /// Read the entire USERROW as an array.
    ///
    /// This is a convenience function to read all USERROW data at once.
    pub fn read_all(&self) -> [u8; 0 + USERROW_SIZE] {
        let mut data = [0u8; 0 + USERROW_SIZE];
        let ptr = USERROW_START as *const u8;
        
        for i in 0..USERROW_SIZE {
            data[i] = unsafe { ptr::read_volatile(ptr.add(i)) };
        }
        
        data
    }

    /// Write the entire USERROW from an array.
    ///
    /// This is a convenience function to write all USERROW data at once.
    pub fn program_all(&self, data: &[u8; 0 + USERROW_SIZE]) -> Result<(), Error> {
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
