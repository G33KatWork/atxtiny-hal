//! # Watchdog

use crate::pac::{
    wdt::ctrla::{PERIOD_A, WINDOW_A},
    WDT,
};
use core::fmt;

use avr_device::generic::ProtectedWritable;

/// The timeout how long it should take for the watchdog take to expire when
/// it's not fed by calling [`feed`]
///
/// [`feed`]: `WatchdogTimer::feed`
#[derive(ufmt::derive::uDebug, Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchdogTimeout {
    Disabled,
    Ms8,
    Ms16,
    Ms31,
    Ms63,
    Ms125,
    Ms250,
    Ms500,
    S1,
    S2,
    S4,
    S8,
}

impl From<WatchdogTimeout> for PERIOD_A {
    fn from(value: WatchdogTimeout) -> Self {
        match value {
            WatchdogTimeout::Disabled => PERIOD_A::OFF,
            WatchdogTimeout::Ms8 => PERIOD_A::_8CLK,
            WatchdogTimeout::Ms16 => PERIOD_A::_16CLK,
            WatchdogTimeout::Ms31 => PERIOD_A::_32CLK,
            WatchdogTimeout::Ms63 => PERIOD_A::_64CLK,
            WatchdogTimeout::Ms125 => PERIOD_A::_128CLK,
            WatchdogTimeout::Ms250 => PERIOD_A::_256CLK,
            WatchdogTimeout::Ms500 => PERIOD_A::_512CLK,
            WatchdogTimeout::S1 => PERIOD_A::_1KCLK,
            WatchdogTimeout::S2 => PERIOD_A::_2KCLK,
            WatchdogTimeout::S4 => PERIOD_A::_4KCLK,
            WatchdogTimeout::S8 => PERIOD_A::_8KCLK,
        }
    }
}

impl From<WatchdogTimeout> for WINDOW_A {
    fn from(value: WatchdogTimeout) -> Self {
        match value {
            WatchdogTimeout::Disabled => WINDOW_A::OFF,
            WatchdogTimeout::Ms8 => WINDOW_A::_8CLK,
            WatchdogTimeout::Ms16 => WINDOW_A::_16CLK,
            WatchdogTimeout::Ms31 => WINDOW_A::_32CLK,
            WatchdogTimeout::Ms63 => WINDOW_A::_64CLK,
            WatchdogTimeout::Ms125 => WINDOW_A::_128CLK,
            WatchdogTimeout::Ms250 => WINDOW_A::_256CLK,
            WatchdogTimeout::Ms500 => WINDOW_A::_512CLK,
            WatchdogTimeout::S1 => WINDOW_A::_1KCLK,
            WatchdogTimeout::S2 => WINDOW_A::_2KCLK,
            WatchdogTimeout::S4 => WINDOW_A::_4KCLK,
            WatchdogTimeout::S8 => WINDOW_A::_8KCLK,
        }
    }
}

/// Extension trait that constrains the [`crate::pac::WDT`] peripheral
pub trait WdtExt: crate::private::Sealed {
    /// Constrains the [`pac::WDT`] peripheral.
    ///
    /// Consumes the [`pac::WDT`] peripheral and converts it to a [`HAL`] internal type
    /// constraining it's public access surface to fit the design of the `HAL`.
    ///
    /// [`pac::WDT`]: `crate::pac::WDT`
    /// [`HAL`]: `crate`
    fn constrain(self) -> WatchdogTimer;
}

impl crate::private::Sealed for WDT {}

impl WdtExt for WDT {
    fn constrain(self) -> WatchdogTimer {
        WatchdogTimer { wdt: self }
    }
}

/// Constrained Watchdog peripheral
///
/// An instance of this struct is acquired by calling the [`constrain`](WdtExt::constrain) function
/// on the [`WDT`] struct.
///
/// ```
/// let dp = pac::Peripherals::take().unwrap();
/// let watchdog = dp.WDT.constrain();
/// ```
pub struct WatchdogTimer {
    wdt: WDT,
}

