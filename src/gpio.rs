//! # General Purpose Input / Output

// TODO: Emulate open drain outputs by toggling the direction?

use core::{convert::Infallible, marker::PhantomData};

use crate::{
    embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin},
    Toggle,
};

/// Extension trait to split a GPIO peripheral into independent pins and registers
pub trait GpioExt {
    /// The Parts to split the GPIO peripheral into
    type Parts;

    /// Splits the GPIO block into independent pins and registers
    fn split(self) -> Self::Parts;
}

/// GPIO Register interface traits private to this module
mod private {
    use super::Edge;

    pub trait GpioRegExt {
        fn is_low(&self, i: u8) -> bool;
        fn is_set_low(&self, i: u8) -> bool;
        fn set_high(&self, i: u8);
        fn set_low(&self, i: u8);
        fn toggle(&self, i: u8);

        fn input(&self, i: u8);
        fn output(&self, i: u8);

        fn floating(&self, i: u8);
        fn pull_up(&self, i: u8);

        fn normal(&self, i: u8);
        fn inverted(&self, i: u8);

        fn enable_input_buffer(&self, i: u8);
        fn disable_input_buffer(&self, i: u8);

        fn interrupt_pending(&self, i: u8) -> bool;
        fn clear_interrupt_pending(&self, i: u8);
        fn configure_interrupt(&self, i: u8, edge: Edge);
    }

    pub trait Gpio {
        type Reg: GpioRegExt + ?Sized;

        fn ptr(&self) -> *const Self::Reg;
        fn port_index(&self) -> u8;
    }
}

use embedded_hal::digital::{ErrorType, PinState};
use private::GpioRegExt;

/// Marker traits used in this module
pub mod marker {
    /// Marker trait for GPIO ports
    pub trait Gpio: super::private::Gpio {}

    /// Marker trait for compile time defined GPIO ports
    pub trait GpioStatic: Gpio {}

    /// Marker trait for pin number
    pub trait Index {
        #[doc(hidden)]
        fn index(&self) -> u8;
    }

    /// Marker trait for readable pin modes
    pub trait Readable {}

    /// Marker trait for active pins where either the input or the output driver is enabled
    pub trait Active {}

    /// Marker trait for pins which can have an enabled pull-up resistor
    pub trait Pullupable {}
}

/// Runtime defined GPIO port (type state)
#[derive(Debug)]
pub struct Gpiox {
    ptr: *const dyn GpioRegExt,
    index: u8,
}

// # SAFETY
// `Send` is not auto-implemented because of the raw `dyn GpioRegExt` pointer.
// Moving a pin to another execution context is sound because of unique
// ownership, not atomicity: every pin exists exactly once (`split()` hands
// each out a single time, pins are not `Clone`), so whichever context owns
// the pin is the only one touching its per-pin state. The registers shared
// between pins of a port (DIR/OUT/INTFLAGS) are only ever written through
// their single-write strobe registers (OUTSET/OUTCLR/OUTTGL/DIRSET/DIRCLR,
// W1C for INTFLAGS), which cannot lose updates from concurrent pin owners.
// The non-atomic read-modify-writes that do exist (PINCTRL, including the
// ISC interrupt configuration with its side effects) target the pin's *own*
// PINCTRL register, which no other pin instance accesses.
unsafe impl Send for Gpiox {}

// # SAFETY
// `Sync` is sound because everything reachable through `&Pin` is a
// side-effect-free read (IN via `is_low`, INTFLAGS via
// `is_interrupt_pending`); all writes — including the non-atomic PINCTRL
// read-modify-writes — require `&mut self` and are therefore exclusive.
unsafe impl Sync for Gpiox {}

impl private::Gpio for Gpiox {
    type Reg = dyn GpioRegExt;

    fn ptr(&self) -> *const Self::Reg {
        self.ptr
    }

    fn port_index(&self) -> u8 {
        self.index
    }
}

impl marker::Gpio for Gpiox {}

/// Runtime defined pin number (type state)
#[derive(ufmt::derive::uDebug, Debug)]
pub struct Ux(u8);

impl marker::Index for Ux {
    fn index(&self) -> u8 {
        self.0
    }
}

/// Compile time defined pin number (type state)
#[derive(ufmt::derive::uDebug, Debug)]
pub struct U<const X: u8>;

impl<const X: u8> marker::Index for U<X> {
    #[inline(always)]
    fn index(&self) -> u8 {
        X
    }
}

