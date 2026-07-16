//! # Two-wire interface (TWI) bus

// TODO: client mode

use core::{marker::PhantomData, ops::Deref};

use embedded_hal::i2c::{ErrorType, I2c, Operation};

use crate::{clkctrl::Clocks, pac::twi0::RegisterBlock, time::*, Toggle};

#[cfg(feature = "enumset")]
use enumset::{EnumSet, EnumSetType};

pub mod config;

pub use config::{InvalidTwiClock, TwiClock};

/// SCL pin
pub trait SclPin<TWI>: crate::private::Sealed {}

/// SDA pin
pub trait SdaPin<TWI>: crate::private::Sealed {}

/// Pin set for the port multiplexer
pub struct TwiPinset<TWI, Scl: SclPin<TWI>, Sda: SdaPin<TWI>> {
    _twi: PhantomData<TWI>,
    scl: Scl,
    sda: Sda,
}

impl<TWI, Scl, Sda> TwiPinset<TWI, Scl, Sda>
where
    Scl: SclPin<TWI>,
    Sda: SdaPin<TWI>,
{
    pub(crate) fn new(scl: Scl, sda: Sda) -> Self {
        TwiPinset {
            _twi: PhantomData,
            scl,
            sda,
        }
    }

    pub fn free(self) -> (Scl, Sda) {
        (self.scl, self.sda)
    }
}

/// TWI error
#[derive(ufmt::derive::uDebug, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Error {
    /// Arbitration loss
    Arbitration,
    /// Bus error
    Bus,
    /// Bus busy
    ///
    /// Another master holds the bus. Retryable: the transaction was not
    /// started. (embedded-hal has no dedicated error kind for this, so it
    /// maps to [`ErrorKind::Other`](embedded_hal::i2c::ErrorKind::Other).)
    Busy,
    /// Not Acknowledge received
    Nack(NackSource),
    /// A polling loop gave up waiting
    ///
    /// A client stretched the clock (or held SDA/SCL low) for longer than
    /// the built-in poll budget - roughly 50-100 ms depending on the
    /// peripheral clock, comfortably above the 35 ms the SMBus
    /// specification allows a client to stretch. Without this bound a
    /// glitched client would hang the firmware forever.
    ///
    /// The master state machine is automatically reset before this error
    /// is returned (the abandoned transaction leaves it in an unknown,
    /// otherwise unrecoverable state), so retrying is safe and starts from
    /// a clean master. The *client* side may still hold the bus; if so,
    /// retries report arbitration/bus errors until it releases.
    Timeout,
}

/// TWI NACK error source
#[derive(ufmt::derive::uDebug, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NackSource {
    /// NACK received during Address phase
    Address,

    /// NACK received while sending or receiving data
    Data,
}

impl embedded_hal::i2c::Error for Error {
    fn kind(&self) -> embedded_hal::i2c::ErrorKind {
        use embedded_hal::i2c::{ErrorKind, NoAcknowledgeSource};

        match *self {
            Error::Arbitration => ErrorKind::ArbitrationLoss,
            Error::Bus => ErrorKind::Bus,
            Error::Nack(NackSource::Address) => {
                ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address)
            }
            Error::Nack(NackSource::Data) => ErrorKind::NoAcknowledge(NoAcknowledgeSource::Data),
            _ => ErrorKind::Other,
        }
    }
}

/// Status events.
///
/// All events can be cleared by [`Twi::clear_event`] or [`Twi::clear_events`].
/// Some events are also cleared on other conditions.
#[derive(ufmt::derive::uDebug, Debug)]
#[cfg_attr(feature = "enumset", derive(EnumSetType))]
#[cfg_attr(not(feature = "enumset"), derive(Copy, Clone, PartialEq, Eq))]
pub enum Event {
    /// Read Interrupt Flag
    ///
    /// This flag is set to when the host byte read operation is completed.
    #[doc(alias = "RIF")]
    ReadInterrupt,

    /// Write Interrupt Flag
    ///
    /// This flag is set to when a host transmit address or byte write operation
    /// is completed, regardless of the occurrence of a bus error or arbitration
    /// lost condition.
    #[doc(alias = "WIF")]
    WriteInterrupt,

