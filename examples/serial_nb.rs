#![no_std]
#![no_main]

use panic_halt as _;

use atxtiny_hal::embedded_hal_nb::serial::{Read, Write};
use atxtiny_hal::pac;
use atxtiny_hal::prelude::*;
use atxtiny_hal::serial::Serial;

use atxtiny_hal::embedded_hal_nb::nb::block;

#[avr_device::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Constrain a few peripherals into our HAL types
    let clkctrl = dp.clkctrl.constrain();
    let portmux = dp.portmux.constrain();

    // Configure our clocks
    let clocks = clkctrl.freeze();

    // Split the PORTA peripheral into its pins
    let a = dp.porta.split();

    // Grab the serial port pins
    // We need to annotate the pins with the peripheral here because PA1/2 can
    // also be used as TWI pins and we need to tell the MUX what bit to flip
    let rxpin = a.pa2.into_peripheral::<pac::Usart0>();
    let txpin = a.pa1.into_peripheral::<pac::Usart0>();

    // Multiplex the serial port pins
    let usart_pair = (rxpin, txpin);
    let usart_pair = usart_pair.mux(&portmux);

    // Create a serial port abstraction
    let mut s = Serial::new(dp.usart0, usart_pair, 115200u32.bps(), clocks);

    // Say Hello
    for b in b"Hello World\r\n" {
        block!(s.write(*b)).unwrap();
    }

    loop {
        let b = block!(s.read()).unwrap();
        block!(s.write(b)).unwrap();
    }
}
