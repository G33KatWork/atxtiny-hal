//! # Event output pins

use core::marker::PhantomData;

use crate::gpio::Peripheral;
use crate::pac::EVSYS;
use crate::portmux::EvoutRouting;

/// Event output channel 0 (EVOUT0)
pub type const EVOUT0: u8 = 0;

/// Event output channel 1 (EVOUT1)
pub type const EVOUT1: u8 = 1;

/// Event output channel 2 (EVOUT2)
pub type const EVOUT2: u8 = 2;

/// A pin can be marked with this when it can be used as an event output pin
pub trait EventOutputPin<Ev, const EVOUT: u8> {}

/// Pin set for the port multiplexer
///
/// Also holds the PORTMUX routing token that enabled the EVOUT pin
/// routing, so [`free`](Self::free) can switch the routing off again —
/// otherwise the "freed" pin would keep outputting events.
pub struct EventOutputPinset<Ev, EventOutput, Routing, const EVOUT: u8>
where
    EventOutput: EventOutputPin<Ev, EVOUT>,
    Routing: EvoutRouting,
{
    _evsys: PhantomData<Ev>,
    output: EventOutput,
    routing: Routing,
}

impl<Ev, EventOutput, Routing, const EVOUT: u8> crate::private::Sealed
    for EventOutputPinset<Ev, EventOutput, Routing, EVOUT>
where
    EventOutput: EventOutputPin<Ev, EVOUT>,
    Routing: EvoutRouting,
{
}

impl<Ev, EventOutput, Routing, const EVOUT: u8> EventOutputPinset<Ev, EventOutput, Routing, EVOUT>
where
    EventOutput: EventOutputPin<Ev, EVOUT>,
    Routing: EvoutRouting,
{
    pub(crate) fn new(output: EventOutput, routing: Routing) -> Self {
        EventOutputPinset {
            _evsys: PhantomData,
            output,
            routing,
        }
    }

    /// Disable the EVOUT pin routing and release the pin and the routing
    /// token
    pub fn free(self) -> (EventOutput, Routing) {
        self.routing.set_routing(false);
        (self.output, self.routing)
    }
}

// One event output per port (PA2/PB2/PC2) — each exists exactly when its
// port is bonded out on the package. Each entry generates the marker impl
// and the PORTMUX `IntoMuxedPinset` impl; EVOUT differs from the other
// muxed functions in that the pinset stores the routing token so `free`
// can switch the pin routing off again.
macro_rules! evout_pins {
    ($(
        $(#[$meta:meta])*
        ($X:ident/$x:ident, $pin:literal) => $EVOUT:ident / $Token:ident,
    )+) => {
        $(
            paste::paste! {
                $(#[$meta])*
                impl EventOutputPin<EVSYS, $EVOUT>
                    for crate::gpio::[<port $x>]::[<P $X $pin>]<Peripheral<EVSYS>>
                {
                }

                $(#[$meta])*
                impl crate::portmux::IntoMuxedPinset<EVSYS>
                    for crate::gpio::[<port $x>]::[<P $X $pin>]<Peripheral<EVSYS>>
                {
                    type Pinset = EventOutputPinset<
                        EVSYS,
                        crate::gpio::[<port $x>]::[<P $X $pin>]<Peripheral<EVSYS>>,
                        crate::portmux::$Token,
                        $EVOUT,
                    >;

                    type Token = crate::portmux::$Token;

                    fn mux(self, token: Self::Token) -> Self::Pinset {
                        use crate::portmux::EvoutRouting;
                        token.set_routing(true);
                        EventOutputPinset::new(self, token)
                    }
                }
            }
        )+
    };
}

evout_pins! {
    (A/a, 2) => EVOUT0 / Evout0Mux,
    #[cfg(not(feature = "pins-8"))]
    (B/b, 2) => EVOUT1 / Evout1Mux,
    #[cfg(any(feature = "pins-20", feature = "pins-24"))]
    (C/c, 2) => EVOUT2 / Evout2Mux,
}

use crate::evsys::{Async, EventUser, Evsys, Sync, UserRegisterFile};

// The EVOUT users live in the async register file (ASYNCUSER8..10), but
// sync channels can drive async-file users too (ASYNCUSERn select values
// 1/2 = SYNCCH0/1) — hence one EventUser impl per channel flavor.
macro_rules! evout_user {
    ($($Pin:ty => $EVOUT:ident,)+) => {
        $(
            impl<R: EvoutRouting> EventUser<Evsys, Async>
                for EventOutputPinset<EVSYS, $Pin, R, $EVOUT>
            {
                const MULTIPLEXER_INDEX: u8 = 8 + $EVOUT;
                const FILE: UserRegisterFile = UserRegisterFile::Async;
            }

            impl<R: EvoutRouting> EventUser<Evsys, Sync>
                for EventOutputPinset<EVSYS, $Pin, R, $EVOUT>
            {
                const MULTIPLEXER_INDEX: u8 = 8 + $EVOUT;
                const FILE: UserRegisterFile = UserRegisterFile::Async;
            }
        )+
    };
}

// The ASYNCUSERn index assignment (8 + EVOUT number) is identical across
// the whole 0/1-series, verified against the ATDF register captions of all
// supported chips.
evout_user! {
    crate::gpio::porta::PA2<Peripheral<EVSYS>> => EVOUT0,
}
#[cfg(not(feature = "pins-8"))]
evout_user! {
    crate::gpio::portb::PB2<Peripheral<EVSYS>> => EVOUT1,
}
#[cfg(any(feature = "pins-20", feature = "pins-24"))]
evout_user! {
    crate::gpio::portc::PC2<Peripheral<EVSYS>> => EVOUT2,
}
