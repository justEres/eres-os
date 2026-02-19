use core::cell::UnsafeCell;

use crate::arch;

use super::io;

const BUFFER_SIZE: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyEvent {
    Char(u8),
    Enter,
    Backspace,
    Up,
    Down,
}

struct KeyboardState {
    shift: bool,
    e0_prefix: bool,
    head: usize,
    tail: usize,
    buffer: [u16; BUFFER_SIZE],
}

impl KeyboardState {
    const fn new() -> Self {
        Self {
            shift: false,
            e0_prefix: false,
            head: 0,
            tail: 0,
            buffer: [0; BUFFER_SIZE],
        }
    }
}

struct KeyboardCell(UnsafeCell<KeyboardState>);

unsafe impl Sync for KeyboardCell {}

static KEYBOARD_STATE: KeyboardCell = KeyboardCell(UnsafeCell::new(KeyboardState::new()));

pub fn handle_irq() {
    let scancode = io::inb(0x60);
    feed_scancode(scancode);
}

pub fn try_read_char() -> Option<u8> {
    match try_read_key() {
        Some(KeyEvent::Char(ch)) => Some(ch),
        Some(KeyEvent::Enter) => Some(b'\n'),
        Some(KeyEvent::Backspace) => Some(8),
        _ => None,
    }
}

pub fn try_read_key() -> Option<KeyEvent> {
    let interrupts_were_enabled = arch::x86_64::save_and_disable_interrupts();

    let result = unsafe {
        let state = &mut *KEYBOARD_STATE.0.get();
        if state.head == state.tail {
            None
        } else {
            let code = state.buffer[state.tail];
            state.tail = (state.tail + 1) % BUFFER_SIZE;
            decode_event(code)
        }
    };

    arch::x86_64::restore_interrupts(interrupts_were_enabled);
    result
}

pub fn inject_scancode(scancode: u8) {
    feed_scancode(scancode);
}

pub fn clear_buffer() {
    let interrupts_were_enabled = arch::x86_64::save_and_disable_interrupts();
    unsafe {
        let state = &mut *KEYBOARD_STATE.0.get();
        state.head = 0;
        state.tail = 0;
    }
    arch::x86_64::restore_interrupts(interrupts_were_enabled);
}

fn feed_scancode(scancode: u8) {
    unsafe {
        let state = &mut *KEYBOARD_STATE.0.get();

        if scancode == 0xE0 {
            state.e0_prefix = true;
            return;
        }

        if state.e0_prefix {
            state.e0_prefix = false;
            if (scancode & 0x80) != 0 {
                return;
            }

            match scancode {
                0x48 => push_event(state, KeyEvent::Up),
                0x50 => push_event(state, KeyEvent::Down),
                _ => {}
            }
            return;
        }

        match scancode {
            0x2A | 0x36 => {
                state.shift = true;
                return;
            }
            0xAA | 0xB6 => {
                state.shift = false;
                return;
            }
            _ => {}
        }

        if (scancode & 0x80) != 0 {
            return;
        }

        if let Some(ch) = decode_scancode(scancode, state.shift) {
            match ch {
                8 => push_event(state, KeyEvent::Backspace),
                b'\n' => push_event(state, KeyEvent::Enter),
                _ => push_event(state, KeyEvent::Char(ch)),
            }
        }
    }
}

fn push_event(state: &mut KeyboardState, event: KeyEvent) {
    let next_head = (state.head + 1) % BUFFER_SIZE;
    if next_head == state.tail {
        return;
    }

    state.buffer[state.head] = encode_event(event);
    state.head = next_head;
}

const KEY_ENTER: u16 = 0x100;
const KEY_BACKSPACE: u16 = 0x101;
const KEY_UP: u16 = 0x102;
const KEY_DOWN: u16 = 0x103;

fn encode_event(event: KeyEvent) -> u16 {
    match event {
        KeyEvent::Char(ch) => ch as u16,
        KeyEvent::Enter => KEY_ENTER,
        KeyEvent::Backspace => KEY_BACKSPACE,
        KeyEvent::Up => KEY_UP,
        KeyEvent::Down => KEY_DOWN,
    }
}

