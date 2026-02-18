mod debugcon;
mod vga;

pub fn clear() {
    vga::clear();
}

pub fn write_byte(byte: u8) {
    vga::write_byte(byte);
    debugcon::write_bytes(&[byte]);
}

pub fn backspace() {
    vga::backspace();
}

pub fn write_str(msg: &[u8]) {
    vga::write_bytes(msg);
    debugcon::write_bytes(msg);
}

pub fn write_line(msg: &[u8]) {
    write_str(msg);
    write_byte(b'\n');
}
