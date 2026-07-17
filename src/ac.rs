//! # Analog comparator

use crate::{
    gpio::{Analog, Output, Stateless},
    pac::AC0,
};
use core::marker::PhantomData;

#[cfg(feature = "periph-dac0")]
use crate::dac::DACOutputToAC;

/// Enabled Comparator (type state)
pub struct Enabled;

/// Disabled Comparator (type state)
pub struct Disabled;

pub trait ED {}
impl ED for Enabled {}
impl ED for Disabled {}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Config {
    pub hysteresis: Hysteresis,
    /// Trade comparator response time for lower current draw.
    ///
    /// Only the 1-series comparators have the LPMODE bit, so the option
    /// does not exist on 0-series chips.
    #[cfg(feature = "series-1")]
    pub low_power_mode: bool,
    pub inverted: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hysteresis: Hysteresis::Off,
            #[cfg(feature = "series-1")]
            low_power_mode: false,
            inverted: false,
        }
    }
}

impl Config {
    pub fn hysteresis(mut self, hysteresis: Hysteresis) -> Self {
        self.hysteresis = hysteresis;
        self
    }

    #[cfg(feature = "series-1")]
    pub fn low_power_mode(mut self) -> Self {
        self.low_power_mode = true;
        self
    }

    pub fn output_inverted(mut self) -> Self {
        self.inverted = true;
        self
    }