/// Input mode (type state)
#[derive(ufmt::derive::uDebug, Debug)]
pub struct Input;

/// Output mode (type state)
#[derive(Debug)]
pub struct Output<Statefulness>(PhantomData<Statefulness>);

/// Analog mode with disabled input buffer (type state)
#[derive(ufmt::derive::uDebug, Debug)]
pub struct Analog;

/// Peripheral mode with disabled input buffer (type state)
///
/// This mode is the same as [`Analog`], but for the right
/// sets of Pins the [`IntoMuxedPinset`] trait is implemented
/// to switch between sets of pins that are routed to different
/// peripherals
///
/// [`IntoMuxedPinset`]: crate::portmux::IntoMuxedPinset
pub struct Peripheral<PER> {
    _peripheral: PhantomData<PER>,
}

/// Stateful output (type state)
#[derive(ufmt::derive::uDebug, Debug)]
pub struct Stateful;

/// Stateless output (type state)
#[derive(ufmt::derive::uDebug, Debug)]
pub struct Stateless;

impl marker::Readable for Input {}
impl marker::Readable for Output<Stateful> {}
impl marker::Active for Input {}
impl marker::Pullupable for Input {}
impl<PER> marker::Pullupable for Peripheral<PER> {}
impl marker::Pullupable for Output<Stateful> {}
impl marker::Pullupable for Output<Stateless> {}
impl marker::Active for Output<Stateful> {}
impl marker::Active for Output<Stateless> {}

/// GPIO interrupt trigger edge selection
#[derive(ufmt::derive::uDebug, Debug, Copy, Clone, PartialEq, Eq)]
pub enum Edge {
    /// Rising edge of voltage
    Rising,
    /// Falling edge of voltage
    Falling,
    /// Rising and falling edge of voltage
    RisingFalling,
    /// Low level of voltage
    LowLevel,
}

/// Generic pin
#[derive(Debug)]
pub struct Pin<Gpio, Index, Mode> {
    pub(crate) gpio: Gpio,
    pub(crate) index: Index,
    _mode: PhantomData<Mode>,
}

// Make all GPIO peripheral trait extensions sealable.
impl<Gpio, Index, Mode> crate::private::Sealed for Pin<Gpio, Index, Mode> {}

/// Fully erased pin
///
/// This moves the pin type information to be known
/// at runtime, and erases the specific compile time type of the GPIO.
/// The only compile time information of the GPIO pin is it's Mode.
pub type PXx<Mode> = Pin<Gpiox, Ux, Mode>;

impl<Gpio, Mode, const X: u8> Pin<Gpio, U<X>, Mode> {
    /// Erases the pin number from the type
    ///
    /// This is useful when you want to collect the pins into an array where you
    /// need all the elements to have the same type
    pub fn downgrade(self) -> Pin<Gpio, Ux, Mode> {
        Pin {
            gpio: self.gpio,
            index: Ux(X),
            _mode: self._mode,
        }
    }
}

impl<Gpio, Mode> Pin<Gpio, Ux, Mode>
where
    Gpio: marker::GpioStatic,
    Gpio::Reg: 'static + Sized,
{
    /// Erases the port letter from the type
    ///
    /// This is useful when you want to collect the pins into an array where you
    /// need all the elements to have the same type
    pub fn downgrade(self) -> PXx<Mode> {
        PXx {
            gpio: Gpiox {
                ptr: self.gpio.ptr(),
                index: self.gpio.port_index(),
            },
            index: self.index,
            _mode: self._mode,
        }
    }
}

impl<Gpio, Index, Mode> Pin<Gpio, Index, Mode> {
    fn into_mode<NewMode>(self) -> Pin<Gpio, Index, NewMode> {
        Pin {
            gpio: self.gpio,
            index: self.index,
            _mode: PhantomData,
        }
    }
}

