use crate::{time::*, Toggle};

use super::tcb::{Event, Interrupt, TCBClockSource, Tcb8bitPwmCapable};

/// A TCB instance reconfigured into its 8-bit PWM mode (CNTMODE = PWM8).
///
/// Obtained via [`Tcb8bitPwmCapable::into_8bit_pwm`]; the type parameter is
/// the underlying PAC instance (e.g. `TCB8Bit<TCB0>`).
pub struct TCB8Bit<TCB> {
    pub(crate) tim: TCB,
}

impl<TCB: Tcb8bitPwmCapable> super::Instance for TCB8Bit<TCB> {}
impl<TCB: Tcb8bitPwmCapable> crate::private::Sealed for TCB8Bit<TCB> {}

// Clocking and the generic counter plumbing only go through the trait
// methods of the wrapped instance, so they are written once for every
// `Tcb8bitPwmCapable` instance. The register-touching impls (PeriodicMode,
// WithPwm) live in the `tcb_8bit!` macro below instead.

impl<TCB: Tcb8bitPwmCapable> super::TimerClock for TCB8Bit<TCB> {
    type ClockSource = TCBClockSource;

    #[inline(always)]
    fn get_input_clock_rate(clk: Self::ClockSource) -> Hertz {
        TCB::get_input_clock_rate(clk)
    }

    #[inline(always)]
    fn prepare_clock_source(&mut self, clk: Self::ClockSource) {
        self.tim.prepare_clock_source(clk)
    }

    #[inline(always)]
    fn get_valid_prescalers(clk: Self::ClockSource) -> &'static [u16] {
        TCB::get_valid_prescalers(clk)
    }

    #[inline(always)]
    fn set_prescaler(&mut self, psc: u16) {
        self.tim.set_prescaler(psc)
    }

    #[inline(always)]
    fn read_prescaler(&self) -> u16 {
        self.tim.read_prescaler()
    }
}

impl<TCB: Tcb8bitPwmCapable> super::General for TCB8Bit<TCB> {
    const TIMER_WIDTH_BITS: u8 = 8;
    type CounterValue = u8;
    type Interrupt = Interrupt;
    type Event = Event;

    #[inline(always)]
    fn reset_counter_peripheral(&mut self) {}

    #[inline(always)]
    fn enable_counter(&mut self) {
        self.tim.enable_counter();
    }

    #[inline(always)]
    fn disable_counter(&mut self) {
        self.tim.disable_counter();
    }

    #[inline(always)]
    fn is_counter_enabled(&self) -> bool {
        self.tim.is_counter_enabled()
    }

    #[inline(always)]
    fn reset_count(&mut self) {
        self.tim.reset_count();
    }

    #[inline(always)]
    fn read_count(&self) -> Self::CounterValue {
        self.tim.read_count() as u8
    }

    #[inline(always)]
    fn configure_interrupt(&mut self, interrupt: Self::Interrupt, enable: impl Into<Toggle>) {
        self.tim.configure_interrupt(interrupt, enable)
    }

    #[inline(always)]
    fn is_interrupt_configured(&self, interrupt: Self::Interrupt) -> bool {
        self.tim.is_interrupt_configured(interrupt)
    }

    #[inline(always)]
    fn is_event_triggered(&self, event: Self::Event) -> bool {
        self.tim.is_event_triggered(event)
    }

    #[inline(always)]
    fn clear_event(&mut self, event: Self::Event) {
        self.tim.clear_event(event)
    }
}

