#![no_std]

use core::arch::asm;
use core::panic::PanicInfo;

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    let message = b"Eres OS: Rust kernel reached long mode.";
    write_vga(message);
    write_debugcon(message);

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    let message = b"Kernel panic.";
    write_vga(message);
    write_debugcon(message);

    loop {
        unsafe {
            asm!("cli; hlt");
        }
    }
}

fn write_vga(msg: &[u8]) {
    let vga = 0xb8000 as *mut u8;

    for (i, byte) in msg.iter().copied().enumerate() {
        unsafe {
            *vga.add(i * 2) = byte;
            *vga.add(i * 2 + 1) = 0x0f;
        }
    }
}

fn write_debugcon(msg: &[u8]) {
    for byte in msg {
        unsafe {
            asm!(
                "out dx, al",
                in("dx") 0xe9_u16,
                in("al") *byte,
                options(nomem, nostack, preserves_flags)
            );
        }
    }
}
