//! # Brownout Detector

use crate::{
    pac::{bod, Bod},
    Toggle,
};

/// Sampling frequency
///
/// The configured sampling frequency is loaded from fusebits on reset.
#[derive(ufmt::derive::uDebug, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingFrequency {
    _1KHz,
    _125KHz,
}

/// The brownout detector mode
#[derive(ufmt::derive::uDebug, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The brownout detector is disabled
    Disabled,

    /// The brownout detector is enabled continously
    Enabled,

    /// The brownout detector is enabled but samples the voltage at
    /// regular intervals as defined by [`SamplingFrequency`]
    Sampled,

    /// The brownout detector is continously enabled during Active mode and
    /// disabled in sleep modes. When a wake-up event occurs, the wake-up is
    /// halted until the the brownout detector signals that the power is good
    EnabledAndWakeupHaltedTillBODReady,
}

impl From<Mode> for bod::ctrla::Active {
    fn from(value: Mode) -> Self {
        use bod::ctrla::Active::*;
        match value {
            Mode::Disabled => Dis,
            Mode::Enabled => Enabled,
            Mode::Sampled => Sampled,
            Mode::EnabledAndWakeupHaltedTillBODReady => Enwake,
        }
    }
}

impl From<bod::ctrla::Active> for Mode {
    fn from(value: bod::ctrla::Active) -> Self {
        use bod::ctrla::Active::*;
        match value {
            Dis => Mode::Disabled,
            Enabled => Mode::Enabled,
            Sampled => Mode::Sampled,
            Enwake => Mode::EnabledAndWakeupHaltedTillBODReady,
        }
    }
}

impl From<Mode> for bod::ctrla::Sleep {
    fn from(value: Mode) -> Self {
        use bod::ctrla::Sleep::*;
        match value {
            Mode::Disabled => Dis,
            Mode::Enabled => Enabled,
            Mode::Sampled => Sampled,
            _ => unreachable!(),
        }
    }
}

impl From<bod::ctrla::Sleep> for Mode {
    fn from(value: bod::ctrla::Sleep) -> Self {
        use bod::ctrla::Sleep::*;
        match value {
            Dis => Mode::Disabled,
            Enabled => Mode::Enabled,
            Sampled => Mode::Sampled,
        }
    }
}

/// The brownout detector level
///
/// The configured level is loaded from fusebits on reset.
#[derive(ufmt::derive::uDebug, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// 1.8V
    Level180V,

    /// 2.15V
    Level215V,

    /// 2.6V
    Level260V,

    /// 2.95V
    Level295V,

    /// 3.3V
    Level330V,

    /// 3.7V
    Level370V,

    /// 4V
    Level400V,

    /// 4.3V
    Level430V,
}

impl From<bod::ctrlb::Lvl> for Level {
    fn from(value: bod::ctrlb::Lvl) -> Self {
        use bod::ctrlb::Lvl::*;
        match value {
            Bodlevel0 => Level::Level180V,
            Bodlevel1 => Level::Level215V,
            Bodlevel2 => Level::Level260V,
            Bodlevel3 => Level::Level295V,
            Bodlevel4 => Level::Level330V,
            Bodlevel5 => Level::Level370V,
            Bodlevel6 => Level::Level400V,
            Bodlevel7 => Level::Level430V,
        }
    }
}

/// The voltage level monitor threshold relative to the BOD threshold
#[derive(ufmt::derive::uDebug, Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoltageLevelThreshold {
    /// VLM threshold 5% above BOD threshold
    FivePercentAbove,

    /// VLM threshold 15% above BOD threshold
    FifteenPercentAbove,

    /// VLM threshold 25% above BOD threshold
    TwentyfivePercentAbove,
}

impl From<VoltageLevelThreshold> for bod::vlmctrla::Vlmlvl {
    fn from(value: VoltageLevelThreshold) -> Self {
        use bod::vlmctrla::Vlmlvl::*;
        match value {
            VoltageLevelThreshold::FivePercentAbove => _5above,
            VoltageLevelThreshold::FifteenPercentAbove => _15above,
            VoltageLevelThreshold::TwentyfivePercentAbove => _25above,
        }
    }
}

