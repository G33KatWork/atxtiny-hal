//! # Voltage reference

// TODO: macros for different CPUs which have different peripherals
// FIXME: move this into the DAC and ADC modules? DAC and AC share the channel though

use crate::Toggle;

/// Extension trait that constrains the [`crate::pac::VREF`] peripheral
pub trait VrefExt {
    /// Constrains the [`pac::VREF`] peripheral.
    ///
    /// Consumes the [`pac::VREF`] peripheral and converts it to a [`HAL`] internal type
    /// constraining it's public access surface to fit the design of the `HAL`.
    ///
    /// Returns the constrained peripheral together with one ownership token
    /// per reference-voltage channel.
    ///
    /// [`pac::VREF`]: `crate::pac::VREF`
    /// [`HAL`]: `crate`
    fn constrain(self) -> Parts;
}

/// Constrained VREF peripheral
///
/// An instance of this struct is acquired by calling the [`constrain`](VrefExt::constrain) function
/// on the [`VREF`](crate::pac::VREF) struct.
///
/// ```
/// let dp = pac::Peripherals::take().unwrap();
/// let vref = dp.VREF.constrain().vref;
/// ```
pub struct Vref {
    vref: crate::pac::VREF,
}

/// The constrained VREF peripheral and one ownership token per
/// reference-voltage channel
pub struct Parts {
    pub vref: Vref,
    pub adc0: ADCReferenceVoltage<0>,
    pub dac0: DACReferenceVoltage<0>,
}

impl VrefExt for crate::pac::VREF {
    fn constrain(self) -> Parts {
        Parts {
            vref: Vref { vref: self },
            adc0: ADCReferenceVoltage,
            dac0: DACReferenceVoltage,
        }
    }
}

// The reference tokens are deliberately neither `Copy` nor `Clone` and are
// minted exactly once, by `constrain`: a peripheral that stores the token for
// its reference channel thereby holds exclusive use of that channel, and
// methods that mutate the channel configuration require the token.

/// Reference voltage for an ADC
#[derive(Eq, PartialEq)]
pub struct ADCReferenceVoltage<const IDX: u8>;

/// Reference voltage for a DAC
#[derive(Eq, PartialEq)]
pub struct DACReferenceVoltage<const IDX: u8>;

impl<const IDX: u8> crate::private::Sealed for ADCReferenceVoltage<IDX> {}
impl<const IDX: u8> crate::private::Sealed for DACReferenceVoltage<IDX> {}

macro_rules! impl_reference_voltage {
    ($periphname:ident, $refstruct:ty, $refvolttype:ty, $refselreg:ident, $refselbits:ident, $forceenreg:ident, $forceenbit:ident) => {
        #[doc = "The reference voltage for the peripheral "]
        #[doc = stringify!($periphname)]
        impl $refstruct {
            /// Set the reference voltage to the specified level.
            pub fn voltage(&mut self, vref: &mut Vref, voltage: $refvolttype) {
                vref.vref
                    .$refselreg()
                    .modify(|_, w| unsafe { w.$refselbits().bits(voltage as u8) });
            }

            /// Force-enable the reference voltage.
            ///
            /// Usually the peripherals that use the reference voltage enable it
            /// automatically. Using this method it can be force-enabled.
            pub fn force(&mut self, vref: &mut Vref, force: impl Into<Toggle>) {
                let force: Toggle = force.into();
                let force: bool = force.into();
                vref.vref
                    .$forceenreg()
                    .modify(|_, w| w.$forceenbit().bit(force));
            }
        }
    };
}

/// Reference Voltage.
#[derive(ufmt::derive::uDebug, Debug, Copy, Clone, PartialEq, Eq)]
pub enum ReferenceVoltage {
    /// 0.55V
    _0V55 = 0x00,

    /// 1.1V
    _1V10 = 0x01,

    /// 2.5V
    _2V50 = 0x02,

    /// 4.34V
    _4V34 = 0x03,

    /// 1.5V
    _1V50 = 0x04,
}

impl_reference_voltage!(
    ADC0,
    ADCReferenceVoltage<0>,
    ReferenceVoltage,
    ctrla,
    adc0refsel,
    ctrlb,
    adc0refen
);

impl_reference_voltage!(
    DAC0,
    DACReferenceVoltage<0>,
    ReferenceVoltage,
    ctrla,
    dac0refsel,
    ctrlb,
    dac0refen
);