    /// Clock Hold
    ///
    /// When this flag is set, it indicates that the host is currently holding
    /// the SCL low, stretching the TWI clock period.
    #[doc(alias = "CLKHOLD")]
    ClockHold,

    /// Received Acknowledge
    ///
    /// When this flag is read as ‘0’, it indicates that the most recent
    /// Acknowledge bit from the client was ACK, and the client is ready for
    /// more data.
    /// When this flag is read as ‘1’, it indicates that the most recent
    /// Acknowledge bit from the client was NACK, and the client is not able to
    /// or does not need to receive more data.
    ///
    /// **This is pure status, not a latched event**: hardware updates it on
    /// every acknowledge bit and software cannot clear it. Passing it to
    /// [`Twi::clear_event`] does nothing.
    #[doc(alias = "RXACK")]
    ReceivedAcknowledge,

    /// Arbitration Lost
    ///
    /// When this bit is read as ‘1’, it indicates that the host has lost
    /// arbitration. This can happen in one of the following cases:
    /// 1. While transmitting a high data bit.
    /// 2. While transmitting a NACK bit.
    /// 3. While issuing a Start condition (S).
    /// 4. While issuing a repeated Start (Sr).
    #[doc(alias = "ARBLOST")]
    ArbitrationLost,

    /// Bus Error
    ///
    /// The BUSERR flag indicates that an illegal bus operation has occurred.
    /// An illegal bus operation is detected if a protocol violating the
    /// Start (S), repeated Start (Sr), or Stop (P) conditions is detected on
    /// the TWI bus lines. A Start condition directly followed by a Stop
    /// condition is one example of a protocol violation.
    #[doc(alias = "BUSERR")]
    BusError,
}

/// TWI bus state.
///
/// Indication of the current TWI bus state.
pub enum BusState {
    /// Unknown bus state
    #[doc(alias = "UNKNOWN")]
    Unknown,

    /// Idle bus state
    #[doc(alias = "IDLE")]
    Idle,

    /// This TWI controls the bus
    #[doc(alias = "OWNER")]
    Owner,

    /// Busy bus state
    #[doc(alias = "BUSY")]
    Busy,
}

/// Interrupts.
///
/// Interrupts that can be enabled or disabled by [`Twi::enable_interrupt`]
/// or [`Twi::disable_interrupt`].
/// When an interrupt occurs, [`Event`] flags in status registers are set which can be read by
/// [`Twi::is_event_triggered`] and cleared by [`Twi::clear_event`].
#[derive(ufmt::derive::uDebug, Debug)]
#[cfg_attr(feature = "enumset", derive(EnumSetType))]
#[cfg_attr(not(feature = "enumset"), derive(Copy, Clone, PartialEq, Eq))]
pub enum Interrupt {
    /// Read Interrupt Enable
    #[doc(alias = "RIEN")]
    Read,

    /// Write Interrupt Enable
    #[doc(alias = "WIEN")]
    Write,
}

/// TWI abstraction in master mode
///
/// This is an abstraction of the TWI peripheral intended to be
/// used in master mode.
pub struct Twi<TWI, Pinset> {
    twi: TWI,
    pinset: Pinset,
}

