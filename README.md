atxtiny-hal
===========

Hardware abstraction layer for the tinyAVR 0- and 1-series microcontrollers. The 2-series is not supported yet.

Possibly applicable to other modern AVRs based on the xmega peripherals as well as long as somebody does the work of implementing macros for peripheral instantiation and actual definitions of the peripherals of the different controllers.

I built this by myself for private projects by looking at similar HAL crates for the STM32 series or the avr-hal from rahix.

## Supported controllers

Select exactly one with a Cargo feature:

* 0-series: attiny202, attiny204, attiny402, attiny404, attiny804, attiny1604, attiny1606
* 1-series: attiny212, attiny214, attiny412, attiny414, attiny416, attiny417, attiny816, attiny817, attiny1614, attiny1617, attiny3217

## Supported peripherals:

* CLKCTRL - Clock controller
* NVMCTRL - Nonvolatile Memory Controller
* SLPCTRL - Sleep controller
* RSTCTRL - Reset controller
* BOD - Brownout detector
* VREF - Voltage reference
* AC - Analog comparator
* DAC - Digital to Analog converter
* GPIO - General Purpose I/O
* PORTMUX - Port Multiplexer
* WDT - Watchdog Timer
* USART - Universal Synchronous and Asynchronous Receiver and Transmitter
    * only in asynchronous serial mode
* SPI - Serial Peripheral Interface
    * only unbuffered host mode
* TWI - Two-Wire Interface
    * only master mode
* CCL
* TCA
    * normal and split mode
* TCB
    * Periodic mode
    * PWM in 8 bit mode
* CPUINT
* RTC

## Missing peripheral support:

* TCD
    * including event generators
* ADC
    * including event generators
* PIT
    * including event generators
* CRCSCAN
* (PTC) - very proprietary and undocumented

## General TODOs:

* Implement SPI `embedded_hal_nb` traits

* FIXME: `IntoMuxedPinset::mux` consumes the PORTMUX routing token, but only EVOUT pinsets store it. All other pinsets (USART, SPI, TWI, TCA, TCB, CCL) drop it, so `free()` neither resets the routing bit nor returns the token, and the function can't be muxed again

* FIXME: TCA, TCB and CCL output pins still enter `mux` as `Output<Stateless>`; move them to `Peripheral<PERI>` to avoid type conflicts like USART, SPI, TWI and EVOUT already do

* fugit should support 16 bit time types on AVR, 32 bits are wasteful as most timers only have 16 bits resolution anyway
    * is this even feasible?

* CLKCTRL
    * allow to configure 32khz clock either from internal source, external crystal or external clock
    * pass this clock to RTC to make sure if external 32KHz clock is used that it is actually configured and works

* EVSYS
    * More event generators and users for a few peripherals
    * Software strobes
    * Use the type-system the declare generator and user constants somehow
    * Allow multiple users per channel - see FIXME in example when assigning users
    * It's really only applicable to the 1-series AVRs, 2-series is very different

* RTC
    * Event system
    * External 32KHz clock support
    * Compare channel support (see general timer TODO)

* tinyAVR 2-series support
    * EVSYS, PORTMUX, VREF, ADC and CCL have different register layouts

* attiny814, attiny1616, attiny3216 are missing from avr-device

* Custom startup code
    * Right now we depend on avr-libc for crt0 stuff, linking and interrupt vectors
    * Ideally we want our own code that does this just like for the ARM with the cortex-m crate for example

## TODO CCL:

* The API is suboptimal
* Passing input pins into it is meh
* The timers solve this problem with tuples and macros, maybe do the same on CCL?

## TODO Timers:

* General
    * RUNSTDBY support?
    * Allow for dummy pins to be able to use compare channels with EVSYS later
    * split PWM trait in PWM and compare channel traits as we have timers that have compare channels, but not assigned PWM outputs like the RTC
    * Implement pwm_custom for an FTimer

* TCB
    * oneshot mode
    * delay in oneshot mode
    * input capture mode
    * timeout check mode

* TCD
    * completely different beast, currently no support at all


# Setup

## Patched dependency crates

* atdf2svd
  * Needed to handle register map modes - [patch has been merged upstream](https://github.com/Rahix/atdf2svd/pull/48)

* svd2rust
  * Support for CCP protected registers - [patch has been merged upstream](https://github.com/rust-embedded/svd2rust/pull/784)

* avr-device
  * tinyAVR 0/1-series support - [fork](https://github.com/G33KatWork/avr-device)

Both are pulled from pinned git revisions via `[patch.crates-io]` in `Cargo.toml`.

## Installing stuff

1. You need a patched version of avr-libc which contains support for the newer ATtiny chips. On Arch Linux there is an [AUR package](https://aur.archlinux.org/packages/avr-libc-avrxmega3-svn) available

2. Install the rest of the AVR toolchain from your distro like binutils

3. Install rustup. `rust-toolchain.toml` pins the nightly toolchain; `core` is built with `build-std` for the `avr-none` target.

4. Clone the repo and build the examples. `.cargo/config.toml` defaults to `-C target-cpu=attiny817`; override it via `RUSTFLAGS` for other chips:
```
git clone https://github.com/G33KatWork/atxtiny-hal.git
cd atxtiny-hal
cargo build --release --examples --features attiny817
RUSTFLAGS="-C target-cpu=attiny1614" cargo build --release --examples --features attiny1614
```

`scripts/build-all-chips.sh` builds the library and all examples for every supported chip. Examples whose peripherals a chip lacks are skipped.

## Programming things using pymcuprog

```
# Installing pymcuprog
python -m venv env
source env/bin/activate
pip install pymcuprog

# Building and flashing stuff
cargo build --release --examples --features attiny817 && avr-objcopy -O ihex target/avr-none/release/examples/serial.elf /tmp/avr.hex && pymcuprog -d attiny817 -t uart -u /dev/ttyUSB0 erase && pymcuprog -d attiny817 -t uart -u /dev/ttyUSB0 write -f /tmp/avr.hex
```

## Other possible programmers

* https://daumemo.com/diy-updi-usb-programmer-which-can-be-made-with-cheap-hardware/
