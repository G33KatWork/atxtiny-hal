use super::{AsClockSource, Counter, CounterHz, Delay, Error, General, Instance, PeriodicMode};
use crate::time::*;
use crate::Toggle;

/// Timer wrapper for dynamic precision timers
///
/// This wrapper can be used for all different timer variants (A, B and D)
pub struct Timer<TIM: Instance> {
    pub(crate) tim: TIM,
    pub(crate) clk: TIM::ClockSource,
}

impl<TIM: Instance + General> Timer<TIM> {
    /// Initialize timer
    pub fn new(mut tim: TIM, clk: TIM::ClockSource) -> Self {
        tim.reset_counter_peripheral();
        Self { tim, clk }
    }

    /// Releases the TIM peripheral
    pub fn release(self) -> TIM {
        self.tim
    }

    #[inline]
    pub fn enable_interrupt(&mut self, interrupt: TIM::Interrupt) {
        self.configure_interrupt(interrupt, Toggle::On);
    }

    #[inline]
    pub fn disable_interrupt(&mut self, interrupt: TIM::Interrupt) {
        self.configure_interrupt(interrupt, Toggle::Off);
    }

    #[inline]
    pub fn configure_interrupt(&mut self, interrupt: TIM::Interrupt, enable: impl Into<Toggle>) {
        self.tim.configure_interrupt(interrupt, enable);
    }

    #[cfg(feature = "enumset")]
    #[cfg_attr(docsrs, doc(cfg(feature = "enumset")))]
    #[inline]
    pub fn configure_interrupts(&mut self, interrupts: enumset::EnumSet<TIM::Interrupt>) {
        for event in interrupts.complement().iter() {
            self.configure_interrupt(event, false);
        }

        for event in interrupts.iter() {
            self.configure_interrupt(event, true);
        }
    }

    #[inline]
    pub fn is_interrupt_configured(&self, interrupt: TIM::Interrupt) -> bool {
        self.tim.is_interrupt_configured(interrupt)
    }

    #[cfg(feature = "enumset")]
    #[cfg_attr(docsrs, doc(cfg(feature = "enumset")))]
    #[inline]
    pub fn configured_interrupts(&mut self) -> enumset::EnumSet<TIM::Interrupt> {
        let mut interrupts = EnumSet::new();

        for interrupt in enumset::EnumSet::<TIM::Interrupt>::all().iter() {
            if self.is_interrupt_configured(interrupt) {
                interrupts |= interrupt;
            }
        }

        interrupts
    }

    #[inline]
    pub fn is_event_triggered(&self, event: TIM::Event) -> bool {
        self.tim.is_event_triggered(event)
    }

    #[cfg(feature = "enumset")]
    #[cfg_attr(docsrs, doc(cfg(feature = "enumset")))]
    #[inline]
    pub fn triggered_events(&self) -> enumset::EnumSet<TIM::Event> {
        let mut events = enumset::EnumSet::new();

        for event in enumset::EnumSet::<TIM::Event>::all().iter() {
            if self.is_event_triggered(event) {
                events |= event;
            }
        }

        events
    }

    #[inline]
    pub fn clear_event(&mut self, event: TIM::Event) {
        self.tim.clear_event(event)
    }
}

impl<TIM: Instance + General + PeriodicMode> Timer<TIM> {
    /// Creates [`CounterHz`] with dynamic precision
    pub fn counter_hz(self) -> CounterHz<TIM> {
        CounterHz(self)
    }
}

// // FIXME: add this for tcb sync feature?
// impl<TIM: Instance + MasterTimer> Timer<TIM> {
//     pub fn set_master_mode(&mut self, mode: TIM::Mms) {
//         self.tim.master_mode(mode)
//     }
// }

/// Timer wrapper for fixed precision timers
///
/// Uses `fugit::TimerDurationU32` for most of operations
pub struct FTimer<TIM, const FREQ: u32> {
    pub(crate) tim: TIM,
}

// /// `FTimer` with precision of 1 μs (1 MHz sampling)
// pub type FTimerUs<TIM> = FTimer<TIM, 1_000_000>;
//
// /// `FTimer` with precision of 1 ms (1 kHz sampling)
// pub type FTimerMs<TIM> = FTimer<TIM, 1_000>;

