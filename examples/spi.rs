#![no_std]
#![no_main]

use panic_halt as _;

use atxtiny_hal::pac;
use atxtiny_hal::prelude::*;
use atxtiny_hal::serial::{BaudRate, Config, Serial};
use atxtiny_hal::spi::{Spi, SpiClock};

use atxtiny_hal::embedded_hal::spi::SpiDevice;
use atxtiny_hal::embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};

#[avr_device::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Constrain a few peripherals into our HAL types
    let clkctrl = dp.CLKCTRL.constrain();
    let portmux = dp.PORTMUX.constrain();

    // Configure our clocks
    let _clocks = clkctrl.freeze().expect("valid clock config");

    // Pin choices per package: the 20/24-pin packages route the SPI to its
    // PORTC alternate position and keep the USART on the PA1/PA2 alternate;
    // the 14-pin packages use the default positions of both (USART on
    // PB2/PB3, SPI on PA1-PA3), because the USART alternate would collide
    // with the SPI pins there. The second chip select moves to PB4 on
    // 20-pin packages (no PC4) and to PA5 on 14-pin ones. The 8-pin
    // packages cannot fit a UART plus SPI with two chip selects at all,
    // hence the `port-b` requirement in Cargo.toml.
    #[cfg(feature = "pins-24")]
    let (usart_pair, spi_pair, mut cs_ms, mut cs_mpu) = {
        let (a, c) = (dp.PORTA.split(), dp.PORTC.split());
        let usart_pair = (
            a.pa2.into_peripheral::<pac::USART0>(),
            a.pa1.into_peripheral::<pac::USART0>(),
        )
            .mux(portmux.usart0);
        let spi_pair = (
            c.pc0.into_peripheral(),
            c.pc1.into_peripheral(),
            c.pc2.into_peripheral(),
        )
            .mux(portmux.spi0);
        (
            usart_pair,
            spi_pair,
            c.pc3.into_stateless_push_pull_output(),
            c.pc4.into_stateless_push_pull_output(),
        )
    };
    #[cfg(feature = "pins-20")]
    let (usart_pair, spi_pair, mut cs_ms, mut cs_mpu) = {
        let (a, b, c) = (dp.PORTA.split(), dp.PORTB.split(), dp.PORTC.split());
        let usart_pair = (
            a.pa2.into_peripheral::<pac::USART0>(),
            a.pa1.into_peripheral::<pac::USART0>(),
        )
            .mux(portmux.usart0);
        let spi_pair = (
            c.pc0.into_peripheral(),
            c.pc1.into_peripheral(),
            c.pc2.into_peripheral(),
        )
            .mux(portmux.spi0);
        (
            usart_pair,
            spi_pair,
            c.pc3.into_stateless_push_pull_output(),
            b.pb4.into_stateless_push_pull_output(),
        )
    };
    #[cfg(feature = "pins-14")]
    let (usart_pair, spi_pair, mut cs_ms, mut cs_mpu) = {
        let (a, b) = (dp.PORTA.split(), dp.PORTB.split());
        let usart_pair = (
            b.pb3.into_peripheral::<pac::USART0>(),
            b.pb2.into_peripheral::<pac::USART0>(),
        )
            .mux(portmux.usart0);
        let spi_pair = (
            a.pa3.into_peripheral(),
            a.pa2.into_peripheral(),
            a.pa1.into_peripheral(),
        )
            .mux(portmux.spi0);
        (
            usart_pair,
            spi_pair,
            a.pa4.into_stateless_push_pull_output(),
            a.pa5.into_stateless_push_pull_output(),
        )
    };

    // Serial port setup
    const BAUD: BaudRate = BaudRate::new(20_000_000, 115_200);
    let mut s = Serial::new(dp.USART0, usart_pair, BAUD, Config::default());

    // Deselect any chip-selects
    cs_ms.set_high().unwrap();
    cs_mpu.set_high().unwrap();

    // Create an SPI abstraction
    const SPI_CLK: SpiClock = SpiClock::new(20_000_000, 625_000);
    let spi = Spi::new_unbuffered(dp.SPI0, spi_pair, SPI_CLK, Default::default());

    // Create an SpiDevice for the MS5611
    let mut ms5611 = ExclusiveDevice::new(spi, cs_ms, NoDelay)
        .expect("Unable to create SPI device");

    // Read MS5611 PROM
    let mut prom = [0u16; 8];
    for i in 0..8 {
        let mut buf = [0xA0 + i * 2, 0xFF, 0xFF];
        ms5611.transfer_in_place(&mut buf).unwrap();

        prom[i as usize] = ((buf[1] as u16) << 8) | (buf[2] as u16);
    }

    let c = Coefficients { data: prom };

    if c.check_crc() {
        ufmt::uwriteln!(s, "CRC of MS5611 PROM correct!").unwrap();
    }

    ufmt::uwrite!(s, "Calibration coefficients: ").unwrap();
    for b in prom {
        ufmt::uwrite!(s, "{:04x} ", b).unwrap();
    }
    ufmt::uwriteln!(s, "").unwrap();

    loop {}
}

/// MSP5611 default factory coefficients
#[derive(ufmt::derive::uDebug, Debug, Default)]
pub struct Coefficients {
    data: [u16; 8],
}

#[allow(non_camel_case_types)]
pub enum CoefficientsAddr {
    MANUFACTURER = 0x0,
    COEFF_1 = 0x2,
    COEFF_2 = 0x4,
    COEFF_3 = 0x6,
    COEFF_4 = 0x8,
    COEFF_5 = 0xA,
    COEFF_6 = 0xC,
    CRC = 0xE,
}

impl Coefficients {
    pub fn get_data(&self, addr: CoefficientsAddr) -> u16 {
        self.data[addr as usize >> 1]
    }

    fn get_crc(&self) -> u8 {
        (self.get_data(CoefficientsAddr::CRC) & 0xF) as u8
    }

    pub fn check_crc(&self) -> bool {
        let mut crc: u16 = 0;
        let data_crc = self.get_crc() as u16;
        for item in self.data[..self.data.len() - 1].iter() {
            crc = Self::crc_coefficient(crc, item);
        }
        crc = Self::crc_coefficient(crc, &(self.get_data(CoefficientsAddr::CRC) & 0xFF00));

        crc = (crc >> 12) & 0xF;
        crc == data_crc
    }

    fn crc_coefficient(crc: u16, coefficient: &u16) -> u16 {
        let mut crc = crc;
        crc ^= (coefficient >> 8) & 0xFFu16;
        crc = Self::crc_round(crc);
        crc ^= coefficient & 0xFF;
        crc = Self::crc_round(crc);
        crc
    }

    fn crc_round(crc: u16) -> u16 {
        let mut crc = crc;
        for _ in (1..9).rev() {
            crc = if (crc & 0x8000) > 0 {
                (crc << 1) ^ 0x3000
            } else {
                crc << 1
            }
        }
        crc
    }
}