impl From<bod::vlmctrla::Vlmlvl> for VoltageLevelThreshold {
    fn from(value: bod::vlmctrla::Vlmlvl) -> Self {
        use bod::vlmctrla::Vlmlvl::*;
        match value {
            _5above => VoltageLevelThreshold::FivePercentAbove,
            _15above => VoltageLevelThreshold::FifteenPercentAbove,
            _25above => VoltageLevelThreshold::TwentyfivePercentAbove,
        }
    }
}

/// The VLM (voltage level monitor) configuration
#[derive(ufmt::derive::uDebug, Debug, Clone, Copy, PartialEq, Eq)]
pub enum VlmConfiguration {
    /// Voltage falls below the VDD threshold
    VoltageFallsBelowThreshold,

    /// Voltage rises above the VDD threshold
    VoltageRisesAboveThreshold,

    /// Voltage crosses the VDD threshold from either direction
    Cross,
}

impl From<VlmConfiguration> for bod::intctrl::Vlmcfg {
    fn from(value: VlmConfiguration) -> Self {
        use bod::intctrl::Vlmcfg::*;
        match value {
            VlmConfiguration::VoltageRisesAboveThreshold => Above,
            VlmConfiguration::VoltageFallsBelowThreshold => Below,
            VlmConfiguration::Cross => Cross,
        }
    }
}

impl From<bod::intctrl::Vlmcfg> for VlmConfiguration {
    fn from(value: bod::intctrl::Vlmcfg) -> Self {
        use bod::intctrl::Vlmcfg::*;
        match value {
            Above => VlmConfiguration::VoltageRisesAboveThreshold,
            Below => VlmConfiguration::VoltageFallsBelowThreshold,
            Cross => VlmConfiguration::Cross,
        }
    }
}

/// Extension trait that constrains the [`crate::pac::Bod`] peripheral
pub trait BodExt {
    /// Constrains the [`pac::Bod`] peripheral into a configurator.
    ///
    /// Consumes the [`pac::Bod`] peripheral and converts it to a [`HAL`] internal type
    /// constraining it's public access surface to fit the design of the `HAL`.
    ///
    /// Using the [`configurator`], the peripheral can be initially configured with
    /// a builder pattern. Afterwards the settings can be changed using the
    /// provided methods.
    ///
    /// [`pac::Bod`]: `crate::pac::Bod`
    /// [`HAL`]: `crate`
    /// [`configurator`]: `BrownoutDetectorConfigurator`
    fn constrain(self) -> BrownoutDetectorConfigurator;
}

/// Constrained BOD peripheral configurator
///
/// An instance of this struct is acquired by calling the [`constrain`](BodExt::constrain) function
/// on the [`BOD`] struct.
///
/// ```
/// let dp = pac::Peripherals::take().unwrap();
/// let bod_cfg = dp.bod.constrain();
/// ```
pub struct BrownoutDetectorConfigurator {
    bod: Bod,
    sleep_mode: Option<Mode>,
    vlm_level: VoltageLevelThreshold,
    vlm_mode: VlmConfiguration,
    vlm_int: bool,
}

impl BodExt for Bod {
    fn constrain(self) -> BrownoutDetectorConfigurator {
        BrownoutDetectorConfigurator {
            bod: self,
            sleep_mode: None,
            vlm_level: VoltageLevelThreshold::FivePercentAbove,
            vlm_mode: VlmConfiguration::VoltageFallsBelowThreshold,
            vlm_int: false,
        }
    }
}

/// Configured BOD peripheral
///
/// An instance of this struct is acquired by calling the [`constrain`](BodExt::constrain) function
/// on the [`Bod`] struct and then [finishing the configuration](BrownoutDetectorConfigurator::configure)
/// on the constrained peripheral.
///
/// ```
/// let dp = pac::Peripherals::take().unwrap();
/// let bod_cfg = dp.bod.constrain();
/// let bod = bod_cfg.configure();
/// ```
pub struct BrownoutDetector {
    bod: Bod,
}

impl BrownoutDetectorConfigurator {
    /// Set the brownout detection mode when the CPU is in a sleep state
    pub fn sleep_mode(mut self, sleep_mode: Mode) -> Self {
        self.sleep_mode = Some(sleep_mode);
        self
    }

