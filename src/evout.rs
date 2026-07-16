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

impl EventOutputPin<EVSYS, EVOUT0> for crate::gpio::porta::PA2<Peripheral<EVSYS>> {}
impl EventOutputPin<EVSYS, EVOUT1> for crate::gpio::portb::PB2<Peripheral<EVSYS>> {}
impl EventOutputPin<EVSYS, EVOUT2> for crate::gpio::portc::PC2<Peripheral<EVSYS>> {}

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

evout_user! {
    crate::gpio::porta::PA2<Peripheral<EVSYS>> => EVOUT0,
    crate::gpio::portb::PB2<Peripheral<EVSYS>> => EVOUT1,
    crate::gpio::portc::PC2<Peripheral<EVSYS>> => EVOUT2,
}
