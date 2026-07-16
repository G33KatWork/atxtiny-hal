#![no_std]
#![no_main]

use panic_halt as _;

use atxtiny_hal::embedded_hal_nb::serial::{Read, Write};
use atxtiny_hal::pac;
use atxtiny_hal::prelude::*;
use atxtiny_hal::serial::{BaudRate, Config, Serial};

use atxtiny_hal::embedded_hal_nb::nb::block;

#[avr_device::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Constrain a few peripherals into our HAL types
    let clkctrl = dp.CLKCTRL.constrain();
    let portmux = dp.PORTMUX.constrain();

    // Configure our clocks
    let _clocks = clkctrl.freeze().expect("valid clock config");

    // Split the PORTA peripheral into its pins
    let a = dp.PORTA.split();

    // Grab the serial port pins
    // We need to annotate the pins with the peripheral here because PA1/2 can
    // also be used as TWI pins and we need to tell the MUX what bit to flip
    let rxpin = a.pa2.into_peripheral::<pac::USART0>();
    let txpin = a.pa1.into_peripheral::<pac::USART0>();

    // Multiplex the serial port pins
    let usart_pair = (rxpin, txpin);
    let usart_pair = usart_pair.mux(portmux.usart0);

    // Create a serial port abstraction
    const BAUD: BaudRate = BaudRate::new(20_000_000, 115_200);
    let mut s = Serial::new(dp.USART0, usart_pair, BAUD, Config::default());

    // Say Hello
    for b in b"Hello World\r\n" {
        block!(s.write(*b)).unwrap();
    }

    loop {
        let b = block!(s.read()).unwrap();
        block!(s.write(b)).unwrap();
    }
}
