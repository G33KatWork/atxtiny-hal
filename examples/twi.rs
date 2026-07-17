#![no_std]
#![no_main]

use panic_halt as _;

use atxtiny_hal::pac;
use atxtiny_hal::prelude::*;
use atxtiny_hal::twi::{Error, NackSource, Twi, TwiClock};

use atxtiny_hal::embedded_hal::i2c::I2c;

#[avr_device::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Constrain a few peripherals into our HAL types
    let clkctrl = dp.CLKCTRL.constrain();
    let portmux = dp.PORTMUX.constrain();

    // Configure our clocks
    let _clocks = clkctrl.freeze().expect("valid clock config");

    // Grab and multiplex the TWI pins: PB0/PB1 on 14-pin-and-up packages,
    // PA2/PA1 (the sole position) on the 8-pin ones.
    #[cfg(not(feature = "pins-8"))]
    let twi_pair = {
        let b = dp.PORTB.split();
        (b.pb0.into_peripheral(), b.pb1.into_peripheral()).mux(portmux.twi0)
    };
    #[cfg(feature = "pins-8")]
    let twi_pair = {
        // The turbofish disambiguates: PA2/PA1 also form the USART0
        // alternate pinset on the 8-pin packages.
        let a = dp.PORTA.split();
        (
            a.pa2.into_peripheral::<pac::TWI0>(),
            a.pa1.into_peripheral::<pac::TWI0>(),
        )
            .mux(portmux.twi0)
    };

    // Create a TWI abstraction
    const TWI_CLK: TwiClock = TwiClock::new(20_000_000, 100_000);
    let mut twi = Twi::new(dp.TWI0, twi_pair, TWI_CLK);

    // Send a string to address 3
    twi.write(0x03, "Hello over I2C".as_bytes()).unwrap();

    // I2C eeprom at address 0x50
    // Write 1 byte 0x55 to EEPROM offset 0x0000
    twi.write(0x50, &[0, 0, 0x55]).unwrap();

    // Wait for the EEPROM to finish the write
    // While the EEPROM writes, it won't ACK any addressing attempts
    while twi.read(0x50, &mut []) == Result::Err(Error::Nack(NackSource::Address)) {}

    // Read 1 byte from EEPROM offset 0x0000
    let mut buf = [0u8];
    twi.write_read(0x50, &[0x00, 0x00], &mut buf).unwrap();

    if buf[0] == 0x55 {
        twi.write(0x03, "EEPROM read success".as_bytes()).unwrap();
    } else {
        twi.write(0x03, "EEPROM read failure".as_bytes()).unwrap();
    }

    loop {}
}
