//! Panic-Verhalten im Kernel.
//!
//! In `no_std`-Kernen muss ein eigener `#[panic_handler]` definiert werden.

use core::panic::PanicInfo;

use crate::{arch, console};

/// Gibt eine einfache Fehlermeldung aus und hält das System an.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    console::write_line(b"Kernel panic.");
    arch::x86_64::hang()
}
