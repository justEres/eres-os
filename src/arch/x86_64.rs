//! x86_64-nahe CPU-Steuerfunktionen.
//!
//! Referenzen:
//! - Interrupt-Flag (IF): <https://wiki.osdev.org/Interrupt>
//! - `hlt`: <https://www.felixcloutier.com/x86/hlt>
//! - Reboot über Tastaturcontroller: <https://wiki.osdev.org/Reboot>

#[cfg(eres_kernel)]
use core::arch::asm;

/// Interrupt- und Ausnahmebehandlung.
pub mod interrupts;
mod io;
/// PS/2-Tastatur-Dekodierung und Eingabepuffer.
pub mod keyboard;
mod pic;
/// Programmable Interval Timer (Systemtakte).
pub mod pit;

/// Hält die CPU bis zum nächsten Interrupt an (`hlt`).
#[inline]
pub fn halt() {
    #[cfg(eres_kernel)]
    unsafe {
        asm!("hlt");
    }

    #[cfg(not(eres_kernel))]
    core::hint::spin_loop();
}

/// Deaktiviert maskierbare Interrupts (`cli`).
#[inline]
pub fn disable_interrupts() {
    #[cfg(eres_kernel)]
    unsafe {
        asm!("cli", options(nomem, nostack, preserves_flags));
    }
}

/// Aktiviert maskierbare Interrupts (`sti`).
#[inline]
pub fn enable_interrupts() {
    #[cfg(eres_kernel)]
    unsafe {
        asm!("sti", options(nomem, nostack, preserves_flags));
    }
}

/// Prüft, ob das CPU-Interrupt-Flag aktuell gesetzt ist.
#[inline]
pub fn interrupts_enabled() -> bool {
    #[cfg(not(eres_kernel))]
    {
        false
    }

    #[cfg(eres_kernel)]
    {
        let rflags: u64;
        unsafe {
            asm!("pushfq; pop {}", out(reg) rflags, options(nomem, preserves_flags));
        }
        (rflags & (1 << 9)) != 0
    }
}

/// Merkt sich den IF-Status und deaktiviert anschließend Interrupts.
#[inline]
pub fn save_and_disable_interrupts() -> bool {
    let was_enabled = interrupts_enabled();
    disable_interrupts();
    was_enabled
}

/// Stellt den vorherigen Interrupt-Status wieder her.
#[inline]
pub fn restore_interrupts(was_enabled: bool) {
    if was_enabled {
        enable_interrupts();
    }
}

/// Endlosschleife mit `hlt`.
pub fn halt_loop() -> ! {
    loop {
        halt();
    }
}

/// Stoppt das System dauerhaft (Interrupts aus + Halt-Loop).
pub fn hang() -> ! {
    disable_interrupts();
    halt_loop();
}

/// Versucht einen Hardware-Reboot anzustoßen.
pub fn reboot() -> ! {
    #[cfg(not(eres_kernel))]
    {
        halt_loop();
    }

    #[cfg(eres_kernel)]
    {
        disable_interrupts();
        io::outb(0x64, 0xFE);
        halt_loop();
    }
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