impl<Gpio, Index, Mode> Pin<Gpio, Index, Mode>
where
    Gpio: marker::GpioStatic,
    Index: marker::Index,
{
    /// Convenience method to configure the pin to operate as an input pin
    /// and set the internal resistor floating
    ///
    /// Note: like every mode transition, this rewrites the pin's ISC field,
    /// so a previously configured pin-change interrupt is silently disabled
    /// — call [`configure_interrupt`](Self::configure_interrupt) again
    /// afterwards if the interrupt should survive the transition.
    pub fn into_floating_input(self) -> Pin<Gpio, Index, Input> {
        unsafe { (*self.gpio.ptr()).enable_input_buffer(self.index.index()) }
        unsafe { (*self.gpio.ptr()).input(self.index.index()) }
        unsafe { (*self.gpio.ptr()).floating(self.index.index()) }
        self.into_mode()
    }

    /// Convenience method to configure the pin to operate as an input pin
    /// and set the internal resistor pull-up
    ///
    /// Note: like every mode transition, this rewrites the pin's ISC field,
    /// so a previously configured pin-change interrupt is silently disabled
    /// — call [`configure_interrupt`](Self::configure_interrupt) again
    /// afterwards if the interrupt should survive the transition.
    pub fn into_pull_up_input(self) -> Pin<Gpio, Index, Input> {
        unsafe { (*self.gpio.ptr()).enable_input_buffer(self.index.index()) }
        unsafe { (*self.gpio.ptr()).input(self.index.index()) }
        unsafe { (*self.gpio.ptr()).pull_up(self.index.index()) }
        self.into_mode()
    }

    /// Configures the pin to operate as a stateful push-pull output pin
    ///
    /// Note: like every mode transition, this rewrites the pin's ISC field,
    /// so a previously configured pin-change interrupt is silently disabled
    /// — call [`configure_interrupt`](Self::configure_interrupt) again
    /// afterwards if the interrupt should survive the transition.
    pub fn into_push_pull_output(self) -> Pin<Gpio, Index, Output<Stateful>> {
        unsafe { (*self.gpio.ptr()).enable_input_buffer(self.index.index()) }
        unsafe { (*self.gpio.ptr()).output(self.index.index()) }
        self.into_mode()
    }

    /// Configures the pin to operate as a stateless push-pull output pin
    ///
    /// This is the same as a regular output, but with a disabled input buffer
    /// which means that the current value cannot be read back. Toggling still
    /// works — see [`toggle`](Self::toggle) — because the OUTTGL strobe flips
    /// the latch in hardware.
    ///
    /// Note: like every mode transition, this rewrites the pin's ISC field,
    /// so a previously configured pin-change interrupt is silently disabled
    /// — call [`configure_interrupt`](Self::configure_interrupt) again
    /// afterwards if the interrupt should survive the transition.
    pub fn into_stateless_push_pull_output(self) -> Pin<Gpio, Index, Output<Stateless>> {
        unsafe { (*self.gpio.ptr()).disable_input_buffer(self.index.index()) }
        unsafe { (*self.gpio.ptr()).output(self.index.index()) }
        self.into_mode()
    }

    /// Configures the pin to operate as a stateless push-pull output pin,
    /// driving `state` from the very first cycle of output mode
    ///
    /// [`into_stateless_push_pull_output`](Self::into_stateless_push_pull_output)
    /// enables the output driver with whatever level the OUT latch happens to
    /// hold (after reset: low); setting the level afterwards leaves a glitch
    /// of a few cycles at the stale level. This variant writes the OUT latch
    /// *before* switching the direction, so the pin only ever drives `state`
    /// — needed e.g. for a UART TX pin, where a low glitch looks like a start
    /// bit to receivers.
    pub fn into_stateless_push_pull_output_in_state(
        self,
        state: PinState,
    ) -> Pin<Gpio, Index, Output<Stateless>> {
        match state {
            PinState::High => unsafe { (*self.gpio.ptr()).set_high(self.index.index()) },
            PinState::Low => unsafe { (*self.gpio.ptr()).set_low(self.index.index()) },
        }
        self.into_stateless_push_pull_output()
    }

    /// Configures the pin to operate in an analog mode
    ///
    /// It is not strictly necessary to configure a pin into an analog mode,
    /// but the datasheet recommends to disable the input and output driver.
    ///
    /// Note: like every mode transition, this rewrites the pin's ISC field,
    /// so a previously configured pin-change interrupt is silently disabled
    /// — call [`configure_interrupt`](Self::configure_interrupt) again
    /// afterwards if the interrupt should survive the transition.
    pub fn into_analog_input(self) -> Pin<Gpio, Index, Analog> {
        unsafe { (*self.gpio.ptr()).disable_input_buffer(self.index.index()) }
        unsafe { (*self.gpio.ptr()).input(self.index.index()) }
        unsafe { (*self.gpio.ptr()).floating(self.index.index()) }
        self.into_mode()
    }

    /// Configures the pin to operate in a peripheral mode
    ///
    /// It is not strictly necessary to configure a pin into an peripheral mode,
    /// because the peripheral takes over anyway. But it is just nicer for the
    /// [`IntoMuxedPinset`] trait to have a dedicated pin mode for which we can
    /// implement it.
    ///
    /// Note: This mode is functionally equivalent to [`Analog`] in that it
    /// switches the pin into an input mode, disables the pull-ups and also
    /// disables the digital input buffer. However, pull-ups can be enabled
    /// afterwards in contrast to the analog mode.
    ///
    /// [`IntoMuxedPinset`]: crate::portmux::IntoMuxedPinset
    ///
    /// Note: like every mode transition, this rewrites the pin's ISC field,
    /// so a previously configured pin-change interrupt is silently disabled
    /// — call [`configure_interrupt`](Self::configure_interrupt) again
    /// afterwards if the interrupt should survive the transition.
    pub fn into_peripheral<PER>(self) -> Pin<Gpio, Index, Peripheral<PER>> {
        unsafe { (*self.gpio.ptr()).disable_input_buffer(self.index.index()) }
        unsafe { (*self.gpio.ptr()).input(self.index.index()) }
        unsafe { (*self.gpio.ptr()).floating(self.index.index()) }
        self.into_mode()
    }
}