    pub fn output_polarity(mut self, inverted: bool) -> Self {
        self.inverted = inverted;
        self
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Hysteresis {
    Off = 0,
    _10mV = 1,
    _25mV = 2,
    _50mV = 3,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum InterruptMode {
    BothEdges = 0,
    NegativeEdge = 2,
    PositiveEdge = 3,
}

pub struct Comparator<AC, ED> {
    regs: AC,
    _enabled: PhantomData<ED>,
}

pub trait ComparatorExt<AC> {
    /// Initializes a comparator
    fn comparator<P: PositiveInput<AC>, N: NegativeInput<AC>>(
        self,
        positive_input: P,
        negative_input: N,
        config: Config,
    ) -> Comparator<AC, Disabled>;
}

macro_rules! impl_comparator {
    ($COMP:ty, $comp:ident) => {
        impl ComparatorExt<$COMP> for $COMP {
            fn comparator<P: PositiveInput<$COMP>, N: NegativeInput<$COMP>>(
                self,
                positive_input: P,
                negative_input: N,
                config: Config,
            ) -> Comparator<$COMP, Disabled> {
                self.ctrla().modify(|_, w| {
                    let w = w.hysmode().set(config.hysteresis as u8);
                    // Only the 1-series comparators have the LPMODE bit.
                    #[cfg(feature = "series-1")]
                    let w = w.lpmode().bit(config.low_power_mode);
                    w
                });

                self.muxctrla()
                    .modify(|_, w| w.invert().bit(config.inverted));
                positive_input.setup(&self);
                negative_input.setup(&self);

                Comparator {
                    regs: self,
                    _enabled: PhantomData,
                }
            }
        }

        impl Comparator<$COMP, Disabled> {
            /// Initializes a comparator
            pub fn $comp<P: PositiveInput<$COMP>, N: NegativeInput<$COMP>>(
                comp: $COMP,
                positive_input: P,
                negative_input: N,
                config: Config,
            ) -> Self {
                comp.comparator(positive_input, negative_input, config)
            }

            /// Enables the comparator
            pub fn enable(self) -> Comparator<$COMP, Enabled> {
                self.regs.ctrla().modify(|_, w| w.enable().set_bit());
                // A CMP flag left over from a previous enable period (or from
                // the output settling on enable) would fire a spurious
                // interrupt the moment `listen` is active. Discard it so only
                // edges from here on are reported.
                self.regs.status().write(|w| w.cmp().set_bit());
                Comparator {
                    regs: self.regs,
                    _enabled: PhantomData,
                }
            }

            /// Enables raising the comparator interrupt at the specified output signal edge
            #[inline]
            pub fn listen(&self, mode: InterruptMode) {
                self.regs
                    .ctrla()
                    .modify(|_, w| unsafe { w.intmode().bits(mode as u8) });
                self.regs.intctrl().write(|w| w.cmp().set_bit());
            }
        }

        impl Comparator<$COMP, Enabled> {
            /// Returns the value of the output of the comparator
            #[inline]
            pub fn output(&self) -> bool {
                self.regs.status().read().state().bit_is_set()
            }

            /// Disables the comparator
            pub fn disable(self) -> Comparator<$COMP, Disabled> {
                self.regs.ctrla().modify(|_, w| w.enable().clear_bit());
                Comparator {
                    regs: self.regs,
                    _enabled: PhantomData,
                }
            }
        }

        impl<ED> Comparator<$COMP, ED> {
            /// Disables raising interrupts for the output signal
            #[inline]
            pub fn unlisten(&self) {
                self.regs.intctrl().modify(|_, w| w.cmp().clear_bit());
            }

            /// Returns `true` if the output signal interrupt is pending
            #[inline]
            pub fn is_pending(&self) -> bool {
                self.regs.status().read().cmp().bit_is_set()
            }

            /// Unpends the output signal interrupt
            #[inline]
            pub fn unpend(&self) {
                // Plain write, not read-modify-write: on a write-1-to-clear
                // register an RMW would also clear any other flag that became
                // pending between the read and the write.
                self.regs.status().write(|w| w.cmp().set_bit());
            }

            /// Configures a GPIO pin to output the signal of the comparator
            #[inline]
            pub fn output_pin<P: ComparatorOutput<$COMP>>(&self, pin: P) {
                pin.setup(&self.regs);
            }
        }
    };
}

pub trait NegativeInput<AC>: crate::private::Sealed {
    fn setup(&self, comp: &AC);
}
pub trait PositiveInput<AC>: crate::private::Sealed {
    fn setup(&self, comp: &AC);
}
pub trait ComparatorOutput<AC>: crate::private::Sealed {
    fn setup(&self, comp: &AC);
}

macro_rules! positive_input_pin {
    ($COMP:ident, $pin:ty, $variant:expr) => {
        impl PositiveInput<$COMP> for $pin {
            #[inline]
            fn setup(&self, comp: &$COMP) {
                comp.muxctrla().modify(|_, w| w.muxpos().variant($variant));
            }
        }
    };
}

macro_rules! negative_input_pin {
    ($COMP:ident, $pin:ty, $variant:expr) => {
        impl NegativeInput<$COMP> for $pin {
            #[inline]
            fn setup(&self, comp: &$COMP) {
                comp.muxctrla().modify(|_, w| w.muxneg().variant($variant));
            }
        }
    };
}

macro_rules! output_pin {
    ($COMP:ident, $pin:ty) => {
        impl ComparatorOutput<$COMP> for $pin {
            #[inline]
            fn setup(&self, comp: &$COMP) {
                comp.ctrla().modify(|_, w| w.outen().set_bit());
            }
        }
    };
}

macro_rules! refint_input {
    ($COMP:ident, $reft:ty, $variant:expr) => {
        impl NegativeInput<$COMP> for $reft {
            #[inline]
            fn setup(&self, comp: &$COMP) {
                comp.muxctrla().modify(|_, w| w.muxneg().variant($variant));
            }
        }
    };
}

// ================================================================================
// AC0 (all parts)
// ================================================================================
//
// Pin tables from the datasheet I/O-multiplexing chapter, cross-checked
// with the ATDF `<signals>` sections: P0/N0 (PA7/PA6) exist everywhere;
// the output pin is PA3 on the 8-pin parts and PA5 elsewhere; P1/N1
// (PB5/PB4) require a 20/24-pin package; P2 (PB1) exists on the 16 KB+
// 1-series parts, P3 (PB6) on their 24-pin variants only.

impl_comparator!(AC0, ac0);

positive_input_pin!(
    AC0,
    crate::gpio::porta::PA7<Analog>,
    crate::pac::ac0::muxctrla::MUXPOS_A::PIN0
);
#[cfg(any(feature = "pins-20", feature = "pins-24"))]
positive_input_pin!(
    AC0,
    crate::gpio::portb::PB5<Analog>,
    crate::pac::ac0::muxctrla::MUXPOS_A::PIN1
);
#[cfg(feature = "periph-ac1")]
positive_input_pin!(
    AC0,
    crate::gpio::portb::PB1<Analog>,
    crate::pac::ac0::muxctrla::MUXPOS_A::PIN2
);
#[cfg(all(feature = "periph-ac1", feature = "pins-24"))]
positive_input_pin!(
    AC0,
    crate::gpio::portb::PB6<Analog>,
    crate::pac::ac0::muxctrla::MUXPOS_A::PIN3
);

negative_input_pin!(
    AC0,
    crate::gpio::porta::PA6<Analog>,
    crate::pac::ac0::muxctrla::MUXNEG_A::PIN0
);
#[cfg(any(feature = "pins-20", feature = "pins-24"))]
negative_input_pin!(
    AC0,
    crate::gpio::portb::PB4<Analog>,
    crate::pac::ac0::muxctrla::MUXNEG_A::PIN1
);

#[cfg(feature = "periph-dac0")]
impl NegativeInput<AC0> for DACOutputToAC<0> {
    #[inline]
    fn setup(&self, comp: &AC0) {
        comp.muxctrla().modify(|_, w| w.muxneg().dac());
    }
}

#[cfg(feature = "pins-8")]
output_pin!(AC0, crate::gpio::porta::PA3<Output<Stateless>>);
#[cfg(not(feature = "pins-8"))]
output_pin!(AC0, crate::gpio::porta::PA5<Output<Stateless>>);

use crate::vref::DACReferenceVoltage;
refint_input!(
    AC0,
    DACReferenceVoltage<0>,
    crate::pac::ac0::muxctrla::MUXNEG_A::VREF
);

// ================================================================================
// AC1/AC2 (16 KB+ 1-series parts)
// ================================================================================
//
// Each comparator owns the reference channel of its equally-numbered
// internal DAC (DACn feeds ACn, the `refint_input` token is the matching
// `DACReferenceVoltage`). The additional P/N inputs on PB4-PB7 need the
// 24-pin package.

#[cfg(feature = "periph-ac1")]
mod ac1 {
    use super::*;
    use crate::pac::AC1;

    impl_comparator!(AC1, ac1);

    positive_input_pin!(
        AC1,
        crate::gpio::porta::PA7<Analog>,
        crate::pac::ac1::muxctrla::MUXPOS_A::PIN0
    );
    positive_input_pin!(
        AC1,
        crate::gpio::porta::PA6<Analog>,
        crate::pac::ac1::muxctrla::MUXPOS_A::PIN1
    );
    positive_input_pin!(
        AC1,
        crate::gpio::portb::PB0<Analog>,
        crate::pac::ac1::muxctrla::MUXPOS_A::PIN2
    );
    #[cfg(feature = "pins-24")]
    positive_input_pin!(
        AC1,
        crate::gpio::portb::PB4<Analog>,
        crate::pac::ac1::muxctrla::MUXPOS_A::PIN3
    );

    negative_input_pin!(
        AC1,
        crate::gpio::porta::PA5<Analog>,
        crate::pac::ac1::muxctrla::MUXNEG_A::PIN0
    );
    #[cfg(feature = "pins-24")]
    negative_input_pin!(
        AC1,
        crate::gpio::portb::PB7<Analog>,
        crate::pac::ac1::muxctrla::MUXNEG_A::PIN1
    );

    impl NegativeInput<AC1> for DACOutputToAC<1> {
        #[inline]
        fn setup(&self, comp: &AC1) {
            comp.muxctrla().modify(|_, w| w.muxneg().dac());
        }
    }

    output_pin!(AC1, crate::gpio::portb::PB3<Output<Stateless>>);

    refint_input!(
        AC1,
        DACReferenceVoltage<1>,
        crate::pac::ac1::muxctrla::MUXNEG_A::VREF
    );
}

#[cfg(feature = "periph-ac2")]
mod ac2 {
    use super::*;
    use crate::pac::AC2;

    impl_comparator!(AC2, ac2);

    positive_input_pin!(
        AC2,
        crate::gpio::porta::PA6<Analog>,
        crate::pac::ac2::muxctrla::MUXPOS_A::PIN0
    );
    positive_input_pin!(
        AC2,
        crate::gpio::portb::PB0<Analog>,
        crate::pac::ac2::muxctrla::MUXPOS_A::PIN1
    );
    #[cfg(feature = "pins-24")]
    positive_input_pin!(
        AC2,
        crate::gpio::portb::PB5<Analog>,
        crate::pac::ac2::muxctrla::MUXPOS_A::PIN2
    );
    #[cfg(feature = "pins-24")]
    positive_input_pin!(
        AC2,
        crate::gpio::portb::PB7<Analog>,
        crate::pac::ac2::muxctrla::MUXPOS_A::PIN3
    );

    negative_input_pin!(
        AC2,
        crate::gpio::porta::PA7<Analog>,
        crate::pac::ac2::muxctrla::MUXNEG_A::PIN0
    );
    #[cfg(feature = "pins-24")]
    negative_input_pin!(
        AC2,
        crate::gpio::portb::PB6<Analog>,
        crate::pac::ac2::muxctrla::MUXNEG_A::PIN1
    );

    impl NegativeInput<AC2> for DACOutputToAC<2> {
        #[inline]
        fn setup(&self, comp: &AC2) {
            comp.muxctrla().modify(|_, w| w.muxneg().dac());
        }
    }

    output_pin!(AC2, crate::gpio::portb::PB2<Output<Stateless>>);

    refint_input!(
        AC2,
        DACReferenceVoltage<2>,
        crate::pac::ac2::muxctrla::MUXNEG_A::VREF
    );
}

use crate::evsys::ChannelConfigurator;
use crate::evsys::{Channel, EventGenerator, GeneratorAssigned, Unconfigured};

// AC0 only: generator value 0x03 is AC0_OUT on every async channel.
// TODO: AC1/AC2 event generation — their generator values differ per
//       async channel (0x13/0x12/0x11/0x12 style), so they need a small
//       per-channel table instead of one constant.
impl<Evsys, Index> EventGenerator<Evsys, crate::evsys::Async, Index>
    for Comparator<AC0, Disabled>
where
    Evsys: crate::evsys::marker::Evsys,
    Index: crate::evsys::marker::Index,
{
    type EventSource = ();

    fn connect_event_generator(
        &mut self,
        mut channel: Channel<Evsys, crate::evsys::Async, Index, Unconfigured>,
        _source: (),
    ) -> Channel<Evsys, crate::evsys::Async, Index, GeneratorAssigned> {
        channel.set_generator(0x03);
        channel.with_state(GeneratorAssigned)
    }
}
