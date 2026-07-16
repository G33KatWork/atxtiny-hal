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
/// The returned reference is the *only* handle to the port; call
/// `share_serial_port_with_panic` at most once. When a panic fires, the
/// handler takes the port out of the shared slot for its final output.
#[macro_export]
macro_rules! impl_panic_handler {
    ($type:ty) => {
        static mut PANIC_PORT: Option<$type> = None;

        #[inline(never)]
        #[panic_handler]
        fn panic(info: &::core::panic::PanicInfo) -> ! {
            use ::atxtiny_hal::embedded_io::Write;

            ::avr_device::interrupt::disable();

            // Move the port out of the static instead of forming a second
            // `&mut` next to the one handed out by
            // `share_serial_port_with_panic` - two live mutable references
            // would be immediate UB. Going through a raw pointer only
            // *invalidates* the user's reference, which is sound because it
            // can never be used again: interrupts are off and this handler
            // diverges.
            //
            // Taking (rather than borrowing) the port also leaves `None`
            // behind, so if the printing below panics again - e.g. a broken
            // `Display` impl in the payload under `fullpanic` - the
            // re-entered handler falls through to the halt loop instead of
            // recursing until stack overflow.
            let port = unsafe { (&raw mut PANIC_PORT).replace(None) };

            if let Some(mut panic_port) = port {
                // Let any in-flight transmission of the interrupted program
                // finish, so the panic message isn't interleaved mid-frame.
                _ = panic_port.flush();
                ::atxtiny_hal::panic_serial::_print_panic(&mut panic_port, info);
            }
            loop {
                ::core::sync::atomic::compiler_fence(::core::sync::atomic::Ordering::SeqCst);
            }
        }

        pub fn share_serial_port_with_panic(port: $type) -> &'static mut $type {
            // Store the port and hand out the single long-lived mutable
            // reference to it. All access goes through `&raw mut` so no
            // temporary reference to the `static mut` is ever created
            // (edition-2024 `static_mut_refs`); the panic handler above
            // only ever moves the value out through the same raw pointer.
            unsafe {
                let slot = &raw mut PANIC_PORT;
                slot.write(Some(port));
                // SAFETY: just written as `Some`.
                (*slot).as_mut().unwrap_unchecked()
            }
        }
    };
}
