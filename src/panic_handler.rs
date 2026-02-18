use core::panic::PanicInfo;

use crate::{arch, console};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    console::write_line(b"Kernel panic.");
    arch::x86_64::hang()
}
