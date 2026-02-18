use core::arch::asm;

pub fn write_bytes(msg: &[u8]) {
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
