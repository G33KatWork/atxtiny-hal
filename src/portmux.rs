//! # Port Multiplexer

// FIXME: Do we really need a constrained peripheral here? We could just get the
//        pointer to the PORTMUX in every `mux` implementation and work with that.
//        This also alleviates the need to pass a reference to it around.

use embedded_hal::digital::OutputPin;

/// Extension trait that constrains the [`crate::pac::Portmux`] peripheral
pub trait PortmuxExt {
    /// Constrains the [`pac::PORTMUX`] peripheral.
    ///
    /// Consumes the [`pac::PORTMUX`] peripheral and converts it to a [`HAL`] internal type
    /// constraining it's public access surface to fit the design of the `HAL`.
    ///
    /// [`pac::PORTMUX`]: `crate::pac::Portmux`
    /// [`HAL`]: `crate`
    fn constrain(self) -> Portmux;
}

/// Constrained Portmux peripheral
///
/// An instance of this struct is acquired by calling the [`constrain`](PortmuxExt::constrain) function
/// on the [`PORTMUX`] struct.
///
/// ```
/// let dp = pac::Peripherals::take().unwrap();
/// let portmux = dp.portmux.constrain();
/// ```
pub struct Portmux {
    mux: crate::pac::Portmux,
}

impl PortmuxExt for crate::pac::Portmux {
    fn constrain(self) -> Portmux {
        Portmux { mux: self }
    }
}

/// Trait implemented by pinsets that can be muxed onto physical pins.
///
/// The actual muxing happens when calling the [`IntoMuxedPinset::mux`] method
/// on a defined pinset
///
/// ```
/// let dp = pac::Peripherals::take().unwrap();
/// let portmux = dp.portmux.constrain();
/// let porta = dp.PORTA.split();
///
/// let rxpin = porta.pa2.into_peripheral::<pac::USART0>();
/// let txpin = porta.pa1.into_peripheral::<pac::USART0>();
///
/// let usart_pair = (rxpin, txpin);
/// let usart_pair = usart_pair.mux(&portmux);
/// ```
pub trait IntoMuxedPinset<Peripheral> {
    /// The resulting pinset that is returned when the mux is configure to
    /// enable it.
    type Pinset;

    /// Setup the hardware to enable the multiplexing of this pinset.
    ///
    /// Calling this function may also reconfigure GPIO input or output modes
    /// and set pin levels if needed.
    fn mux(self, portmux: &Portmux) -> Self::Pinset;
}

use crate::gpio::{Input, Output, Peripheral, Stateless};

// Serial
use crate::pac::Usart0;
use crate::serial::UartPinset;

impl IntoMuxedPinset<Usart0>
    for (
        crate::gpio::portb::PB3<Peripheral<Usart0>>,
        crate::gpio::portb::PB2<Peripheral<Usart0>>,
    )
{
    type Pinset = UartPinset<
        Usart0,
        crate::gpio::portb::PB3<Input>,
        crate::gpio::portb::PB2<Output<Stateless>>,
    >;

    fn mux(self, portmux: &Portmux) -> Self::Pinset {
        portmux.mux.ctrlb().modify(|_r, w| w.usart0().clear_bit());
        let mut tx = self.1.into_stateless_push_pull_output();

        // Set the TX pin high to turn switch it to idle level
        // Otherwise receivers might mistake the low level as a start bit and if
        // not enough time passes between init and the first data to be sent, the
        // receiver becomes confused because it's not in sync with the transmitter
        // anymore
        tx.set_high().unwrap();

        UartPinset::new(self.0.into_floating_input(), tx)
    }
}

impl IntoMuxedPinset<Usart0>
    for (
        crate::gpio::porta::PA2<Peripheral<Usart0>>,
        crate::gpio::porta::PA1<Peripheral<Usart0>>,
    )
{
    type Pinset = UartPinset<
        Usart0,
        crate::gpio::porta::PA2<Input>,
        crate::gpio::porta::PA1<Output<Stateless>>,
    >;

    fn mux(self, portmux: &Portmux) -> Self::Pinset {
        portmux.mux.ctrlb().modify(|_r, w| w.usart0().set_bit());
        let mut tx = self.1.into_stateless_push_pull_output();

        // Set the TX pin high to turn switch it to idle level
        // Otherwise receivers might mistake the low level as a start bit and if
        // not enough time passes between init and the first data to be sent, the
        // receiver becomes confused because it's not in sync with the transmitter
        // anymore
        tx.set_high().unwrap();

        UartPinset::new(self.0.into_floating_input(), tx)
    }
}

// TWI
use crate::pac::Twi0;
use crate::twi::TwiPinset;

