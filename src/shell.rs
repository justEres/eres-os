use crate::{arch, console};
use core::arch::asm;

const MAX_LINE: usize = 128;

pub fn run() -> ! {
    let mut line_buf = [0_u8; MAX_LINE];
    let mut len = 0_usize;

    console::write_line(b"Type 'help' for commands.");
    prompt();

    loop {
        if let Some(ch) = arch::x86_64::keyboard::try_read_char() {
            match ch {
                b'\n' => {
                    console::write_byte(b'\n');
                    execute_command(&line_buf[..len]);
                    len = 0;
                    prompt();
                }
                8 => {
                    if len > 0 {
                        len -= 1;
                        console::backspace();
                    }
                }
                b if is_printable_ascii(b) => {
                    if len < MAX_LINE {
                        line_buf[len] = b;
                        len += 1;
                        console::write_byte(b);
                    }
                }
                _ => {}
            }
        } else {
            arch::x86_64::halt();
        }
    }
}

fn prompt() {
    console::write_str(b"> ");
}

fn execute_command(line: &[u8]) {
    if line.is_empty() {
        return;
    }

    if line == b"help" {
        console::write_line(b"commands: help echo clear panic halt reboot");
        return;
    }

    if line == b"clear" {
        console::clear();
        return;
    }

    if line == b"panic" {
        unsafe {
            asm!("ud2", options(nomem, nostack, preserves_flags));
        }
        arch::x86_64::hang();
    }

    if line == b"halt" {
        console::write_line(b"Halting CPU.");
        arch::x86_64::hang();
    }

    if line == b"reboot" {
        console::write_line(b"Rebooting.");
        arch::x86_64::reboot();
    }

    if let Some(rest) = line.strip_prefix(b"echo ") {
        console::write_line(rest);
        return;
    }

    console::write_line(b"unknown command");
}

fn is_printable_ascii(byte: u8) -> bool {
    (0x20..=0x7E).contains(&byte)
}