impl<TWI, SCL, SDA> Twi<TWI, TwiPinset<TWI, SCL, SDA>>
where
    TWI: Instance,
    SCL: SclPin<TWI>,
    SDA: SdaPin<TWI>,
{
    /// Configures the TWI peripheral to work in master mode
    ///
    /// The bus clock arrives as a precomputed [`config::TwiClock`], ideally
    /// built in a `const` context so no divider arithmetic ends up in flash
    /// and unreachable rates fail the build. See [`config::TwiClock`].
    pub fn new(twi: TWI, pinset: TwiPinset<TWI, SCL, SDA>, clock: config::TwiClock) -> Self {
        twi.ctrla()
            .modify(|_, w| w.fmpen().variant(clock.fast_mode_plus));

        // Set the baud rate divider and enable the peripheral.
        //
        // Smart mode (SMEN) is deliberately NOT used: tinyAVR 0/1-series
        // silicon errata "TWI Smart Mode Gives Extra Clock Pulse" (e.g.
        // DS80000886A section 2.9.3, no workaround) emits a rogue SCL pulse
        // after every NACK, which corrupts the bus-state monitor - and the
        // state machine cannot self-heal because the bus-timeout bits are
        // also unusable (erratum 2.9.1). Observed on an ATtiny1617 as every
        // transaction after the first NACK failing. The read loop in
        // `transaction` therefore issues explicit RECVTRANS commands
        // instead, matching the reference implementations for this family.
        twi.mctrla().modify(|_, w| w.enable().clear_bit());
        twi.mbaud().write(|w| w.set(clock.mbaud));
        twi.mctrla().modify(|_, w| w.enable().set_bit());

        // Force the state-machine into IDLE state and clear all W1C status
        // flags in one write. Using write() (not modify()) is deliberate:
        // a read-modify-write on the W1C MSTATUS register would write back
        // whatever flags happen to be set and clear them as a side effect -
        // wanted here, but only by accident; write() states the intent.
        twi.mstatus().write(|w| {
            w.busstate()
                .idle()
                .rif()
                .set_bit()
                .wif()
                .set_bit()
                .clkhold()
                .set_bit()
                .arblost()
                .set_bit()
                .buserr()
                .set_bit()
        });

        Self { twi, pinset }
    }

    /// Get access to the underlying register block.
    ///
    /// # Safety
    ///
    /// This function is not _memory_ unsafe per se, but does not guarantee
    /// anything about assumptions of invariants made in this implementation.
    ///
    /// Changing specific options can lead to un-expected behavior and nothing
    /// is guaranteed.
    pub unsafe fn peripheral(&mut self) -> &mut TWI {
        &mut self.twi
    }

    /// Enable the interrupt for the specified [`Interrupt`].
    #[inline]
    pub fn enable_interrupt(&mut self, interrupt: Interrupt) {
        self.configure_interrupt(interrupt, Toggle::On);
    }

    /// Disable the interrupt for the specified [`Interrupt`].
    #[inline]
    pub fn disable_interrupt(&mut self, interrupt: Interrupt) {
        self.configure_interrupt(interrupt, Toggle::Off);
    }

    /// Enable or disable the interrupt for the specified [`Interrupt`].
    #[inline]
    pub fn configure_interrupt(&mut self, interrupt: Interrupt, enable: impl Into<Toggle>) {
        // Do a round way trip to be convert Into<Toggle> -> bool
        let enable: Toggle = enable.into();
        let enable: bool = enable.into();
        match interrupt {
            Interrupt::Read => self.twi.mctrla().modify(|_, w| w.rien().bit(enable)),
            Interrupt::Write => self.twi.mctrla().modify(|_, w| w.wien().bit(enable)),
        };
    }

    /// Enable or disable interrupt for the specified [`Interrupt`]s.
    ///
    /// Like [`Twi::configure_interrupt`], but instead using an enumset. The corresponding
    /// interrupt for every [`Interrupt`] in the set will be enabled, every other interrupt will be
    /// **disabled**.
    #[cfg(feature = "enumset")]
    #[cfg_attr(docsrs, doc(cfg(feature = "enumset")))]
    #[inline]
    pub fn configure_interrupts(&mut self, interrupts: EnumSet<Interrupt>) {
        for event in interrupts.complement().iter() {
            self.configure_interrupt(event, false);
        }
        for event in interrupts.iter() {
            self.configure_interrupt(event, true);
        }
    }

    /// Check if an interrupt is configured for the [`Interrupt`]
    #[inline]
    pub fn is_interrupt_configured(&self, interrupt: Interrupt) -> bool {
        match interrupt {
            Interrupt::Read => self.twi.mctrla().read().rien().bit_is_set(),
            Interrupt::Write => self.twi.mctrla().read().wien().bit_is_set(),
        }
    }

    /// Check which interrupts are enabled for all [`Interrupt`]s
    #[cfg(feature = "enumset")]
    #[cfg_attr(docsrs, doc(cfg(feature = "enumset")))]
    #[inline]
    pub fn configured_interrupts(&mut self) -> EnumSet<Interrupt> {
        let mut interrupts = EnumSet::new();

        for interrupt in EnumSet::<Interrupt>::all().iter() {
            if self.is_interrupt_configured(interrupt) {
                interrupts |= interrupt;
            }
        }

        interrupts
    }

    /// Check if an event happend.
    #[inline]
    pub fn is_event_triggered(&self, event: Event) -> bool {
        let mstatus = self.twi.mstatus().read();
        match event {
            Event::ReadInterrupt => mstatus.rif().bit(),
            Event::WriteInterrupt => mstatus.wif().bit(),
            Event::ClockHold => mstatus.clkhold().bit(),
            Event::ReceivedAcknowledge => mstatus.rxack().bit(),
            Event::ArbitrationLost => mstatus.arblost().bit(),
            Event::BusError => mstatus.buserr().bit(),
        }
    }

    /// Get an [`enumset::EnumSet`] of all fired interrupt events.
    ///
    /// # Examples
    ///
    /// This allows disabling all fired event at once, via the enum set abstraction, like so
    ///
    /// ```rust
    /// for event in twi.triggered_events() {
    ///     twi.configure_interrupt(event, false);
    /// }
    /// ```
    #[cfg(feature = "enumset")]
    #[cfg_attr(docsrs, doc(cfg(feature = "enumset")))]
    #[inline]
    pub fn triggered_events(&self) -> EnumSet<Event> {
        let mut events = EnumSet::new();

        for event in EnumSet::<Event>::all().iter() {
            if self.is_event_triggered(event) {
                events |= event;
            }
        }

        events
    }

    /// Releases the TWI peripheral and associated pins
    pub fn free(self) -> (TWI, TwiPinset<TWI, SCL, SDA>) {
        (self.twi, self.pinset)
    }

    /// Clear the given event flag.
    ///
    /// [`Event::ReceivedAcknowledge`] is pure status (updated by hardware
    /// on every acknowledge bit, not software-clearable) and is silently
    /// ignored here.
    #[inline]
    pub fn clear_event(&mut self, event: Event) {
        self.twi.mstatus().write(|w| match event {
            Event::ReadInterrupt => w.rif().set_bit(),
            Event::WriteInterrupt => w.wif().set_bit(),
            Event::ClockHold => w.clkhold().set_bit(),
            Event::ReceivedAcknowledge => w, // Not clearable, see doc comment
            Event::ArbitrationLost => w.arblost().set_bit(),
            Event::BusError => w.buserr().set_bit(),
        });
    }

    /// Clear **all** events.
    #[inline]
    pub fn clear_events(&mut self) {
        // SAFETY: This atomic write clears all flags that are clearable and spares out the busstate.
        self.twi.mstatus().write(|w| unsafe { w.bits(0b11101100) });
    }
}

