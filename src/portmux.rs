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
        // Non-exhaustive so that tokens for any not-yet-supported routings
        // can be added without a breaking change: users can move fields
        // out but never construct or exhaustively destructure the struct.
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

            $(#[$meta])*
            impl $Token {
                // Register access for the mux impls. The token is the sole
                // owner of its routing field: `constrain` consumed the PAC
                // singleton and hands out each token exactly once.
                //
                // allow(dead_code): tokens for single-position functions
                // (e.g. TWI0 on the 0-series) never touch PORTMUX, so their
                // `regs` goes unused on those chips.
                #[allow(dead_code)]
                pub(crate) fn regs(&self) -> &crate::pac::portmux::RegisterBlock {
                    unsafe { &*crate::pac::PORTMUX::ptr() }
                }
            }

            $(#[$meta])*
            impl crate::private::Sealed for $Token {}
        )+

        impl PortmuxExt for crate::pac::PORTMUX {
            // allow(unused_doc_comments): the forwarded field metas include
            // the fields' doc comments, which rustdoc ignores on struct
            // expression fields; only the forwarded `#[cfg]`s matter here.
            #[allow(unused_doc_comments)]
            fn constrain(self) -> Portmux {
                Portmux {
                    $($(#[$meta])* $field: $Token { _private: () },)+
                }
            }
        }
    };
}

// Tokens whose function has no bonded pin on the selected package are
// cfg-gated away entirely rather than handed out dead: a token that can
// never be consumed by a pinset would only suggest a capability the
// package doesn't have. Functions with a single pin position (e.g. TWI0
// on the 0-series) keep their token — the pinset still consumes it so the
// exclusive-use guarantee holds — but their `mux` does not need to write
// any routing bit.
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
    /// Routing token for the TCA0 waveform output 3 pin (split mode)
    tca0_wo3: Tca0Wo3Mux,
    /// Routing token for the TCA0 waveform output 4 pin (split mode)
    #[cfg(not(feature = "pins-8"))]
    tca0_wo4: Tca0Wo4Mux,
    /// Routing token for the TCA0 waveform output 5 pin (split mode)
    #[cfg(not(feature = "pins-8"))]
    tca0_wo5: Tca0Wo5Mux,
    /// Routing token for the TCB0 waveform output pin
    tcb0: Tcb0Mux,
    /// Routing token for the TCB1 waveform output pin
    #[cfg(feature = "periph-tcb1")]
    tcb1: Tcb1Mux,
    /// Enable token for the event system output 0 pin
    evout0: Evout0Mux,
    /// Enable token for the event system output 1 pin
    #[cfg(not(feature = "pins-8"))]
    evout1: Evout1Mux,
    /// Enable token for the event system output 2 pin
    #[cfg(any(feature = "pins-20", feature = "pins-24"))]
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

// The per-peripheral `IntoMuxedPinset` impls live next to their drivers,
// generated from the same pin tables as the pin marker impls (the
// `usart!`/`twi!`/`spi!` invocations in serial.rs/twi.rs/spi.rs and the
// `single_output_pins!` invocations in tca.rs/tcb.rs/ccl.rs), so each
// package's pin data exists exactly once.

/// Generate the pin-marker impls *and* the [`IntoMuxedPinset`] impls for a
/// single-pin peripheral output routed by one PORTMUX bit (TCA/TCB
/// waveform outputs, CCL LUT outputs).
///
/// Two variants: with a `channel:` entry the marker/pinset carry the
/// channel const generic (`Marker<Key, {0 + CHAN}>`), without it they are
/// plain (`Marker<Key>`). Every pin entry is `(Port/port, pin) => action`
/// where the action is `clear` (default position), `set` (alternate) or
/// `none` (single-position function without a routing bit).
///
/// The marker trait and pinset constructor are passed as identifiers and
/// resolve at the invocation site, which is what lets tca.rs/tcb.rs/ccl.rs
/// each use their own local marker trait with this one macro.
macro_rules! single_output_pins {
    (@route $token:ident, $reg:ident, $field:ident, clear) => {
        $token.regs().$reg().modify(|_r, w| w.$field().clear_bit());
    };
    (@route $token:ident, $reg:ident, $field:ident, set) => {
        $token.regs().$reg().modify(|_r, w| w.$field().set_bit());
    };
    (@route $token:ident, $reg:ident, $field:ident, none) => {};

    // Channel-numbered outputs (TCA/TCB waveform outputs). `key` is the
    // marker/pinset key type; `mux_key` is the `IntoMuxedPinset` peripheral
    // parameter — they differ for the TCBs, whose pinsets are keyed on the
    // 8-bit PWM wrapper while muxing stays keyed on the raw PAC type.
    (
        key: $Key:ty,
        mux_key: $MuxKey:ty,
        marker: $Marker:ident,
        pinset: $Pinset:ident,
        channel: $chan:ident,
        token: $Token:ident,
        route: $reg:ident / $field:ident,
        pins: [$(
            $(#[$meta:meta])*
            ($X:ident/$x:ident, $pin:literal) => $route:ident,
        )+]
    ) => {
        $(
            paste::paste! {
                $(#[$meta])*
                impl $Marker<$Key, { 0 + $chan }>
                    for crate::gpio::[<port $x>]::[<P $X $pin>]<
                        crate::gpio::Output<crate::gpio::Stateless>,
                    >
                {
                }

                $(#[$meta])*
                impl crate::portmux::IntoMuxedPinset<$MuxKey>
                    for crate::gpio::[<port $x>]::[<P $X $pin>]<
                        crate::gpio::Output<crate::gpio::Stateless>,
                    >
                {
                    type Pinset = $Pinset<
                        $Key,
                        crate::gpio::[<port $x>]::[<P $X $pin>]<
                            crate::gpio::Output<crate::gpio::Stateless>,
                        >,
                        { 0 + $chan },
                    >;

                    type Token = crate::portmux::$Token;

                    fn mux(self, token: Self::Token) -> Self::Pinset {
                        single_output_pins!(@route token, $reg, $field, $route);
                        let _ = &token;
                        $Pinset::new(self)
                    }
                }
            }
        )+
    };

    // Channel-less outputs (CCL LUT outputs).
    (
        key: $Key:ty,
        marker: $Marker:ident,
        pinset: $Pinset:ident,
        token: $Token:ident,
        route: $reg:ident / $field:ident,
        pins: [$(
            $(#[$meta:meta])*
            ($X:ident/$x:ident, $pin:literal) => $route:ident,
        )+]
    ) => {
        $(
            paste::paste! {
                $(#[$meta])*
                impl $Marker<$Key>
                    for crate::gpio::[<port $x>]::[<P $X $pin>]<
                        crate::gpio::Output<crate::gpio::Stateless>,
                    >
                {
                }

                $(#[$meta])*
                impl crate::portmux::IntoMuxedPinset<$Key>
                    for crate::gpio::[<port $x>]::[<P $X $pin>]<
                        crate::gpio::Output<crate::gpio::Stateless>,
                    >
                {
                    type Pinset = $Pinset<
                        $Key,
                        crate::gpio::[<port $x>]::[<P $X $pin>]<
                            crate::gpio::Output<crate::gpio::Stateless>,
                        >,
                    >;

                    type Token = crate::portmux::$Token;

                    fn mux(self, token: Self::Token) -> Self::Pinset {
                        single_output_pins!(@route token, $reg, $field, $route);
                        let _ = &token;
                        $Pinset::new(self)
                    }
                }
            }
        )+
    };
}
pub(crate) use single_output_pins;

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
}
#[cfg(not(feature = "pins-8"))]
evout_routing! {
    Evout1Mux => evout1,
}
#[cfg(any(feature = "pins-20", feature = "pins-24"))]
evout_routing! {
    Evout2Mux => evout2,
}