impl<Gpio, Index, Mode> Pin<Gpio, Index, Mode>
where
    Gpio: marker::GpioStatic,
    Index: marker::Index,
    Mode: marker::Active,
{
    /// Set pin inversion for inputs or outputs
    pub fn invert_polarity(&mut self, invert: Toggle) {
        match invert {
            Toggle::On => unsafe { (*self.gpio.ptr()).inverted(self.index.index()) },
            Toggle::Off => unsafe { (*self.gpio.ptr()).normal(self.index.index()) },
        }
    }
}

impl<Gpio, Index, Otype> Pin<Gpio, Index, Output<Otype>>
where
    Gpio: marker::Gpio,
    Index: marker::Index,
{
    /// Toggle the output level via the hardware OUTTGL strobe
    ///
    /// Available for *stateless* outputs too: OUTTGL flips the OUT latch in
    /// hardware, so no readable input buffer is required. (For stateful
    /// outputs the [`StatefulOutputPin`] trait method does the same thing;
    /// this inherent method shadows it with identical behavior.)
    pub fn toggle(&mut self) -> Result<(), Infallible> {
        unsafe { (*self.gpio.ptr()).toggle(self.index.index()) }
        Ok(())
    }
}

impl<Gpio, Index, Itype> Pin<Gpio, Index, Itype>
where
    Gpio: marker::GpioStatic,
    Index: marker::Index,
    Itype: marker::Pullupable,
{
    /// Enables / disables the internal pull up on input pins
    pub fn internal_pull_up(&mut self, on: Toggle) {
        match on {
            Toggle::On => unsafe { (*self.gpio.ptr()).pull_up(self.index.index()) },
            Toggle::Off => unsafe { (*self.gpio.ptr()).floating(self.index.index()) },
        }
    }
}

impl<Gpio, Index, Mode> ErrorType for Pin<Gpio, Index, Mode>
where
    Gpio: marker::Gpio,
    Index: marker::Index,
{
    type Error = Infallible;
}

impl<Gpio, Index, Otype> OutputPin for Pin<Gpio, Index, Output<Otype>>
where
    Gpio: marker::Gpio,
    Index: marker::Index,
{
    fn set_high(&mut self) -> Result<(), Self::Error> {
        // NOTE(unsafe) atomic write to a stateless register
        unsafe { (*self.gpio.ptr()).set_high(self.index.index()) };
        Ok(())
    }

    fn set_low(&mut self) -> Result<(), Self::Error> {
        // NOTE(unsafe) atomic write to a stateless register
        unsafe { (*self.gpio.ptr()).set_low(self.index.index()) };
        Ok(())
    }
}

impl<Gpio, Index, Mode> InputPin for Pin<Gpio, Index, Mode>
where
    Gpio: marker::Gpio,
    Index: marker::Index,
    Mode: marker::Readable,
{
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        Ok(!self.is_low()?)
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        // NOTE(unsafe) atomic read with no side effects
        Ok(unsafe { (*self.gpio.ptr()).is_low(self.index.index()) })
    }
}