/// Reset the master state machine after a polling timeout.
///
/// A timeout means the transaction was abandoned in an unknown state
/// (e.g. mid-address with SCL clamped low by a fault). Left alone, the
/// wedged master makes every subsequent transaction fail too - observed on
/// hardware as a permanently dead bus after a transient SCL short.
/// Toggling ENABLE resets the internal state machine, and the MSTATUS
/// write forces the bus-state logic to IDLE (required after enabling, see
/// erratum "TIMEOUT bits not accessible") and clears the W1C flags.
///
/// NOTE: forcing IDLE deliberately assumes a single-master bus - after a
/// fault we cannot know the true bus state, and on this silicon the
/// inactivity timeout that would normally re-synchronize the state logic
/// is unusable (same erratum). The client side may still hold SDA; if so,
/// the next transactions report arbitration/bus errors until the client
/// releases (clients usually re-synchronize on the next clean START or
/// STOP).
fn reset_master(twi: &RegisterBlock) {
    twi.mctrla().modify(|_, w| w.enable().clear_bit());
    twi.mctrla().modify(|_, w| w.enable().set_bit());
    twi.mstatus().write(|w| {
        w.busstate()
            .idle()
            .rif()
            .set_bit()
            .wif()
            .set_bit()
            .clkhold()
            .set_bit()
            .arblost()
            .set_bit()
            .buserr()
            .set_bit()
    });
}

/// Iteration bound for the status polling loops.
///
/// This is a loop count, not a calibrated time: one iteration of the status
/// poll is roughly 10 CPU cycles, so the budget comes out at ~50 ms at
/// 20 MHz and ~100 ms at 10 MHz - comfortably above the 35 ms clock
/// stretch the SMBus specification allows a client, and far below
/// "firmware hangs forever" (the failure mode of an unbounded loop when a
/// glitched client pins SCL or SDA low).
const POLL_BUDGET: u32 = 100_000;

