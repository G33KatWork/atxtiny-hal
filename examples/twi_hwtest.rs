//! # TWI hardware regression test
//!
//! Exercises the TWI fixes from the code review against real
//! hardware. Wiring, matching `examples/twi.rs`:
//!
//! * PB0 = SCL, PB1 = SDA, external pull-ups (4.7k @ 100 kHz)
//!
//!   The internal pull-ups (20-50k) are enabled as a fallback, so on a
//!   short bench bus the test runs without external resistors. Note the
//!   resulting 1-2 µs rise times violate the 1000 ns standard-mode limit:
//!   fine for the functional tests, but fit external pull-ups for the SCL
//!   frequency measurement below - with saggy edges the measured rate
//!   drops below the computed target even when the divider math is right.
//! * An I2C EEPROM with 2-byte addressing (24C32/24C64 style) at 0x50
//! * The i2cdebug listener at address 0x03 - every test reports
//!   "Tn PASS"/"Tn FAIL" there
//!
//! ## What each test proves
//!
//! * **T1 - multi-byte read (SMEN):** a 4-byte sequential read.
//!   The pre-fix driver deadlocked on the second byte (smart mode was
//!   never enabled, so reading MDATA didn't clock in the next byte).
//!   Old firmware hangs here forever; new firmware proceeds.
//! * **T2 - merged write operations (transaction contract):**
//!   one `transaction` with two adjacent `Write` operations must reach
//!   the EEPROM as a single continuous write. The pre-fix driver
//!   re-addressed per operation, so the payload of the second write was
//!   consumed as a fresh address pointer and the data landed in the
//!   wrong place - the readback catches that.
//! * **T3 - NACK recovery:** addressing an unused address must return
//!   `Nack(Address)` promptly (not hang) and leave the bus usable.
//! * **T4 - zero-length ACK polling:** the classic EEPROM busy-poll with
//!   an empty read buffer, as in `examples/twi.rs`.
//!
//! ## Manual checks (scope / logic analyzer)
//!
//! * **SCL frequency:** after the tests pass, the example loops
//!   a `write_read` forever to give you a steady signal. At 20 MHz
//!   CLK_PER expect SCL just under 100 kHz. The pre-fix MBAUD overflow
//!   produced ~92 kHz - a clearly measurable difference.
//! * **Timeout:** while the traffic loop runs, short SCL to GND
//!   for a moment. Old firmware stays dead after that (unbounded poll);
//!   new firmware reports "recovered from: timeout" to the debug listener
//!   once the short is removed and resumes the traffic loop. (The message
//!   necessarily arrives after recovery - while the bus is shorted, the
//!   report itself cannot get through, so the error is latched.)
//! * **Contract on the wire:** capture T2 with a logic analyzer:
//!   exactly one START, one address byte, four data bytes, one STOP - no
//!   repeated START between the two Write operations.

#![no_std]
#![no_main]

use panic_halt as _;

use atxtiny_hal::pac;
use atxtiny_hal::prelude::*;
use atxtiny_hal::twi::{Error, NackSource, Twi, TwiClock};

use atxtiny_hal::embedded_hal::i2c::{I2c, Operation};

/// Address of the i2cdebug listener used for result reporting.
const DEBUG: u8 = 0x03;
/// Address of the EEPROM under test.
const EEPROM: u8 = 0x50;
/// An address where nothing is expected to respond (T3).
const UNUSED: u8 = 0x2A;

/// Poll the EEPROM until it ACKs again after a write cycle (T4 pattern).
///
/// Bounded: a wedged fixture must produce a reportable failure, not hang
/// the test sequence.
fn eeprom_ack_poll<B: I2c<Error = Error>>(twi: &mut B) -> Result<(), &'static str> {
    for _ in 0..10_000u16 {
        match twi.read(EEPROM, &mut []) {
            Ok(()) => return Ok(()),
            Err(Error::Nack(NackSource::Address)) => continue,
            Err(e) => return Err(err_name(&e)),
        }
    }
    Err("ack-poll gave up")
}

fn report<B: I2c<Error = Error>>(twi: &mut B, msg: &str) {
    // The debug listener may be absent; don't let reporting failures
    // abort the test sequence.
    let _ = twi.write(DEBUG, msg.as_bytes());
}

