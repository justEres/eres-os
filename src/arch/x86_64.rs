use core::arch::asm;

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

pub fn halt_loop() -> ! {
    loop {
        halt();
    }
}

pub fn hang() -> ! {
    disable_interrupts();
    halt_loop();
}