impl<Gpio, Index> StatefulOutputPin for Pin<Gpio, Index, Output<Stateful>>
where
    Gpio: marker::Gpio,
    Index: marker::Index,
{
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        Ok(!self.is_set_low()?)
    }

    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        // NOTE(unsafe) atomic read with no side effects
        Ok(unsafe { (*self.gpio.ptr()).is_set_low(self.index.index()) })
    }

    fn toggle(&mut self) -> Result<(), Self::Error> {
        unsafe { (*self.gpio.ptr()).toggle(self.index.index()) }
        Ok(())
    }
}

impl<Gpio, Index, Mode> Pin<Gpio, Index, Mode>
where
    Gpio: marker::Gpio,
    Index: marker::Index,
    Mode: marker::Readable,
{
    /// Configure external interrupts from this pin
    pub fn configure_interrupt(&mut self, edge: Edge) {
        // NOTE(unsafe) atomic write with no side effects
        unsafe { (*self.gpio.ptr()).configure_interrupt(self.index.index(), edge) }
    }

    /// Disable the external interrupts from this pin
    pub fn disable_interrupt(&mut self) {
        // FIXME: This function should exist twice - once for stateful inputs and once for stateless
        //        need to keep the input buffer enabled or disabled
        // NOTE(unsafe) atomic write with no side effects
        unsafe { (*self.gpio.ptr()).enable_input_buffer(self.index.index()) }
    }

    /// Clear the interrupt pending bit for this pin
    pub fn clear_interrupt(&mut self) {
        // NOTE(unsafe) atomic write with no side effects
        unsafe { (*self.gpio.ptr()).clear_interrupt_pending(self.index.index()) }
    }

    /// Reads the interrupt pending bit for this pin
    pub fn is_interrupt_pending(&self) -> bool {
        // NOTE(unsafe) atomic write with no side effects
        unsafe { (*self.gpio.ptr()).interrupt_pending(self.index.index()) }
    }
}

macro_rules! gpio_trait {
    ([$($gpioy:ident),+ $(,)?]) => {
        $(
            impl GpioRegExt for crate::pac::$gpioy::RegisterBlock {
                #[inline(always)]
                fn is_low(&self, i: u8) -> bool {
                    self.input().read().bits() & (1 << i) == 0
                }

                #[inline(always)]
                fn is_set_low(&self, i: u8) -> bool {
                    self.out().read().bits() & (1 << i) == 0
                }

                #[inline(always)]
                fn set_high(&self, i: u8) {
                    // NOTE(unsafe, write) atomic write to a stateless register
                    unsafe { self.outset().write(|w| w.bits(1 << i)) };
                }

                #[inline(always)]
                fn set_low(&self, i: u8) {
                    // NOTE(unsafe, write) atomic write to a stateless register
                    unsafe { self.outclr().write(|w| w.bits(1 << i)) };
                }

                #[inline(always)]
                fn toggle(&self, i: u8) {
                    // NOTE(unsafe, write) atomic write to a stateless register
                    unsafe { self.outtgl().write(|w| w.bits(1 << i)) };
                }

                #[inline(always)]
                fn input(&self, i: u8) {
                    // NOTE(unsafe, write) atomic write to a stateless register
                    unsafe { self.dirclr().write(|w| w.bits(1 << i)) };
                }

                #[inline(always)]
                fn output(&self, i: u8) {
                    // NOTE(unsafe, write) atomic write to a stateless register
                    unsafe { self.dirset().write(|w| w.bits(1 << i)) };
                }

                #[inline(always)]
                fn floating(&self, i: u8) {
                    self.pinctrl(i as usize).modify(|_, w| w.pullupen().clear_bit());
                }

                #[inline(always)]
                fn pull_up(&self, i: u8) {
                    self.pinctrl(i as usize).modify(|_, w| w.pullupen().set_bit());
                }

                #[inline(always)]
                fn normal(&self, i: u8) {
                    self.pinctrl(i as usize).modify(|_, w| w.inven().clear_bit());
                }

                #[inline(always)]
                fn inverted(&self, i: u8) {
                    self.pinctrl(i as usize).modify(|_, w| w.inven().set_bit());
                }

                #[inline(always)]
                fn enable_input_buffer(&self, i: u8) {
                    self.pinctrl(i as usize).modify(|_, w| w.isc().intdisable());
                }

                #[inline(always)]
                fn disable_input_buffer(&self, i: u8) {
                    self.pinctrl(i as usize).modify(|_, w| w.isc().input_disable());
                }

                #[inline(always)]
                fn interrupt_pending(&self, i: u8) -> bool {
                    self.intflags().read().bits() & (1 << i) != 0
                }

                #[inline(always)]
                fn clear_interrupt_pending(&self, i: u8) {
                    // NOTE(unsafe, write) atomic write to a stateless register
                    unsafe { self.intflags().write(|w| w.bits(1 << i)) };
                }

                #[inline(always)]
                fn configure_interrupt(&self, i: u8, edge: Edge) {
                    match edge {
                        Edge::Rising => self.pinctrl(i as usize).modify(|_, w| w.isc().rising()),
                        Edge::Falling => self.pinctrl(i as usize).modify(|_, w| w.isc().falling()),
                        Edge::RisingFalling => self.pinctrl(i as usize).modify(|_, w| w.isc().bothedges()),
                        Edge::LowLevel => self.pinctrl(i as usize).modify(|_, w| w.isc().level()),
                    };
                }
            }
        )+
    };
}

