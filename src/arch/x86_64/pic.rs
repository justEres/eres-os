//! Steuerung des 8259 PIC (Remapping, Masks, EOI).
//!
//! Hintergrund: <https://wiki.osdev.org/8259_PIC>

use super::io::{inb, io_wait, outb};

const PIC1_COMMAND: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_COMMAND: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;
const PIC_EOI: u8 = 0x20;

/// Startvektor für IRQs des Master-PIC nach dem Remap.
pub const PIC1_OFFSET: u8 = 0x20;
/// Startvektor für IRQs des Slave-PIC nach dem Remap.
pub const PIC2_OFFSET: u8 = 0x28;

/// Legt IRQ-Vektoren in den Bereich 0x20.. um (weg von CPU-Exceptions).
pub fn remap() {
    let master_mask = inb(PIC1_DATA);
    let slave_mask = inb(PIC2_DATA);

    outb(PIC1_COMMAND, 0x11);
    io_wait();
    outb(PIC2_COMMAND, 0x11);
    io_wait();

    outb(PIC1_DATA, PIC1_OFFSET);
    io_wait();
    outb(PIC2_DATA, PIC2_OFFSET);
    io_wait();

    outb(PIC1_DATA, 4);
    io_wait();
    outb(PIC2_DATA, 2);
    io_wait();

    outb(PIC1_DATA, 0x01);
    io_wait();
    outb(PIC2_DATA, 0x01);
    io_wait();

    outb(PIC1_DATA, master_mask);
    outb(PIC2_DATA, slave_mask);
}

/// Setzt Interrupt-Masken für Master und Slave PIC.
pub fn set_masks(master: u8, slave: u8) {
    outb(PIC1_DATA, master);
    outb(PIC2_DATA, slave);
}

/// Sendet End-of-Interrupt an den/die zuständigen PIC(s).
pub fn send_eoi(irq: u8) {
    if irq >= 8 {
        outb(PIC2_COMMAND, PIC_EOI);
    }
    outb(PIC1_COMMAND, PIC_EOI);
}