macro_rules! busy_wait {
    ($i2c:expr, $nacksource:expr) => {{
        let mut budget: u32 = POLL_BUDGET;
        loop {
            let mstatus = $i2c.mstatus().read();

            if mstatus.arblost().bit_is_set() {
                // ARBLOST gets cleared on the next MADDR write
                return Err(Error::Arbitration);
            } else if mstatus.buserr().bit_is_set() {
                // BUSERR gets cleared on the next MADDR write
                return Err(Error::Bus);
            } else if (mstatus.wif().bit_is_set() || mstatus.rif().bit_is_set()) {
                // Received NACK
                if mstatus.rxack().bit_is_set() {
                    $i2c.mctrlb().modify(|_, w| w.mcmd().stop());
                    return Err(Error::Nack($nacksource));
                } else {
                    break;
                }
            }

            budget -= 1;
            if budget == 0 {
                reset_master(&$i2c);
                return Err(Error::Timeout);
            }
        }
    }};
}

macro_rules! wait_ownership {
    ($i2c:expr) => {{
        let mut budget: u32 = POLL_BUDGET;
        loop {
            let mstatus = $i2c.mstatus().read();

            if mstatus.arblost().bit_is_set() {
                return Err(Error::Arbitration);
            }

            // Without this check a bus error while waiting for ownership
            // (BUSSTATE never becomes OWNER) would spin the loop into the
            // timeout although the failure is already known.
            if mstatus.buserr().bit_is_set() {
                return Err(Error::Bus);
            }

            if mstatus.busstate().is_owner() {
                break;
            }

            budget -= 1;
            if budget == 0 {
                reset_master(&$i2c);
                return Err(Error::Timeout);
            }
        }
    }};
}

impl<TWI, SCL, SDA> ErrorType for Twi<TWI, TwiPinset<TWI, SCL, SDA>>
where
    TWI: Instance,
    SCL: SclPin<TWI>,
    SDA: SdaPin<TWI>,
{
    type Error = Error;
}

