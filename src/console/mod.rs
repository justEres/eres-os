mod debugcon;
mod vga;

pub fn write_line(msg: &[u8]) {
    vga::write_bytes(msg);
    debugcon::write_bytes(msg);
}