macro_rules! gpio {
    ({
        port: $portx:ident,
        Port: $Portx:ident,
        port_index: $port_index:literal,
        port_mapped: $porty:ident,
        partially_erased_pin: $PXx:ident,
        pins: [$(
           $i:literal => (
               $PXi:ident, $pxi:ident,
           ),
        )+],
    }) => {
        #[doc = concat!("GPIO port ", stringify!($Portx), " (type state)")]
        #[derive(ufmt::derive::uDebug, Debug)]
        pub struct $Portx;

        impl private::Gpio for $Portx {
            type Reg = crate::pac::$porty::RegisterBlock;

            #[inline(always)]
            fn ptr(&self) -> *const Self::Reg {
                crate::pac::$Portx::ptr()
            }

            #[inline(always)]
            fn port_index(&self) -> u8 {
                $port_index
            }
        }

        impl marker::Gpio for $Portx {}
        impl marker::GpioStatic for $Portx {}

        $(
            #[doc = concat!("Pin ", stringify!($PXi))]
            pub type $PXi<Mode> = Pin<$Portx, U<$i>, Mode>;
        )+

        #[doc = concat!("Partially erased pin for ", stringify!($Portx))]
        pub type $PXx<Mode> = Pin<$Portx, Ux, Mode>;

        #[doc = concat!("All Pins and associated registers for GPIO port ", stringify!($Portx))]
        pub mod $portx {
            use core::marker::PhantomData;

            use super::{$Portx, GpioExt, U};
            use super::Input;

            pub use super::{
                $PXx,
                $(
                    $PXi,
                )+
            };

            /// GPIO parts
            pub struct Parts {
                $(
                    #[doc = concat!("Pin ", stringify!($PXi))]
                    pub $pxi: $PXi<Input>,
                )+
            }

            impl GpioExt for crate::pac::$Portx {
                type Parts = Parts;

                fn split(self) -> Parts {
                    Parts {
                        $(
                            $pxi: $PXi {
                                gpio: $Portx,
                                index: U::<$i>,
                                _mode: PhantomData,
                            },
                        )+
                    }
                }
            }
        }
    };

    ({
        pacs: $pacs:tt,
        ports: [$(
            {
                port: ($X:ident/$x:ident, $port_index:literal, $porty:ident),
                pins: [$($i:literal),+],
            },
        )+],
    }) => {
        paste::paste! {
            gpio_trait!($pacs);
            $(
                gpio!({
                    port: [<port $x>],
                    Port: [<PORT $X>],
                    port_index: $port_index,
                    port_mapped: $porty,
                    partially_erased_pin: [<P $X x>],
                    pins: [$(
                        $i => (
                            [<P $X $i>], [<p $x $i>],
                        ),
                    )+],
                });
            )+
        }
    };
}

gpio!({
    pacs: [porta, portb, portc],
    ports: [
        {
            port: (A/a, 0, porta),
            pins: [ 0, 1, 2, 3, 4, 5, 6, 7 ],
        },
        {
            port: (B/b, 1, portb),
            pins: [ 0, 1, 2, 3, 4, 5, 6, 7 ],
        },
        {
            port: (C/c, 2, portc),
            pins: [ 0, 1, 2, 3, 4, 5 ],
        },
    ],
});

