//! # Serial port panic handler

use core::fmt::Write;
use core::panic::PanicInfo;
use ufmt::uWrite;

struct WriteWrapper<'a, W: uWrite>(&'a mut W);

impl<'a, W: uWrite> Write for WriteWrapper<'a, W> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.0.write_str(s).map_err(|_| core::fmt::Error)
    }
}

/// Called internally by the panic handler.
pub fn _print_panic<W: uWrite>(w: &mut W, info: &PanicInfo) {
    if cfg!(feature = "fullpanic") {
        _ = core::fmt::write(&mut WriteWrapper(w), format_args!("{}", info));
    } else {
        if let Some(location) = info.location() {
            _ = ufmt::uwrite!(
                w,
                "Panic at {}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            );
        } else {
            _ = ufmt::uwrite!(w, "Panic");
        }
    }
}

/// Implements the panic handler. You need to call this for the package to work.
///
/// This macro defines the panic handler, as well as a function called `share_serial_port_with_panic`.
/// That function takes an argument of the given `$type` and returns a `&'static mut $type`.
///
#[macro_export]
macro_rules! impl_panic_handler {
    ($type:ty) => {
        static mut PANIC_PORT: Option<$type> = None;

        #[inline(never)]
        #[panic_handler]
        fn panic(info: &::core::panic::PanicInfo) -> ! {
            use ::atxtiny_hal::embedded_io::Write;

            unsafe { avr_device::interrupt::disable() };

            if let Some(panic_port) = unsafe { PANIC_PORT.as_mut() } {
                _ = panic_port.flush();
                ::atxtiny_hal::panic_serial::_print_panic(panic_port, info);
            }
            loop {
                ::core::sync::atomic::compiler_fence(::core::sync::atomic::Ordering::SeqCst);
            }
        }

        pub fn share_serial_port_with_panic(port: $type) -> &'static mut $type {
            unsafe {
                PANIC_PORT = Some(port);
                PANIC_PORT.as_mut().unwrap()
            }
        }
    };
}