fn decode_event(code: u16) -> Option<KeyEvent> {
    match code {
        0..=0xFF => Some(KeyEvent::Char(code as u8)),
        KEY_ENTER => Some(KeyEvent::Enter),
        KEY_BACKSPACE => Some(KeyEvent::Backspace),
        KEY_UP => Some(KeyEvent::Up),
        KEY_DOWN => Some(KeyEvent::Down),
        _ => None,
    }
}

fn decode_scancode(scancode: u8, shift: bool) -> Option<u8> {
    // German QWERTZ layout with ASCII fallbacks for non-ASCII symbols.
    let ch = match scancode {
        0x01 => 0x1B,
        0x02 => if shift { b'!' } else { b'1' },
        0x03 => if shift { b'"' } else { b'2' },
        0x04 => if shift { b'#' } else { b'3' },
        0x05 => if shift { b'$' } else { b'4' },
        0x06 => if shift { b'%' } else { b'5' },
        0x07 => if shift { b'&' } else { b'6' },
        0x08 => if shift { b'/' } else { b'7' },
        0x09 => if shift { b'(' } else { b'8' },
        0x0A => if shift { b')' } else { b'9' },
        0x0B => if shift { b'=' } else { b'0' },
        0x0C => if shift { b'?' } else { b'-' },
        0x0D => if shift { b'`' } else { b'+' },
        0x0E => 8,
        0x0F => b'\t',
        0x10 => if shift { b'Q' } else { b'q' },
        0x11 => if shift { b'W' } else { b'w' },
        0x12 => if shift { b'E' } else { b'e' },
        0x13 => if shift { b'R' } else { b'r' },
        0x14 => if shift { b'T' } else { b't' },
        0x15 => if shift { b'Z' } else { b'z' },
        0x16 => if shift { b'U' } else { b'u' },
        0x17 => if shift { b'I' } else { b'i' },
        0x18 => if shift { b'O' } else { b'o' },
        0x19 => if shift { b'P' } else { b'p' },
        0x1A => if shift { b'U' } else { b'u' },
        0x1B => if shift { b'+' } else { b'#' },
        0x1C => b'\n',
        0x1E => if shift { b'A' } else { b'a' },
        0x1F => if shift { b'S' } else { b's' },
        0x20 => if shift { b'D' } else { b'd' },
        0x21 => if shift { b'F' } else { b'f' },
        0x22 => if shift { b'G' } else { b'g' },
        0x23 => if shift { b'H' } else { b'h' },
        0x24 => if shift { b'J' } else { b'j' },
        0x25 => if shift { b'K' } else { b'k' },
        0x26 => if shift { b'L' } else { b'l' },
        0x27 => if shift { b':' } else { b';' },
        0x28 => if shift { b'"' } else { b'\'' },
        0x29 => if shift { b'~' } else { b'`' },
        0x2B => if shift { b'*' } else { b'\'' },
        0x2C => if shift { b'Y' } else { b'y' },
        0x2D => if shift { b'X' } else { b'x' },
        0x2E => if shift { b'C' } else { b'c' },
        0x2F => if shift { b'V' } else { b'v' },
        0x30 => if shift { b'B' } else { b'b' },
        0x31 => if shift { b'N' } else { b'n' },
        0x32 => if shift { b'M' } else { b'm' },
        0x33 => if shift { b';' } else { b',' },
        0x34 => if shift { b':' } else { b'.' },
        0x35 => if shift { b'_' } else { b'-' },
        0x39 => b' ',
        _ => return None,
    };

    Some(ch)
}

#[cfg(test)]
mod tests {
    use super::{clear_buffer, inject_scancode, try_read_key, KeyEvent};

    #[test]
    fn decodes_arrow_up_down() {
        clear_buffer();

        inject_scancode(0xE0);
        inject_scancode(0x48);
        inject_scancode(0xE0);
        inject_scancode(0x50);

        assert_eq!(try_read_key(), Some(KeyEvent::Up));
        assert_eq!(try_read_key(), Some(KeyEvent::Down));
    }
}
