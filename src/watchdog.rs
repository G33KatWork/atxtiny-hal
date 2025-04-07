//! # Watchdog

use crate::pac::{
    wdt::ctrla::{Period, Window},
    Wdt as WDT,
};
use core::fmt;

use avr_device::ccp::ProtectedWritable;

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

impl Into<Period> for WatchdogTimeout {
    fn into(self) -> Period {
        match self {
            Self::Disabled => Period::Off,
            Self::Ms8 => Period::_8clk,
            Self::Ms16 => Period::_16clk,
            Self::Ms31 => Period::_32clk,
            Self::Ms63 => Period::_64clk,
            Self::Ms125 => Period::_128clk,
            Self::Ms250 => Period::_256clk,
            Self::Ms500 => Period::_512clk,
            Self::S1 => Period::_1kclk,
            Self::S2 => Period::_2kclk,
            Self::S4 => Period::_4kclk,
            Self::S8 => Period::_8kclk,
        }
    }
}

impl Into<Window> for WatchdogTimeout {
    fn into(self) -> Window {
        match self {
            Self::Disabled => Window::Off,
            Self::Ms8 => Window::_8clk,
            Self::Ms16 => Window::_16clk,
            Self::Ms31 => Window::_32clk,
            Self::Ms63 => Window::_64clk,
            Self::Ms125 => Window::_128clk,
            Self::Ms250 => Window::_256clk,
            Self::Ms500 => Window::_512clk,
            Self::S1 => Window::_1kclk,
            Self::S2 => Window::_2kclk,
            Self::S4 => Window::_4kclk,
            Self::S8 => Window::_8kclk,
        }
    }
}

/// Extension trait that constrains the [`crate::pac::Wdt`] peripheral
pub trait WdtExt: crate::private::Sealed {
    /// Constrains the [`pac::WDT`] peripheral.
    ///
    /// Consumes the [`pac::WDT`] peripheral and converts it to a [`HAL`] internal type
    /// constraining it's public access surface to fit the design of the `HAL`.
    ///
    /// [`pac::WDT`]: `crate::pac::Wdt`
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
    fn setup(&self, timeout: WatchdogTimeout, window: Option<WatchdogTimeout>) {
        let window = window.unwrap_or(WatchdogTimeout::Disabled);

        self.wdt.ctrla().write_protected(|w| {
            w.period()
                .variant(timeout.into())
                .window()
                .variant(window.into())
        });
    }

    /// Lock the watchdog peripheral.
    ///
    /// Once this function has been called, it cannot be reconfigured anymore
    pub fn lock(&self) {
        self.wdt.status().write_protected(|w| w.lock().set_bit());
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

    /// Feed the watchdog and prevent it from expiring
    ///
    /// NOTE: This was an Embedded-HAL trait method once which was removed and
    /// will be added back at a later time
    #[inline(always)]
    pub fn feed(&mut self) {
        avr_device::asm::wdr()
    }
}
