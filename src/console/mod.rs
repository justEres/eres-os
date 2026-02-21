//! Konsolenabstraktion.
//!
//! Schreibt parallel in den VGA-Textmodus und auf Port `0xE9` (Debug-Ausgabe für Emulatoren).

mod debugcon;
mod vga;

/// Löscht den sichtbaren Textbildschirm.
pub fn clear() {
    vga::clear();
}

/// Schreibt genau ein Byte auf beide Ausgabekanäle.
pub fn write_byte(byte: u8) {
    vga::write_byte(byte);
    debugcon::write_bytes(&[byte]);
}

/// Führt eine Backspace-Operation aus.
pub fn backspace() {
    vga::backspace();
    debugcon::write_bytes(&[8, b' ', 8]);
}

/// Schreibt ein Byte-Slice ohne automatischen Zeilenumbruch.
pub fn write_str(msg: &[u8]) {
    vga::write_bytes(msg);
    debugcon::write_bytes(msg);
}

/// Schreibt ein Byte-Slice plus `\n`.
pub fn write_line(msg: &[u8]) {
    write_str(msg);
    write_byte(b'\n');
}

/// Gibt eine `u64` als hexadezimale Zahl mit Präfix `0x` aus.
pub fn write_hex_u64(value: u64) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    write_str(b"0x");
    for shift in (0..16).rev() {
        let nibble = ((value >> (shift * 4)) & 0xF) as usize;
        write_byte(HEX[nibble]);
    }
}

/// Gibt eine `u64` als Dezimalzahl aus.
pub fn write_u64(mut value: u64) {
    if value == 0 {
        write_byte(b'0');
        return;
    }

    let mut digits = [0_u8; 20];
    let mut len = 0;
    while value > 0 {
        digits[len] = b'0' + (value % 10) as u8;
        len += 1;
        value /= 10;
    }

    while len > 0 {
        len -= 1;
        write_byte(digits[len]);
    }
}
