use core::arch::asm;

pub mod interrupts;
pub mod keyboard;
pub mod pit;
mod io;
mod pic;

#[inline]
pub fn halt() {
    unsafe {
        asm!("hlt");
    }
}

#[inline]
pub fn disable_interrupts() {
    unsafe {
        asm!("cli", options(nomem, nostack, preserves_flags));
    }
}

#[inline]
pub fn enable_interrupts() {
    unsafe {
        asm!("sti", options(nomem, nostack, preserves_flags));
    }
}

#[inline]
pub fn interrupts_enabled() -> bool {
    let rflags: u64;
    unsafe {
        asm!("pushfq; pop {}", out(reg) rflags, options(nomem, preserves_flags));
    }
    (rflags & (1 << 9)) != 0
}

#[inline]
pub fn save_and_disable_interrupts() -> bool {
    let was_enabled = interrupts_enabled();
    disable_interrupts();
    was_enabled
}

#[inline]
pub fn restore_interrupts(was_enabled: bool) {
    if was_enabled {
        enable_interrupts();
    }
}

pub fn halt_loop() -> ! {
    loop {
        halt();
    }
}

pub fn hang() -> ! {
    disable_interrupts();
    halt_loop();
}

pub fn reboot() -> ! {
    disable_interrupts();
    io::outb(0x64, 0xFE);
    halt_loop();
}

#[cfg(feature = "qemu-test")]
pub fn qemu_exit_success() -> ! {
    qemu_exit(0x10);
}

#[cfg(feature = "qemu-test")]
pub fn qemu_exit_failure() -> ! {
    qemu_exit(0x11);
}

#[cfg(feature = "qemu-test")]
fn qemu_exit(code: u32) -> ! {
    disable_interrupts();
    unsafe {
        asm!(
            "out dx, eax",
            in("dx") 0xf4_u16,
            in("eax") code,
            options(nomem, nostack, preserves_flags)
        );
    }
    halt_loop();
}