/// Static name for an error so failures can be reported over the debug
/// listener instead of dying silently in `panic_halt`.
fn err_name(e: &Error) -> &'static str {
    match e {
        Error::Arbitration => "arbitration",
        Error::Bus => "bus error",
        Error::Busy => "bus busy",
        Error::Nack(NackSource::Address) => "nack (address)",
        Error::Nack(NackSource::Data) => "nack (data)",
        Error::Timeout => "timeout",
    }
}

fn push(buf: &mut [u8; 48], len: &mut usize, s: &str) {
    for &b in s.as_bytes() {
        if *len < buf.len() {
            buf[*len] = b;
            *len += 1;
        }
    }
}

/// Report `label PASS` / `label FAIL: <detail>` as a single message and
/// carry on. Every test failure must end up on the debug listener - with
/// `panic_halt` an `unwrap()` would halt the chip without a trace.
fn check<B: I2c<Error = Error>>(twi: &mut B, label: &str, result: Result<(), &'static str>) {
    let mut buf = [0u8; 48];
    let mut len = 0;

    push(&mut buf, &mut len, label);
    match result {
        Ok(()) => push(&mut buf, &mut len, " PASS"),
        Err(detail) => {
            push(&mut buf, &mut len, " FAIL: ");
            push(&mut buf, &mut len, detail);
        }
    }

    let _ = twi.write(DEBUG, &buf[..len]);
}

