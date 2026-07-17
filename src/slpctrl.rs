//! # Sleep Controller

use core::arch::asm;

use crate::pac::{slpctrl, SLPCTRL};

/// Extension trait that constrains the [`crate::pac::SLPCTRL`] peripheral
pub trait SlpctrlExt {
    /// Constrains the [`pac::SLPCTRL`] peripheral.
    ///
    /// Consumes the [`pac::SLPCTRL`] peripheral and converts it to a [`HAL`] internal type
    /// constraining it's public access surface to fit the design of the `HAL`.
    ///
    /// [`pac::SLPCTRL`]: `crate::pac::SLPCTRL`
    /// [`HAL`]: `crate`
    fn constrain(self) -> Slpctrl;
}

/// Constrained Slpctrl peripheral
///
/// An instance of this struct is acquired by calling the [`constrain`](SlpctrlExt::constrain) function
/// on the [`SLPCTRL`] struct.
///
/// ```
/// let dp = pac::Peripherals::take().unwrap();
/// let slpctrl = dp.SLPCTRL.constrain();
/// ```
pub struct Slpctrl;

impl SlpctrlExt for SLPCTRL {
    fn constrain(self) -> Slpctrl {
        Slpctrl
    }
}

impl Slpctrl {
    /// Set the desired [sleep mode](SleepMode)
    pub fn set_sleep_mode(&mut self, mode: SleepMode) {
        let ctrla = unsafe { &(*SLPCTRL::ptr()).ctrla() };
        ctrla.modify(|_, w| w.smode().variant(mode.into()));
    }

    /// Enter the [previously configured](Slpctrl::set_sleep_mode) sleep mode
    ///
    /// This function sets the sleep-enable bit, performs the sleep and clears
    /// the enable bit once the CPU woke up again and yielded control back to
    /// the non-interrupt context. It returns with interrupts globally
    /// **enabled** — waking from sleep requires servicing the wake interrupt.
    ///
    /// To sleep race-free until an event, call this with interrupts globally
    /// *disabled*, after checking the wake condition:
    ///
    /// ```ignore
    /// avr_device::interrupt::disable();
    /// while !wake_condition() {
    ///     slpctrl.sleep(); // atomically re-enables interrupts and sleeps
    ///     avr_device::interrupt::disable();
    /// }
    /// unsafe { avr_device::interrupt::enable() };
    /// ```
    ///
    /// If instead interrupts are enabled on entry, the classic AVR sleep race
    /// applies: a wake event arriving just before the `sleep` instruction is
    /// serviced first and the CPU then sleeps anyway — forever, if the event
    /// was a one-shot.
    pub fn sleep(&mut self) {
        let ctrla = unsafe { &(*SLPCTRL::ptr()).ctrla() };
        ctrla.modify(|_, w| w.sen().set_bit());
        // `sei` only takes effect after the *following* instruction, so with
        // interrupts disabled on entry no interrupt can slip in between
        // enabling them and entering sleep: pending wake events are serviced
        // after the CPU is already sleeping, i.e. they wake it as intended.
        unsafe { asm!("sei", "sleep") };
        ctrla.modify(|_, w| w.sen().clear_bit());
    }
}

/// The desired sleep mode that is to be entered when calling
/// [`sleep`](Slpctrl::sleep)
pub enum SleepMode {
    Idle,
    Standby,
    PowerDown,
}

impl From<SleepMode> for slpctrl::ctrla::SMODE_A {
    fn from(value: SleepMode) -> Self {
        match value {
            SleepMode::Idle => slpctrl::ctrla::SMODE_A::IDLE,
            SleepMode::Standby => slpctrl::ctrla::SMODE_A::STANDBY,
            SleepMode::PowerDown => slpctrl::ctrla::SMODE_A::PDOWN,
        }
    }
}