impl<TIM: Instance + General, const FREQ: u32> FTimer<TIM, FREQ> {
    /// Initialize timer
    pub fn new(mut tim: TIM, clk: TIM::ClockSource) -> Result<Self, Error> {
        tim.reset_counter_peripheral();
        let mut t = Self { tim };
        t.configure(clk)?;
        Ok(t)
    }

    /// Calculate and set prescaler depending on `Clocks` state
    pub fn configure(&mut self, clk: TIM::ClockSource) -> Result<(), Error> {
        self.tim.prepare_clock_source(clk);

        let clk_rate = TIM::get_input_clock_rate(clk);
        if clk_rate.raw() % FREQ != 0 {
            return Err(Error::ImpossiblePrescaler);
        }

        let psc = (clk_rate.raw() / FREQ) as u16;
        if !TIM::is_prescaler_valid(psc, clk) {
            return Err(Error::ImpossiblePrescaler);
        }

        self.tim.set_prescaler(psc);

        Ok(())
    }

    /// Releases the TIM peripheral
    pub fn release(self) -> TIM {
        self.tim
    }

    #[inline]
    pub fn enable_interrupt(&mut self, interrupt: TIM::Interrupt) {
        self.configure_interrupt(interrupt, Toggle::On);
    }

    #[inline]
    pub fn disable_interrupt(&mut self, interrupt: TIM::Interrupt) {
        self.configure_interrupt(interrupt, Toggle::Off);
    }

    #[inline]
    pub fn configure_interrupt(&mut self, interrupt: TIM::Interrupt, enable: impl Into<Toggle>) {
        self.tim.configure_interrupt(interrupt, enable);
    }

    #[cfg(feature = "enumset")]
    #[cfg_attr(docsrs, doc(cfg(feature = "enumset")))]
    #[inline]
    pub fn configure_interrupts(&mut self, interrupts: enumset::EnumSet<TIM::Interrupt>) {
        for event in interrupts.complement().iter() {
            self.configure_interrupt(event, false);
        }

        for event in interrupts.iter() {
            self.configure_interrupt(event, true);
        }
    }

    #[inline]
    pub fn is_interrupt_configured(&self, interrupt: TIM::Interrupt) -> bool {
        self.tim.is_interrupt_configured(interrupt)
    }

    #[cfg(feature = "enumset")]
    #[cfg_attr(docsrs, doc(cfg(feature = "enumset")))]
    #[inline]
    pub fn configured_interrupts(&mut self) -> enumset::EnumSet<TIM::Interrupt> {
        let mut interrupts = enumset::EnumSet::new();

        for interrupt in enumset::EnumSet::<TIM::Interrupt>::all().iter() {
            if self.is_interrupt_configured(interrupt) {
                interrupts |= interrupt;
            }
        }

        interrupts
    }

    #[inline]
    pub fn is_event_triggered(&self, event: TIM::Event) -> bool {
        self.tim.is_event_triggered(event)
    }

    #[cfg(feature = "enumset")]
    #[cfg_attr(docsrs, doc(cfg(feature = "enumset")))]
    #[inline]
    pub fn triggered_events(&self) -> enumset::EnumSet<TIM::Event> {
        let mut events = enumset::EnumSet::new();

        for event in enumset::EnumSet::<TIM::Event>::all().iter() {
            if self.is_event_triggered(event) {
                events |= event;
            }
        }

        events
    }

    #[inline]
    pub fn clear_event(&mut self, event: TIM::Event) {
        self.tim.clear_event(event)
    }
}

impl<TIM: Instance + General + PeriodicMode, const FREQ: u32> FTimer<TIM, FREQ> {
    /// Creates `Counter`
    pub fn counter(self) -> Counter<TIM, FREQ> {
        Counter(self)
    }

    /// Creates `Delay` that imlements [embedded_hal::delay::DelayNs] traits
    pub fn delay(self) -> Delay<TIM, FREQ> {
        Delay(self)
    }
}

impl<TIM: AsClockSource, const FREQ: u32> FTimer<TIM, FREQ> {
    pub fn use_as_clock_source(&self) -> TIM::OutputClock {
        self.tim.use_as_clock_source(Hertz::from_raw(FREQ))
    }
}
