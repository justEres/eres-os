#![no_std]

mod arch;
mod console;
mod panic_handler;

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    console::write_line(b"Eres OS: Rust kernel reached long mode.");
    arch::x86_64::halt_loop();
}