use crate::evsys::{Channel, ChannelConfigurator, EventGenerator, GeneratorAssigned, Unconfigured};

// Generator for PortA
// only routable to ASYNCCH0
impl<Evsys, Index, const X: u8> EventGenerator<Evsys, crate::evsys::Async, Index>
    for Pin<PORTA, U<X>, Input>
where
    Evsys: crate::evsys::marker::Evsys,
    Index: crate::evsys::marker::Index<X = 0>,
{
    type EventSource = ();

    fn connect_event_generator(
        &mut self,
        mut channel: Channel<Evsys, crate::evsys::Async, Index, Unconfigured>,
        _source: Self::EventSource,
    ) -> Channel<Evsys, crate::evsys::Async, Index, GeneratorAssigned> {
        channel.set_generator(0x0A + X);
        channel.into_state()
    }
}

// only routable to SYNCCH0
impl<Evsys, Index, const X: u8> EventGenerator<Evsys, crate::evsys::Sync, Index>
    for Pin<PORTA, U<X>, Input>
where
    Evsys: crate::evsys::marker::Evsys,
    Index: crate::evsys::marker::Index<X = 0>,
{
    type EventSource = ();

    fn connect_event_generator(
        &mut self,
        mut channel: Channel<Evsys, crate::evsys::Sync, Index, Unconfigured>,
        _source: Self::EventSource,
    ) -> Channel<Evsys, crate::evsys::Sync, Index, GeneratorAssigned> {
        channel.set_generator(0x0D + X);
        channel.into_state()
    }
}

// Generator for PortB
// only routable to ASYNCCH1
impl<Evsys, Index, const X: u8> EventGenerator<Evsys, crate::evsys::Async, Index>
    for Pin<PORTB, U<X>, Input>
where
    Evsys: crate::evsys::marker::Evsys,
    Index: crate::evsys::marker::Index<X = 1>,
{
    type EventSource = ();

    fn connect_event_generator(
        &mut self,
        mut channel: Channel<Evsys, crate::evsys::Async, Index, Unconfigured>,
        _source: Self::EventSource,
    ) -> Channel<Evsys, crate::evsys::Async, Index, GeneratorAssigned> {
        channel.set_generator(0x0A + X);
        channel.into_state()
    }
}

// only routable to SYNCCH1
impl<Evsys, Index, const X: u8> EventGenerator<Evsys, crate::evsys::Sync, Index>
    for Pin<PORTB, U<X>, Input>
where
    Evsys: crate::evsys::marker::Evsys,
    Index: crate::evsys::marker::Index<X = 1>,
{
    type EventSource = ();

    fn connect_event_generator(
        &mut self,
        mut channel: Channel<Evsys, crate::evsys::Sync, Index, Unconfigured>,
        _source: Self::EventSource,
    ) -> Channel<Evsys, crate::evsys::Sync, Index, GeneratorAssigned> {
        channel.set_generator(0x08 + X);
        channel.into_state()
    }
}

// Generator for PortC
// only routable to ASYNCCH2
impl<Evsys, Index, const X: u8> EventGenerator<Evsys, crate::evsys::Async, Index>
    for Pin<PORTC, U<X>, Input>
where
    Evsys: crate::evsys::marker::Evsys,
    Index: crate::evsys::marker::Index<X = 2>,
{
    type EventSource = ();

    fn connect_event_generator(
        &mut self,
        mut channel: Channel<Evsys, crate::evsys::Async, Index, Unconfigured>,
        _source: Self::EventSource,
    ) -> Channel<Evsys, crate::evsys::Async, Index, GeneratorAssigned> {
        channel.set_generator(0x0A + X);
        channel.into_state()
    }
}

// only routable to SYNCCH0
impl<Evsys, Index, const X: u8> EventGenerator<Evsys, crate::evsys::Sync, Index>
    for Pin<PORTC, U<X>, Input>
where
    Evsys: crate::evsys::marker::Evsys,
    Index: crate::evsys::marker::Index<X = 0>,
{
    type EventSource = ();

    fn connect_event_generator(
        &mut self,
        mut channel: Channel<Evsys, crate::evsys::Sync, Index, Unconfigured>,
        _source: Self::EventSource,
    ) -> Channel<Evsys, crate::evsys::Sync, Index, GeneratorAssigned> {
        channel.set_generator(0x07 + X);
        channel.into_state()
    }
}