impl IntoMuxedPinset<Twi0>
    for (
        crate::gpio::portb::PB0<Peripheral<Twi0>>,
        crate::gpio::portb::PB1<Peripheral<Twi0>>,
    )
{
    type Pinset = TwiPinset<
        Twi0,
        crate::gpio::portb::PB0<Peripheral<Twi0>>,
        crate::gpio::portb::PB1<Peripheral<Twi0>>,
    >;

    fn mux(self, portmux: &Portmux) -> Self::Pinset {
        portmux.mux.ctrlb().modify(|_r, w| w.twi0().clear_bit());
        TwiPinset::new(self.0, self.1)
    }
}

impl IntoMuxedPinset<Twi0>
    for (
        crate::gpio::porta::PA2<Peripheral<Twi0>>,
        crate::gpio::porta::PA1<Peripheral<Twi0>>,
    )
{
    type Pinset = TwiPinset<
        Twi0,
        crate::gpio::porta::PA2<Peripheral<Twi0>>,
        crate::gpio::porta::PA1<Peripheral<Twi0>>,
    >;

    fn mux(self, portmux: &Portmux) -> Self::Pinset {
        portmux.mux.ctrlb().modify(|_r, w| w.twi0().set_bit());
        TwiPinset::new(self.0, self.1)
    }
}

// SPI
use crate::pac::Spi0;
use crate::spi::SpiPinset;

impl IntoMuxedPinset<Spi0>
    for (
        crate::gpio::porta::PA3<Peripheral<Spi0>>,
        crate::gpio::porta::PA2<Peripheral<Spi0>>,
        crate::gpio::porta::PA1<Peripheral<Spi0>>,
    )
{
    type Pinset = SpiPinset<
        Spi0,
        crate::gpio::porta::PA3<Output<Stateless>>,
        crate::gpio::porta::PA2<Input>,
        crate::gpio::porta::PA1<Output<Stateless>>,
    >;

    fn mux(self, portmux: &Portmux) -> Self::Pinset {
        portmux.mux.ctrlb().modify(|_r, w| w.spi0().clear_bit());
        // Turn the pins into stateless outputs
        // In SPI host mode, this hands over the pin to the SPI peripheral
        SpiPinset::new(
            self.0.into_stateless_push_pull_output(),
            self.1.into_floating_input(),
            self.2.into_stateless_push_pull_output(),
        )
    }
}