impl<TWI, SCL, SDA> I2c for Twi<TWI, TwiPinset<TWI, SCL, SDA>>
where
    TWI: Instance,
    SCL: SclPin<TWI>,
    SDA: SdaPin<TWI>,
{
    /// Execute an I2C transaction.
    ///
    /// Follows the embedded-hal contract: adjacent operations of the same
    /// direction continue without a repeated START; a direction change (or
    /// the first operation) puts the address on the bus; a single STOP
    /// terminates the whole transaction.
    ///
    /// Note on zero-length reads: the hardware automatically clocks in one
    /// byte after an ACKed read address, so a transaction that only probes
    /// with an empty read buffer still transfers (and discards) one byte on
    /// the wire before the NACK+STOP. This is a property of the TWI IP and
    /// cannot be avoided.
    fn transaction(&mut self, address: u8, operations: &mut [Operation<'_>]) -> Result<(), Error> {
        // The shift below would silently discard the high bit of an
        // out-of-range address; costs nothing in release builds.
        debug_assert!(address < 0x80, "I2C addresses are 7 bit");

        // An empty transaction needs no bus access at all, so it must not
        // fail on a busy bus either - this check has to come first.
        if operations.is_empty() {
            return Ok(());
        }

        // Detect Bus busy
        //
        // NOTE(TOCTOU): another master can claim the bus between this check
        // and the first MADDR write. The hardware mitigates: the START is
        // deferred until the bus is free, and a lost arbitration afterwards
        // is reported by the wait loops.
        if self.twi.mstatus().read().busstate().is_busy() {
            return Err(Error::Busy);
        }

        // The embedded-hal contract requires adjacent operations of the
        // same direction to continue without a repeated START, so only a
        // direction change (or the first operation) may write MADDR.
        // `previous` tracks the direction as Some(is_read).
        let mut previous: Option<bool> = None;

        for i in 0..operations.len() {
            // Whether another read follows decides the ACK/NACK of this
            // operation's last byte: within a merged read chain every byte
            // but the very last must be ACKed. (Peeked before the mutable
            // borrow of the current operation below.)
            let read_follows = matches!(operations.get(i + 1), Some(Operation::Read(_)));

            match &mut operations[i] {
                Operation::Read(buffer) => {
                    if previous != Some(true) {
                        // Write the address and read-bit
                        // This kicks off a START or repeated START condition on the bus
                        self.twi.maddr().write(|w| w.set(address << 1 | 1));

                        // Wait for the bus state to transition into OWNED
                        wait_ownership!(self.twi);

                        // Wait for the address to be ACKed or NACKed
                        busy_wait!(self.twi, NackSource::Address);

                        // Special case for zero-length receive buffers
                        // Just set the ACK action to NACK. The next write to MADDR or
                        // the STOP action that is executed at the end of this function
                        // then performs the NACK and the appropriate action like a STOP or
                        // repeated START
                        self.twi.mctrlb().modify(|_, w| w.ackact().set_bit());

                        previous = Some(true);
                    }

                    let mut it = buffer.iter_mut().peekable();
                    while let Some(b) = it.next() {
                        // Wait for data
                        busy_wait!(self.twi, NackSource::Data);

                        // Read the byte first, then issue the acknowledge
                        // command - the order used by the reference
                        // implementations for this family. Smart mode would
                        // fuse the two, but is unusable here (see `new`).
                        *b = self.twi.mdata().read().bits();

                        // ACK unless this is the last byte of the whole
                        // read chain - a following Read operation continues
                        // it, so its bytes count too. The ACK command also
                        // starts the reception of the next byte. For the
                        // last byte only arm NACK (no command): the final
                        // STOP below or a repeated START of a following
                        // write operation executes it.
                        if it.peek().is_some() || read_follows {
                            self.twi
                                .mctrlb()
                                .write(|w| w.ackact().clear_bit().mcmd().recvtrans());
                        } else {
                            self.twi.mctrlb().write(|w| w.ackact().set_bit());
                        }
                    }
                }

                Operation::Write(buffer) => {
                    if previous != Some(false) {
                        // Write the address and ~read-bit
                        // This kicks off a START or repeated START condition on the bus
                        self.twi.maddr().write(|w| w.set(address << 1 | 0));

                        // Wait for the bus state to transition into OWNED
                        wait_ownership!(self.twi);

                        // Wait for the address to be ACKed or NACKed
                        busy_wait!(self.twi, NackSource::Address);

                        previous = Some(false);
                    }

                    // Send bytes in the buffer
                    // Should the sent byte be NACKed, the busy_wait! macro will
                    // return and issue a STOP condition on the bus
                    for b in buffer.iter() {
                        self.twi.mdata().write(|w| w.set(*b));
                        busy_wait!(self.twi, NackSource::Data);
                    }
                }
            }
        }

        // Send the final STOP. There is deliberately no wait for the STOP
        // to complete: a back-to-back transaction's MADDR write defers its
        // START until the bus-state logic sees the bus free again, so the
        // hardware serializes the sequence by itself.
        self.twi.mctrlb().modify(|_, w| w.mcmd().stop());

        Ok(())
    }
}

/// TWI instance
pub trait Instance: Deref<Target = RegisterBlock> + crate::private::Sealed {
    #[doc(hidden)]
    fn clock(clocks: &Clocks) -> Hertz;
}

macro_rules! twi {
    ({
        instance: $TWI:ident,
        pins: [$(
            {
                scl: ($X_scl:ident/$x_scl:ident, $pin_scl:literal),
                sda: ($X_sda:ident/$x_sda:ident, $pin_sda:literal),
            },
        )+]
    }) => {
        use crate::pac::$TWI;

        impl Instance for crate::pac::$TWI {
            fn clock(clocks: &Clocks) -> Hertz {
                clocks.per()
            }
        }

        impl crate::private::Sealed for crate::pac::$TWI {}

        $(
            paste::paste! {
                impl SclPin<$TWI> for crate::gpio::[<port $x_scl>]::[<P $X_scl $pin_scl>]<Peripheral<$TWI>> {}
                impl SdaPin<$TWI> for crate::gpio::[<port $x_sda>]::[<P $X_sda $pin_sda>]<Peripheral<$TWI>> {}
            }
        )+
    };
}

use crate::gpio::Peripheral;

twi!({
    instance: TWI0,
    pins: [
        {
            scl: (B/b, 0),
            sda: (B/b, 1),
        },
        {
            scl: (A/a, 2),
            sda: (A/a, 1),
        },
    ]
});