// ============================================================================
// CCMP access discipline
// ============================================================================
//
// In PWM8 mode CCMPL holds the period and CCMPH the duty cycle, but byte
// accesses to the two halves still go through the peripheral's ONE shared
// TEMP register — PWM8 mode does not exempt CCMP from the 16-bit access
// mechanism. Hardware-validated on an ATtiny1617 (2026-07-17) with a
// throwaway register probe; raw results in this change's commit message:
//
// * reading CCMPL latches CCMPH into TEMP; a later lone CCMPH write commits
//   TEMP into CCMPL — the period gets replaced by a stale duty value,
// * TEMP is shared with CNT, so a 16-bit CNT read between duty updates
//   poisons the period the same way,
// * a lone CCMPH read returns TEMP content, not the register.
//
// Therefore CCMP is only ever accessed as one 16-bit operation here (the
// compiler emits the low-then-high sequence that drives TEMP correctly);
// updating one half is a 16-bit read-modify-write preserving the other.
//
// On top of that, TEMP is shared with every other 16-bit register of the
// peripheral and with any ISR touching them, so each CCMP access — and in
// particular the read-modify-write pairs — runs inside a critical section.
//
// These impls are stamped out per TCB instance rather than written
// generically: they access registers directly, and the PAC generates a
// separate register module per instance, so the register enum types (e.g.
// `CNTMODE_A`) are distinct types.
macro_rules! tcb_8bit {
    ($TCB:ident) => {

impl Tcb8bitPwmCapable for crate::pac::$TCB {
    fn into_8bit_pwm(self) -> TCB8Bit<crate::pac::$TCB> {
        TCB8Bit { tim: self }
    }
}

impl super::PeriodicMode for TCB8Bit<crate::pac::$TCB> {
    const PERIOD_DOUBLE_BUFFERED: bool = false;

    #[inline(always)]
    fn set_periodic_mode(&mut self) {
        self.tim.ctrlb().modify(|_, w| w.cntmode().pwm8());
    }

    #[inline(always)]
    fn read_period() -> Self::CounterValue {
        // FIXME: function needs to be called from PwmChannel where we don't
        //        have a reference to the Timer, hence this stuff
        //        When the split pwm channels get a ref to the timer, we can
        //        get rid of this again
        let tim = unsafe { &*crate::pac::$TCB::ptr() };
        critical_section::with(|_| (tim.ccmp().read().bits() & 0x00FF) as u8)
    }

    #[inline(always)]
    fn trigger_update(&mut self) {
        // no double buffering, no updating...
    }

    #[inline(always)]
    unsafe fn set_period_unchecked(&mut self, period: Self::CounterValue) {
        // The old lone CCMPL write did not even take effect on its own: the
        // low-byte write only loads TEMP, and the value was committed by
        // whichever high-byte write happened next.
        critical_section::with(|_| {
            let duty = self.tim.ccmp().read().bits() & 0xFF00;
            unsafe { self.tim.ccmp().write(|w| w.bits(duty | period as u16)) };
        });
    }

    #[inline(always)]
    fn max_period() -> Self::CounterValue {
        u8::MAX
    }

    #[inline(always)]
    fn clear_overflow(&mut self) {
        self.tim.intflags().write(|w| w.capt().set_bit());
    }

    #[inline(always)]
    fn get_overflow(&self) -> bool {
        self.tim.intflags().read().capt().bit_is_set()
    }
}

impl super::WithPwm for TCB8Bit<crate::pac::$TCB> {
    const CH_NUMBER: u8 = 1;
    type GenerationMode = ();
    type CompareValue = u8;

    #[inline(always)]
    fn max_compare_value() -> Self::CompareValue {
        u8::MAX
    }

    // Period: CCMPL
    // Compare: CCMPH

    fn set_pwm_mode(&mut self, _mode: Self::GenerationMode) {
        self.tim.ctrlb().write(|w| w.cntmode().pwm8());
    }

    #[inline(always)]
    fn is_period_driven(_mode: &Self::GenerationMode) -> bool {
        // PWM8 always takes its TOP from CCMPL, which is what
        // set_period_unchecked writes.
        true
    }

    fn enable_channel(channel: u8, b: bool) {
        let tim = unsafe { &*crate::pac::$TCB::ptr() };
        match channel {
            0 => _ = tim.ctrlb().modify(|_, w| w.ccmpen().bit(b)),
            _ => panic!("invalid channel number"),
        }
    }

    fn set_compare_value(channel: u8, value: Self::CompareValue) {
        let tim = unsafe { &*crate::pac::$TCB::ptr() };
        match channel {
            0 => critical_section::with(|_| {
                let period = tim.ccmp().read().bits() & 0x00FF;
                tim.ccmp()
                    .write(|w| unsafe { w.bits(((value as u16) << 8) | period) });
            }),
            _ => panic!("invalid channel number"),
        }
    }

    fn read_compare_value(channel: u8) -> Self::CompareValue {
        let tim = unsafe { &*crate::pac::$TCB::ptr() };
        match channel {
            0 => critical_section::with(|_| (tim.ccmp().read().bits() >> 8) as u8),
            _ => panic!("invalid channel number"),
        }
    }

    #[inline(always)]
    fn clear_compare_match(channel: u8) {
        let tim = unsafe { &*crate::pac::$TCB::ptr() };
        match channel {
            0 => _ = tim.intflags().write(|w| w.capt().set_bit()),
            _ => panic!("invalid channel number"),
        }
    }

    #[inline(always)]
    fn get_compare_match(channel: u8) -> bool {
        let tim = unsafe { &*crate::pac::$TCB::ptr() };
        match channel {
            0 => tim.intflags().read().capt().bit_is_set(),
            _ => panic!("invalid channel number"),
        }
    }
}

    };
}

tcb_8bit!(TCB0);
#[cfg(feature = "periph-tcb1")]
tcb_8bit!(TCB1);
