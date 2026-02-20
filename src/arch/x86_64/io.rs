//! Primitive Port-I/O-Helfer (`inb`/`outb`).
//!
//! Hintergrund: <https://wiki.osdev.org/Port_IO>

use core::arch::asm;

#[inline]
/// Schreibt ein Byte auf einen I/O-Port.
pub fn outb(port: u16, value: u8) {
    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }
}

#[inline]
/// Liest ein Byte von einem I/O-Port.
pub fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe {
        asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

#[inline]
/// Sehr kurze I/O-Warteoperation über Port `0x80`.
pub fn io_wait() {
    outb(0x80, 0);
}