impl fmt::Debug for WatchdogTimer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WatchdogTimer")
            .field("wdt", &"WDT")
            .finish()
    }
}

impl WatchdogTimer {
    /// Write the timeout and window values into the CTRLA register
    ///
    /// NOTE: When `STATUS.LOCK` is set (via [`lock`](WatchdogTimer::lock) or
    /// the fuse-enabled watchdog), the hardware ignores all CTRLA writes, so
    /// reconfiguration is silently without effect.
    fn setup(&self, timeout: WatchdogTimeout, window: Option<WatchdogTimeout>) {
        let window = window.unwrap_or(WatchdogTimeout::Disabled);

        // CTRLA writes are ignored while a previous write is still being
        // synchronized into the watchdog clock domain (2-3 WDT clock cycles,
        // i.e. milliseconds), so wait for the synchronizer to go idle first.
        // This matters at boot (a fuse-driven configuration may still be
        // synchronizing) and for back-to-back reconfiguration.
        while self.wdt.status().read().syncbusy().bit_is_set() {}

        self.wdt.ctrla().write_protected(|w| {
            w.period()
                .variant(timeout.into())
                .window()
                .variant(window.into())
        });
    }

    /// Lock the watchdog peripheral.
    ///
    /// Once this function has been called, the watchdog cannot be
    /// reconfigured anymore until the next reset — the hardware silently
    /// ignores all further CTRLA writes. Consuming the [`WatchdogTimer`]
    /// makes those dead reconfiguration attempts unrepresentable; the
    /// returned [`LockedWatchdogTimer`] can only be fed.
    pub fn lock(self) -> LockedWatchdogTimer {
        self.wdt.status().write_protected(|w| w.lock().set_bit());
        LockedWatchdogTimer { _wdt: self.wdt }
    }

    /// Get access to the underlying register block.
    ///
    /// # Safety
    ///
    /// This function is not _memory_ unsafe per se, but does not guarantee
    /// anything about assumptions of invariants made in this implementation.
    ///
    /// Changing specific options can lead to un-expected behavior and nothing
    /// is guaranteed.
    pub unsafe fn peripheral(&mut self) -> &mut WDT {
        &mut self.wdt
    }

    /// Start the watchdog with the supplied timeout period
    ///
    /// NOTE: This was an Embedded-HAL trait method once which was removed and
    /// will be added back at a later time
    pub fn start(&mut self, period: WatchdogTimeout) {
        self.setup(period, None);
    }

    /// Start the watchdog in window mode
    ///
    /// After each feed the window stays *closed* for the `window` duration,
    /// then remains open for the `timeout` duration. The watchdog resets the
    /// system when it is not fed within the open window, so the total period
    /// between feeds may be up to `window + timeout`.
    ///
    /// WARNING: Feeding the watchdog while the window is still closed (i.e.
    /// earlier than `window` after the previous feed) immediately resets the
    /// system. Make sure every feed site in the program respects the window
    /// before enabling this mode.
    pub fn start_windowed(&mut self, timeout: WatchdogTimeout, window: WatchdogTimeout) {
        self.setup(timeout, Some(window));
    }

    /// Feed the watchdog and prevent it from expiring
    ///
    /// NOTE: This was an Embedded-HAL trait method once which was removed and
    /// will be added back at a later time
    #[inline(always)]
    pub fn feed(&mut self) {
        avr_device::asm::wdr()
    }
}

/// A locked watchdog timer
///
/// Obtained by calling [`lock`](WatchdogTimer::lock). The configuration is
/// frozen in hardware until the next reset, so this type only exposes
/// feeding.
pub struct LockedWatchdogTimer {
    _wdt: WDT,
}

impl LockedWatchdogTimer {
    /// Feed the watchdog and prevent it from expiring
    #[inline(always)]
    pub fn feed(&mut self) {
        avr_device::asm::wdr()
    }
}