impl IntoMuxedPinset<Spi0>
    for (
        crate::gpio::portc::PC0<Peripheral<Spi0>>,
        crate::gpio::portc::PC1<Peripheral<Spi0>>,
        crate::gpio::portc::PC2<Peripheral<Spi0>>,
    )
{
    type Pinset = SpiPinset<
        Spi0,
        crate::gpio::portc::PC0<Output<Stateless>>,
        crate::gpio::portc::PC1<Input>,
        crate::gpio::portc::PC2<Output<Stateless>>,
    >;

    fn mux(self, portmux: &Portmux) -> Self::Pinset {
        portmux.mux.ctrlb().modify(|_r, w| w.spi0().set_bit());
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

    fn mux(self, portmux: &Portmux) -> Self::Pinset {
        portmux.mux.ctrla().modify(|_r, w| w.lut0().clear_bit());
        CclLutOutputPinset::new(self)
    }
}

impl IntoMuxedPinset<LUT0> for crate::gpio::portb::PB4<Output<Stateless>> {
    type Pinset = CclLutOutputPinset<LUT0, crate::gpio::portb::PB4<Output<Stateless>>>;

    fn mux(self, portmux: &Portmux) -> Self::Pinset {
        portmux.mux.ctrla().modify(|_r, w| w.lut0().set_bit());
        CclLutOutputPinset::new(self)
    }
}

impl IntoMuxedPinset<LUT1> for crate::gpio::porta::PA7<Output<Stateless>> {
    type Pinset = CclLutOutputPinset<LUT1, crate::gpio::porta::PA7<Output<Stateless>>>;

    fn mux(self, portmux: &Portmux) -> Self::Pinset {
        portmux.mux.ctrla().modify(|_r, w| w.lut1().clear_bit());
        CclLutOutputPinset::new(self)
    }
}

impl IntoMuxedPinset<LUT1> for crate::gpio::portc::PC1<Output<Stateless>> {
    type Pinset = CclLutOutputPinset<LUT1, crate::gpio::portc::PC1<Output<Stateless>>>;

    fn mux(self, portmux: &Portmux) -> Self::Pinset {
        portmux.mux.ctrla().modify(|_r, w| w.lut1().set_bit());
        CclLutOutputPinset::new(self)
    }
}

// TCA
use crate::pac::Tca0;
use crate::timer::tca::TcaPinset;
use crate::timer::{C1, C2, C3};

impl IntoMuxedPinset<Tca0> for crate::gpio::portb::PB0<Output<Stateless>> {
    type Pinset = TcaPinset<Tca0, crate::gpio::portb::PB0<Output<Stateless>>, C1>;

    fn mux(self, portmux: &Portmux) -> Self::Pinset {
        portmux.mux.ctrlc().modify(|_r, w| w.tca00().clear_bit());
        TcaPinset::new(self)
    }
}

impl IntoMuxedPinset<Tca0> for crate::gpio::portb::PB1<Output<Stateless>> {
    type Pinset = TcaPinset<Tca0, crate::gpio::portb::PB1<Output<Stateless>>, C2>;

    fn mux(self, portmux: &Portmux) -> Self::Pinset {
        portmux.mux.ctrlc().modify(|_r, w| w.tca01().clear_bit());
        TcaPinset::new(self)
    }
}

impl IntoMuxedPinset<Tca0> for crate::gpio::portb::PB2<Output<Stateless>> {
    type Pinset = TcaPinset<Tca0, crate::gpio::portb::PB2<Output<Stateless>>, C3>;

    fn mux(self, portmux: &Portmux) -> Self::Pinset {
        portmux.mux.ctrlc().modify(|_r, w| w.tca02().clear_bit());
        TcaPinset::new(self)
    }
}

impl IntoMuxedPinset<Tca0> for crate::gpio::portb::PB3<Output<Stateless>> {
    type Pinset = TcaPinset<Tca0, crate::gpio::portb::PB3<Output<Stateless>>, C1>;

    fn mux(self, portmux: &Portmux) -> Self::Pinset {
        portmux.mux.ctrlc().modify(|_r, w| w.tca00().set_bit());
        TcaPinset::new(self)
    }
}

impl IntoMuxedPinset<Tca0> for crate::gpio::portb::PB4<Output<Stateless>> {
    type Pinset = TcaPinset<Tca0, crate::gpio::portb::PB4<Output<Stateless>>, C2>;

    fn mux(self, portmux: &Portmux) -> Self::Pinset {
        portmux.mux.ctrlc().modify(|_r, w| w.tca01().set_bit());
        TcaPinset::new(self)
    }
}

impl IntoMuxedPinset<Tca0> for crate::gpio::portb::PB5<Output<Stateless>> {
    type Pinset = TcaPinset<Tca0, crate::gpio::portb::PB5<Output<Stateless>>, C3>;

    fn mux(self, portmux: &Portmux) -> Self::Pinset {
        portmux.mux.ctrlc().modify(|_r, w| w.tca02().set_bit());
        TcaPinset::new(self)
    }
}

// TCB 8 Bit PWM outputs
use crate::pac::Tcb0;
use crate::timer::{tcb::TcbPinset, tcb_8bit::TCB8Bit};

impl IntoMuxedPinset<Tcb0> for crate::gpio::porta::PA5<Output<Stateless>> {
    type Pinset = TcbPinset<TCB8Bit, crate::gpio::porta::PA5<Output<Stateless>>, C1>;

    fn mux(self, portmux: &Portmux) -> Self::Pinset {
        portmux.mux.ctrld().modify(|_r, w| w.tcb0().clear_bit());
        TcbPinset::new(self)
    }
}

impl IntoMuxedPinset<Tcb0> for crate::gpio::portc::PC0<Output<Stateless>> {
    type Pinset = TcbPinset<TCB8Bit, crate::gpio::portc::PC0<Output<Stateless>>, C1>;

    fn mux(self, portmux: &Portmux) -> Self::Pinset {
        portmux.mux.ctrld().modify(|_r, w| w.tcb0().set_bit());
        TcbPinset::new(self)
    }
}

// EVOUT
use crate::evout::EventOutputPinset;
use crate::evout::{EVOUT0, EVOUT1, EVOUT2};
use crate::pac::Evsys;

impl IntoMuxedPinset<Evsys> for crate::gpio::porta::PA2<Peripheral<Evsys>> {
    type Pinset = EventOutputPinset<Evsys, crate::gpio::porta::PA2<Peripheral<Evsys>>, EVOUT0>;

    fn mux(self, portmux: &Portmux) -> Self::Pinset {
        portmux.mux.ctrla().modify(|_r, w| w.evout0().set_bit());
        EventOutputPinset::new(self)
    }
}

impl IntoMuxedPinset<Evsys> for crate::gpio::portb::PB2<Peripheral<Evsys>> {
    type Pinset = EventOutputPinset<Evsys, crate::gpio::portb::PB2<Peripheral<Evsys>>, EVOUT1>;

    fn mux(self, portmux: &Portmux) -> Self::Pinset {
        portmux.mux.ctrla().modify(|_r, w| w.evout1().set_bit());
        EventOutputPinset::new(self)
    }
}

impl IntoMuxedPinset<Evsys> for crate::gpio::portc::PC2<Peripheral<Evsys>> {
    type Pinset = EventOutputPinset<Evsys, crate::gpio::portc::PC2<Peripheral<Evsys>>, EVOUT2>;

    fn mux(self, portmux: &Portmux) -> Self::Pinset {
        portmux.mux.ctrla().modify(|_r, w| w.evout2().set_bit());
        EventOutputPinset::new(self)
    }
}
