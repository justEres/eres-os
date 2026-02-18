const WIDTH: usize = 80;
const HEIGHT: usize = 25;
const VGA_TEXT_BUFFER: *mut u8 = 0xb8000 as *mut u8;
const VGA_WHITE_ON_BLACK: u8 = 0x0f;

static mut COLUMN: usize = 0;
static mut ROW: usize = 0;

pub fn clear() {
    unsafe {
        for row in 0..HEIGHT {
            for col in 0..WIDTH {
                write_cell(row, col, b' ', VGA_WHITE_ON_BLACK);
            }
        }
        ROW = 0;
        COLUMN = 0;
    }
}

pub fn write_byte(byte: u8) {
    unsafe {
        match byte {
            b'\n' => newline(),
            8 => backspace_inner(),
            b => {
                if COLUMN >= WIDTH {
                    newline();
                }
                write_cell(ROW, COLUMN, b, VGA_WHITE_ON_BLACK);
                COLUMN += 1;
            }
        }
    }
}

pub fn write_bytes(msg: &[u8]) {
    for byte in msg {
        write_byte(*byte);
    }
}

pub fn backspace() {
    unsafe {
        backspace_inner();
    }
}

unsafe fn backspace_inner() {
    unsafe {
        if COLUMN > 0 {
            COLUMN -= 1;
            write_cell(ROW, COLUMN, b' ', VGA_WHITE_ON_BLACK);
        }
    }
}

unsafe fn newline() {
    unsafe {
        COLUMN = 0;
        if ROW + 1 >= HEIGHT {
            scroll_one_line();
        } else {
            ROW += 1;
        }
    }
}

unsafe fn scroll_one_line() {
    unsafe {
        for row in 1..HEIGHT {
            for col in 0..WIDTH {
                let from = (row * WIDTH + col) * 2;
                let to = ((row - 1) * WIDTH + col) * 2;
                *VGA_TEXT_BUFFER.add(to) = *VGA_TEXT_BUFFER.add(from);
                *VGA_TEXT_BUFFER.add(to + 1) = *VGA_TEXT_BUFFER.add(from + 1);
            }
        }

        for col in 0..WIDTH {
            write_cell(HEIGHT - 1, col, b' ', VGA_WHITE_ON_BLACK);
        }

        ROW = HEIGHT - 1;
    }
}

unsafe fn write_cell(row: usize, col: usize, byte: u8, attr: u8) {
    let offset = (row * WIDTH + col) * 2;
    unsafe {
        *VGA_TEXT_BUFFER.add(offset) = byte;
        *VGA_TEXT_BUFFER.add(offset + 1) = attr;
    }
}
