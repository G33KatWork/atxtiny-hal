//! # Port Multiplexer
//!
//! The PORTMUX peripheral selects which of several possible pin positions
//! each peripheral function is routed to. [`PortmuxExt::constrain`] splits
//! it into one zero-sized routing token per independently routable function
//! (the fields of [`Portmux`]), and [`IntoMuxedPinset::mux`] consumes the
//! pinset's pins *and* the matching token.
//!
//! Consuming the token is what makes conflicting pinsets unrepresentable:
//! if `mux` merely borrowed the PORTMUX, a second pinset for the same
//! function could later flip the routing bit and silently disconnect the
//! first pinset in hardware while its driver keeps "working" in the type
//! system.

use embedded_hal::digital::PinState;

/// Extension trait that constrains the [`crate::pac::PORTMUX`] peripheral
pub trait PortmuxExt {
    /// Constrains the [`pac::PORTMUX`] peripheral.
    ///
    /// Consumes the [`pac::PORTMUX`] peripheral and splits it into one
    /// routing token per muxable peripheral function.
    ///
    /// [`pac::PORTMUX`]: `crate::pac::PORTMUX`
    fn constrain(self) -> Portmux;
}

macro_rules! mux_tokens {
    ($($(#[$meta:meta])* $field:ident: $Token:ident,)+) => {
        /// Routing tokens for the PORTMUX peripheral
        ///
        /// An instance of this struct is acquired by calling the
        /// [`constrain`](PortmuxExt::constrain) function on the
        /// [`PORTMUX`](crate::pac::PORTMUX) struct.
        ///
        /// ```
        /// let dp = pac::Peripherals::take().unwrap();
        /// let portmux = dp.PORTMUX.constrain();
        /// ```
        ///
        /// Each field is the token for one independently routable
        /// peripheral function; move it out and pass it to
        /// [`IntoMuxedPinset::mux`].
        ///
        /// Note: the routing writes are read-modify-writes on shared CTRLx
        /// registers without a critical section. Tokens are independently
        /// movable, so muxing from multiple execution contexts (main and
        /// ISRs) concurrently can lose updates — do the muxing during
        /// single-context initialization, which is the usual pattern.
        //
        // Non-exhaustive so that tokens for not-yet-supported routings
        // (TCA WO3-5, TCB1 on the larger devices) can be added without a
        // breaking change: users can move fields out but never construct
        // or exhaustively destructure the struct.
        #[non_exhaustive]
        pub struct Portmux {
            $($(#[$meta])* pub $field: $Token,)+
        }

        $(
            $(#[$meta])*
            ///
            /// Zero-sized ownership token; see [`Portmux`].
            pub struct $Token {
                _private: (),
            }

            impl $Token {
                // Register access for the mux impls. The token is the sole
                // owner of its routing field: `constrain` consumed the PAC
                // singleton and hands out each token exactly once.
                pub(crate) fn regs(&self) -> &crate::pac::portmux::RegisterBlock {
                    unsafe { &*crate::pac::PORTMUX::ptr() }
                }
            }

            impl crate::private::Sealed for $Token {}
        )+

        impl PortmuxExt for crate::pac::PORTMUX {
            fn constrain(self) -> Portmux {
                Portmux {
                    $($field: $Token { _private: () },)+
                }
            }
        }
    };
}

mux_tokens! {
    /// Routing token for the USART0 pins (RXD/TXD/XCK/XDIR)
    usart0: Usart0Mux,
    /// Routing token for the TWI0 pins (SDA/SCL)
    twi0: Twi0Mux,
    /// Routing token for the SPI0 pins (MOSI/MISO/SCK/SS)
    spi0: Spi0Mux,
    /// Routing token for the CCL LUT0 output pin
    lut0: Lut0Mux,
    /// Routing token for the CCL LUT1 output pin
    lut1: Lut1Mux,
    /// Routing token for the TCA0 waveform output 0 pin
    tca0_wo0: Tca0Wo0Mux,
    /// Routing token for the TCA0 waveform output 1 pin
    tca0_wo1: Tca0Wo1Mux,
    /// Routing token for the TCA0 waveform output 2 pin
    tca0_wo2: Tca0Wo2Mux,
    /// Routing token for the TCB0 waveform output pin
    tcb0: Tcb0Mux,
    /// Enable token for the event system output 0 pin
    evout0: Evout0Mux,
    /// Enable token for the event system output 1 pin
    evout1: Evout1Mux,
    /// Enable token for the event system output 2 pin
    evout2: Evout2Mux,
}

/// Trait implemented by pinsets that can be muxed onto physical pins.
///
/// The actual muxing happens when calling the [`IntoMuxedPinset::mux`] method
/// on a defined pinset, consuming the routing token for the targeted
/// peripheral function
///
/// ```
/// let dp = pac::Peripherals::take().unwrap();
/// let portmux = dp.PORTMUX.constrain();
/// let porta = dp.PORTA.split();
///
/// let rxpin = porta.pa2.into_peripheral::<pac::USART0>();
/// let txpin = porta.pa1.into_peripheral::<pac::USART0>();
///
/// let usart_pair = (rxpin, txpin);
/// let usart_pair = usart_pair.mux(portmux.usart0);
/// ```
pub trait IntoMuxedPinset<Peripheral> {
    /// The resulting pinset that is returned when the mux is configured to
    /// enable it.
    type Pinset;

    /// The routing token consumed by [`mux`](Self::mux).
    type Token;

    /// Setup the hardware to enable the multiplexing of this pinset.
    ///
    /// Consumes the per-function routing token, so only one live pinset can
    /// exist per peripheral function.
    ///
    /// Calling this function may also reconfigure GPIO input or output modes
    /// and set pin levels if needed.
    fn mux(self, token: Self::Token) -> Self::Pinset;
}

use crate::gpio::{Input, Output, Peripheral, Stateless};

// Serial
use crate::pac::USART0;
use crate::serial::UartPinset;

impl IntoMuxedPinset<USART0>
    for (
        crate::gpio::portb::PB3<Peripheral<USART0>>,
        crate::gpio::portb::PB2<Peripheral<USART0>>,
    )
{
    type Pinset = UartPinset<
        USART0,
        crate::gpio::portb::PB3<Input>,
        crate::gpio::portb::PB2<Output<Stateless>>,
    >;

    type Token = Usart0Mux;

    fn mux(self, token: Usart0Mux) -> Self::Pinset {
        token.regs().ctrlb().modify(|_r, w| w.usart0().clear_bit());
        // Drive the TX pin at the idle level (high) from the very first
        // cycle it becomes an output. Enabling the driver before setting the
        // level would emit a short low glitch — receivers mistake it for a
        // start bit and stay out of sync with the transmitter if the first
        // real data follows too quickly.
        let tx = self.1.into_stateless_push_pull_output_in_state(PinState::High);

        UartPinset::new(self.0.into_floating_input(), tx)
    }
}

impl IntoMuxedPinset<USART0>
    for (
        crate::gpio::porta::PA2<Peripheral<USART0>>,
        crate::gpio::porta::PA1<Peripheral<USART0>>,
    )
{
    type Pinset = UartPinset<
        USART0,
        crate::gpio::porta::PA2<Input>,
        crate::gpio::porta::PA1<Output<Stateless>>,
    >;

    type Token = Usart0Mux;

    fn mux(self, token: Usart0Mux) -> Self::Pinset {
        token.regs().ctrlb().modify(|_r, w| w.usart0().set_bit());
        // Drive the TX pin at the idle level (high) from the very first
        // cycle it becomes an output. Enabling the driver before setting the
        // level would emit a short low glitch — receivers mistake it for a
        // start bit and stay out of sync with the transmitter if the first
        // real data follows too quickly.
        let tx = self.1.into_stateless_push_pull_output_in_state(PinState::High);

        UartPinset::new(self.0.into_floating_input(), tx)
    }
}

// TWI
use crate::pac::TWI0;
use crate::twi::TwiPinset;

impl IntoMuxedPinset<TWI0>
    for (
        crate::gpio::portb::PB0<Peripheral<TWI0>>,
        crate::gpio::portb::PB1<Peripheral<TWI0>>,
    )
{
    type Pinset = TwiPinset<
        TWI0,
        crate::gpio::portb::PB0<Peripheral<TWI0>>,
        crate::gpio::portb::PB1<Peripheral<TWI0>>,
    >;

    type Token = Twi0Mux;

    fn mux(self, token: Twi0Mux) -> Self::Pinset {
        token.regs().ctrlb().modify(|_r, w| w.twi0().clear_bit());
        TwiPinset::new(self.0, self.1)
    }
}

impl IntoMuxedPinset<TWI0>
    for (
        crate::gpio::porta::PA2<Peripheral<TWI0>>,
        crate::gpio::porta::PA1<Peripheral<TWI0>>,
    )
{
    type Pinset = TwiPinset<
        TWI0,
        crate::gpio::porta::PA2<Peripheral<TWI0>>,
        crate::gpio::porta::PA1<Peripheral<TWI0>>,
    >;

    type Token = Twi0Mux;

    fn mux(self, token: Twi0Mux) -> Self::Pinset {
        token.regs().ctrlb().modify(|_r, w| w.twi0().set_bit());
        TwiPinset::new(self.0, self.1)
    }
}

// SPI
use crate::pac::SPI0;
use crate::spi::SpiPinset;

impl IntoMuxedPinset<SPI0>
    for (
        crate::gpio::porta::PA3<Peripheral<SPI0>>,
        crate::gpio::porta::PA2<Peripheral<SPI0>>,
        crate::gpio::porta::PA1<Peripheral<SPI0>>,
    )
{
    type Pinset = SpiPinset<
        SPI0,
        crate::gpio::porta::PA3<Output<Stateless>>,
        crate::gpio::porta::PA2<Input>,
        crate::gpio::porta::PA1<Output<Stateless>>,
    >;

    type Token = Spi0Mux;

    fn mux(self, token: Spi0Mux) -> Self::Pinset {
        token.regs().ctrlb().modify(|_r, w| w.spi0().clear_bit());
        // Turn the pins into stateless outputs
        // In SPI host mode, this hands over the pin to the SPI peripheral
        SpiPinset::new(
            self.0.into_stateless_push_pull_output(),
            self.1.into_floating_input(),
            self.2.into_stateless_push_pull_output(),
        )
    }
}

impl IntoMuxedPinset<SPI0>
    for (
        crate::gpio::portc::PC0<Peripheral<SPI0>>,
        crate::gpio::portc::PC1<Peripheral<SPI0>>,
        crate::gpio::portc::PC2<Peripheral<SPI0>>,
    )
{
    type Pinset = SpiPinset<
        SPI0,
        crate::gpio::portc::PC0<Output<Stateless>>,
        crate::gpio::portc::PC1<Input>,
        crate::gpio::portc::PC2<Output<Stateless>>,
    >;

    type Token = Spi0Mux;

    fn mux(self, token: Spi0Mux) -> Self::Pinset {
        token.regs().ctrlb().modify(|_r, w| w.spi0().set_bit());
        // Turn the pins into stateless outputs
        // In SPI host mode, this hands over the pin to the SPI peripheral
        SpiPinset::new(
            self.0.into_stateless_push_pull_output(),
            self.1.into_floating_input(),
            self.2.into_stateless_push_pull_output(),
        )
    }
}

// CCL
use crate::ccl::{CclLutOutputPinset, LUT0, LUT1};

impl IntoMuxedPinset<LUT0> for crate::gpio::porta::PA4<Output<Stateless>> {
    type Pinset = CclLutOutputPinset<LUT0, crate::gpio::porta::PA4<Output<Stateless>>>;

    type Token = Lut0Mux;

    fn mux(self, token: Lut0Mux) -> Self::Pinset {
        token.regs().ctrla().modify(|_r, w| w.lut0().clear_bit());
        CclLutOutputPinset::new(self)
    }
}

impl IntoMuxedPinset<LUT0> for crate::gpio::portb::PB4<Output<Stateless>> {
    type Pinset = CclLutOutputPinset<LUT0, crate::gpio::portb::PB4<Output<Stateless>>>;

    type Token = Lut0Mux;

    fn mux(self, token: Lut0Mux) -> Self::Pinset {
        token.regs().ctrla().modify(|_r, w| w.lut0().set_bit());
        CclLutOutputPinset::new(self)
    }
}

impl IntoMuxedPinset<LUT1> for crate::gpio::porta::PA7<Output<Stateless>> {
    type Pinset = CclLutOutputPinset<LUT1, crate::gpio::porta::PA7<Output<Stateless>>>;

    type Token = Lut1Mux;

    fn mux(self, token: Lut1Mux) -> Self::Pinset {
        token.regs().ctrla().modify(|_r, w| w.lut1().clear_bit());
        CclLutOutputPinset::new(self)
    }
}

impl IntoMuxedPinset<LUT1> for crate::gpio::portc::PC1<Output<Stateless>> {
    type Pinset = CclLutOutputPinset<LUT1, crate::gpio::portc::PC1<Output<Stateless>>>;

    type Token = Lut1Mux;

    fn mux(self, token: Lut1Mux) -> Self::Pinset {
        token.regs().ctrla().modify(|_r, w| w.lut1().set_bit());
        CclLutOutputPinset::new(self)
    }
}

// TCA
use crate::pac::TCA0;
use crate::timer::tca::TcaPinset;
use crate::timer::{C1, C2, C3};

impl IntoMuxedPinset<TCA0> for crate::gpio::portb::PB0<Output<Stateless>> {
    type Pinset = TcaPinset<TCA0, crate::gpio::portb::PB0<Output<Stateless>>, C1>;

    type Token = Tca0Wo0Mux;

    fn mux(self, token: Tca0Wo0Mux) -> Self::Pinset {
        token.regs().ctrlc().modify(|_r, w| w.tca00().clear_bit());
        TcaPinset::new(self)
    }
}

impl IntoMuxedPinset<TCA0> for crate::gpio::portb::PB1<Output<Stateless>> {
    type Pinset = TcaPinset<TCA0, crate::gpio::portb::PB1<Output<Stateless>>, C2>;

    type Token = Tca0Wo1Mux;

    fn mux(self, token: Tca0Wo1Mux) -> Self::Pinset {
        token.regs().ctrlc().modify(|_r, w| w.tca01().clear_bit());
        TcaPinset::new(self)
    }
}

impl IntoMuxedPinset<TCA0> for crate::gpio::portb::PB2<Output<Stateless>> {
    type Pinset = TcaPinset<TCA0, crate::gpio::portb::PB2<Output<Stateless>>, C3>;

    type Token = Tca0Wo2Mux;

    fn mux(self, token: Tca0Wo2Mux) -> Self::Pinset {
        token.regs().ctrlc().modify(|_r, w| w.tca02().clear_bit());
        TcaPinset::new(self)
    }
}

impl IntoMuxedPinset<TCA0> for crate::gpio::portb::PB3<Output<Stateless>> {
    type Pinset = TcaPinset<TCA0, crate::gpio::portb::PB3<Output<Stateless>>, C1>;

    type Token = Tca0Wo0Mux;

    fn mux(self, token: Tca0Wo0Mux) -> Self::Pinset {
        token.regs().ctrlc().modify(|_r, w| w.tca00().set_bit());
        TcaPinset::new(self)
    }
}

impl IntoMuxedPinset<TCA0> for crate::gpio::portb::PB4<Output<Stateless>> {
    type Pinset = TcaPinset<TCA0, crate::gpio::portb::PB4<Output<Stateless>>, C2>;

    type Token = Tca0Wo1Mux;

    fn mux(self, token: Tca0Wo1Mux) -> Self::Pinset {
        token.regs().ctrlc().modify(|_r, w| w.tca01().set_bit());
        TcaPinset::new(self)
    }
}

impl IntoMuxedPinset<TCA0> for crate::gpio::portb::PB5<Output<Stateless>> {
    type Pinset = TcaPinset<TCA0, crate::gpio::portb::PB5<Output<Stateless>>, C3>;

    type Token = Tca0Wo2Mux;

    fn mux(self, token: Tca0Wo2Mux) -> Self::Pinset {
        token.regs().ctrlc().modify(|_r, w| w.tca02().set_bit());
        TcaPinset::new(self)
    }
}

// TCB 8 Bit PWM outputs
use crate::pac::TCB0;
use crate::timer::{tcb::TcbPinset, tcb_8bit::TCB8Bit};

impl IntoMuxedPinset<TCB0> for crate::gpio::porta::PA5<Output<Stateless>> {
    type Pinset = TcbPinset<TCB8Bit, crate::gpio::porta::PA5<Output<Stateless>>, C1>;

    type Token = Tcb0Mux;

    fn mux(self, token: Tcb0Mux) -> Self::Pinset {
        token.regs().ctrld().modify(|_r, w| w.tcb0().clear_bit());
        TcbPinset::new(self)
    }
}

impl IntoMuxedPinset<TCB0> for crate::gpio::portc::PC0<Output<Stateless>> {
    type Pinset = TcbPinset<TCB8Bit, crate::gpio::portc::PC0<Output<Stateless>>, C1>;

    type Token = Tcb0Mux;

    fn mux(self, token: Tcb0Mux) -> Self::Pinset {
        token.regs().ctrld().modify(|_r, w| w.tcb0().set_bit());
        TcbPinset::new(self)
    }
}

/// Routing control for the event output pins
///
/// Implemented by [`Evout0Mux`]/[`Evout1Mux`]/[`Evout2Mux`] so the EVOUT
/// pinset can switch the pin routing off again when it is freed.
pub trait EvoutRouting: crate::private::Sealed {
    /// Enable or disable this event output's pin routing.
    ///
    /// Internal — driven by the mux/free lifecycle; calling it while an
    /// [`EventOutputPinset`](crate::evout::EventOutputPinset) is live would
    /// desynchronize the type state from the hardware.
    #[doc(hidden)]
    fn set_routing(&self, enable: bool);
}

macro_rules! evout_routing {
    ($($Token:ident => $bit:ident,)+) => {$(
        impl EvoutRouting for $Token {
            fn set_routing(&self, enable: bool) {
                self.regs().ctrla().modify(|_r, w| w.$bit().bit(enable));
            }
        }
    )+};
}

evout_routing! {
    Evout0Mux => evout0,
    Evout1Mux => evout1,
    Evout2Mux => evout2,
}

// EVOUT
use crate::evout::EventOutputPinset;
use crate::evout::{EVOUT0, EVOUT1, EVOUT2};
use crate::pac::EVSYS;

impl IntoMuxedPinset<EVSYS> for crate::gpio::porta::PA2<Peripheral<EVSYS>> {
    type Pinset = EventOutputPinset<EVSYS, crate::gpio::porta::PA2<Peripheral<EVSYS>>, Evout0Mux, EVOUT0>;

    type Token = Evout0Mux;

    fn mux(self, token: Evout0Mux) -> Self::Pinset {
        token.set_routing(true);
        EventOutputPinset::new(self, token)
    }
}

impl IntoMuxedPinset<EVSYS> for crate::gpio::portb::PB2<Peripheral<EVSYS>> {
    type Pinset = EventOutputPinset<EVSYS, crate::gpio::portb::PB2<Peripheral<EVSYS>>, Evout1Mux, EVOUT1>;

    type Token = Evout1Mux;

    fn mux(self, token: Evout1Mux) -> Self::Pinset {
        token.set_routing(true);
        EventOutputPinset::new(self, token)
    }
}

impl IntoMuxedPinset<EVSYS> for crate::gpio::portc::PC2<Peripheral<EVSYS>> {
    type Pinset = EventOutputPinset<EVSYS, crate::gpio::portc::PC2<Peripheral<EVSYS>>, Evout2Mux, EVOUT2>;

    type Token = Evout2Mux;

    fn mux(self, token: Evout2Mux) -> Self::Pinset {
        token.set_routing(true);
        EventOutputPinset::new(self, token)
    }
}