#[avr_device::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Constrain a few peripherals into our HAL types
    let clkctrl = dp.CLKCTRL.constrain();
    let portmux = dp.PORTMUX.constrain();

    // Configure our clocks
    let _clocks = clkctrl.freeze().expect("valid clock config");

    // Split the PORTB peripheral into its pins and grab the TWI pins.
    //
    // Peripheral mode deliberately supports internal pull-ups (unlike
    // Analog mode); PULLUPEN stays in effect while the TWI overrides the
    // pin, so the bus works without external resistors on a short bench
    // setup. See the module docs for the rise-time caveat.
    // TWI sits on PB0/PB1 on 14-pin-and-up packages and on PA2/PA1 (the
    // sole position) on the 8-pin ones.
    #[cfg(not(feature = "pins-8"))]
    let (mut sclpin, mut sdapin) = {
        let b = dp.PORTB.split();
        (b.pb0.into_peripheral(), b.pb1.into_peripheral())
    };
    #[cfg(feature = "pins-8")]
    let (mut sclpin, mut sdapin) = {
        // The turbofish disambiguates: PA2/PA1 also form the USART0
        // alternate pinset on the 8-pin packages.
        let a = dp.PORTA.split();
        (
            a.pa2.into_peripheral::<pac::TWI0>(),
            a.pa1.into_peripheral::<pac::TWI0>(),
        )
    };
    sclpin.internal_pull_up(Toggle::On);
    sdapin.internal_pull_up(Toggle::On);

    // Multiplex the TWI pins
    let twi_pair = (sclpin, sdapin).mux(portmux.twi0);

    // Create a TWI abstraction: 100 kHz, computed at compile time
    const TWI_CLK: TwiClock = TwiClock::new(20_000_000, 100_000);
    let mut twi = Twi::new(dp.TWI0, twi_pair, TWI_CLK);

    report(&mut twi, "TWI hwtest start");

    // Probe for the EEPROM fixture: a zero-length write is a bare
    // addressing attempt. Without the EEPROM only T3 can run; report the
    // skip loudly instead of dying in an unwrap (with panic_halt a panic
    // would just go silent after the start banner).
    let eeprom_present = twi.write(EEPROM, &[]).is_ok();
    if !eeprom_present {
        report(&mut twi, "no EEPROM at 0x50 - skipping T1/T2");
    }

    if eeprom_present {
        // -----------------------------------------------------------
        // T1: multi-byte sequential read (deadlocked before SMEN fix)
        // -----------------------------------------------------------

        // Write four known bytes to EEPROM offset 0x0000, then read them
        // back in one go. Old firmware never gets past the second byte of
        // the read - a hang here IS the pre-fix failure signature.
        let r = (|| {
            twi.write(EEPROM, &[0x00, 0x00, 0xDE, 0xAD, 0xBE, 0xEF])
                .map_err(|e| err_name(&e))?;
            eeprom_ack_poll(&mut twi)?;

            let mut buf4 = [0u8; 4];
            twi.write_read(EEPROM, &[0x00, 0x00], &mut buf4)
                .map_err(|e| err_name(&e))?;

            if buf4 == [0xDE, 0xAD, 0xBE, 0xEF] {
                Ok(())
            } else {
                Err("data mismatch")
            }
        })();
        check(&mut twi, "T1 multi-byte read", r);

        // -----------------------------------------------------------
        // T2: adjacent Write operations merge without a repeated START
        // -----------------------------------------------------------

        // Two Write operations in one transaction must arrive at the
        // EEPROM as a single continuous write of [addr_hi, addr_lo,
        // 0xA5, 0x5A]. The pre-fix driver re-addressed for the second
        // operation, so the EEPROM interpreted [0xA5, 0x5A] as a new
        // address pointer instead of data and the readback mismatches.
        let r = (|| {
            let mut ops = [
                Operation::Write(&[0x00, 0x10]),
                Operation::Write(&[0xA5, 0x5A]),
            ];
            twi.transaction(EEPROM, &mut ops)
                .map_err(|e| err_name(&e))?;
            eeprom_ack_poll(&mut twi)?;

            let mut buf2 = [0u8; 2];
            twi.write_read(EEPROM, &[0x00, 0x10], &mut buf2)
                .map_err(|e| err_name(&e))?;

            if buf2 == [0xA5, 0x5A] {
                Ok(())
            } else {
                Err("data mismatch")
            }
        })();
        check(&mut twi, "T2 merged writes", r);
    }

    // ---------------------------------------------------------------
    // T3: NACK on an unused address returns an error and recovers
    // ---------------------------------------------------------------

    let r = (|| {
        let mut probe = [0u8];
        match twi.read(UNUSED, &mut probe) {
            Err(Error::Nack(NackSource::Address)) => {}
            Ok(()) => return Err("unexpected ack - address in use?"),
            Err(e) => return Err(err_name(&e)),
        }

        // The bus must still be usable afterwards. Use the EEPROM if we
        // have one, else a bare addressing of the debug listener (which
        // demonstrably works - it carried the start banner).
        let recovery = if eeprom_present {
            let mut probe = [0u8];
            twi.write_read(EEPROM, &[0x00, 0x00], &mut probe)
        } else {
            twi.write(DEBUG, &[])
        };
        recovery.map_err(|_| "bus stuck after nack")
    })();
    check(&mut twi, "T3 nack + recovery", r);

    report(&mut twi, "tests done, starting traffic loop");

    // ---------------------------------------------------------------
    // Endless traffic for the manual checks: SCL frequency measurement
    // and the short-SCL-to-GND timeout test (see module docs). Without
    // the EEPROM the loop falls back to address probes - a NACKed
    // addressing still produces nine measurable SCL cycles.
    // ---------------------------------------------------------------

    // While the bus is broken (e.g. SCL shorted to GND for the timeout
    // check), reporting over that same bus cannot work - so failures are
    // latched here and reported on the first transaction that succeeds
    // again after the fault is removed.
    let mut pending: Option<&'static str> = None;

    loop {
        let result = if eeprom_present {
            let mut buf = [0u8; 4];
            twi.write_read(EEPROM, &[0x00, 0x00], &mut buf)
        } else {
            match twi.read(UNUSED, &mut []) {
                Ok(()) | Err(Error::Nack(_)) => Ok(()),
                e => e,
            }
        };

        match result {
            Ok(()) => {
                if let Some(msg) = pending.take() {
                    let mut buf = [0u8; 48];
                    let mut len = 0;
                    push(&mut buf, &mut len, "recovered from: ");
                    push(&mut buf, &mut len, msg);
                    let _ = twi.write(DEBUG, &buf[..len]);
                }
            }
            Err(e) => pending = Some(err_name(&e)),
        }

        // Pause between transactions so individual frames are easy to
        // pick out on a scope.
        for _ in 0..20_000u32 {
            core::hint::spin_loop();
        }
    }
}
