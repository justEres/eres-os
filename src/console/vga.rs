const VGA_TEXT_BUFFER: *mut u8 = 0xb8000 as *mut u8;
const VGA_WHITE_ON_BLACK: u8 = 0x0f;

pub fn write_bytes(msg: &[u8]) {
    for (i, byte) in msg.iter().copied().enumerate() {
        unsafe {
            *VGA_TEXT_BUFFER.add(i * 2) = byte;
            *VGA_TEXT_BUFFER.add(i * 2 + 1) = VGA_WHITE_ON_BLACK;
        }
    }
}