    /// Configure the voltage level monitor
    pub fn voltage_level_monitor(
        mut self,
        level: VoltageLevelThreshold,
        mode: VlmConfiguration,
        int: bool,
    ) -> Self {
        self.vlm_level = level;
        self.vlm_mode = mode;
        self.vlm_int = int;
        self
    }

    /// Apply the configuration and return a configured [`BrownoutDetector`]
    pub fn configure(self) -> BrownoutDetector {
        let mut bod = BrownoutDetector { bod: self.bod };

        if let Some(sleep_mode) = self.sleep_mode {
            bod.set_sleep_mode(sleep_mode);
        }

        bod.set_voltage_monitor_threshold(self.vlm_level);
        bod.configure_interrupt(self.vlm_int, self.vlm_mode);

        bod
    }
}

impl BrownoutDetector {
    /// Get the configured sampling frequency for the brownout detection
    ///
    /// This setting is loaded from fusebits during reset and can not be changed
    /// during runtime
    pub fn get_sampling_frequency(&self) -> SamplingFrequency {
        if self.bod.ctrla().read().sampfreq().bit_is_set() {
            SamplingFrequency::_125KHz
        } else {
            SamplingFrequency::_1KHz
        }
    }

    /// Get the configured active brownout detection mode
    ///
    /// This mode is loaded from fusebits during reset and can not be changed
    /// during runtime
    #[inline]
    pub fn get_active_mode(&self) -> Mode {
        self.bod.ctrla().read().active().variant().into()
    }

    /// Get the configured sleep brownout detection mode
    #[inline]
    pub fn get_sleep_mode(&self) -> Mode {
        self.bod.ctrla().read().sleep().variant().unwrap().into()
    }

    /// Set the configured sleep brownout detection mode
    #[inline]
    pub fn set_sleep_mode(&mut self, mode: Mode) {
        self.bod
            .ctrla()
            .modify(|_, w| w.sleep().variant(mode.into()));
    }

    /// Get the configured sleep brownout detection mode
    ///
    /// This setting is loaded from fusebits during reset and can not be changed
    /// during runtime
    #[inline]
    pub fn get_brownout_detection_level(&self) -> Level {
        self.bod.ctrlb().read().lvl().variant().into()
    }

    /// Set the current monitor threshold for the voltage level monitor.
    #[inline]
    pub fn set_voltage_monitor_threshold(&mut self, level: VoltageLevelThreshold) {
        self.bod
            .vlmctrla()
            .modify(|_, w| w.vlmlvl().variant(level.into()));
    }

    /// Get the current monitor threshold for the voltage level monitor.
    #[inline]
    pub fn get_voltage_monitor_threshold(&self) -> VoltageLevelThreshold {
        self.bod
            .vlmctrla()
            .read()
            .vlmlvl()
            .variant()
            .unwrap()
            .into()
    }

    /// Enable or disable the voltage level monitor interrupt.
    ///
    /// The passed [`VlmConfiguration`] configures when an interrupt is triggered.
    #[inline]
    pub fn configure_interrupt(&mut self, enable: impl Into<Toggle>, config: VlmConfiguration) {
        let enable: Toggle = enable.into();
        let enable: bool = enable.into();

        self.bod
            .intctrl()
            .modify(|_, w| w.vlmcfg().variant(config.into()).vlmie().bit(enable));
    }

    /// Enable the voltage level monitor interrupt.
    #[inline]
    pub fn enable_interrupt(&mut self) {
        self.bod.intctrl().modify(|_, w| w.vlmie().set_bit());
    }

    /// Disable the voltage level monitor interrupt.
    #[inline]
    pub fn disable_interrupt(&mut self) {
        self.bod.intctrl().modify(|_, w| w.vlmie().clear_bit());
    }

    /// Check if the voltage level monitoring interrupt event happend.
    #[inline]
    pub fn is_event_triggered(&self) -> bool {
        self.bod.intflags().read().vlmif().bit_is_set()
    }

    /// Clear the voltage level monitoring interrupt event.
    #[inline]
    pub fn clear_event(&mut self) {
        self.bod.intflags().modify(|_, w| w.vlmif().set_bit());
    }
}
