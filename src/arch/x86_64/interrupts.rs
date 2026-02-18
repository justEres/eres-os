use core::arch::{asm, global_asm};
use core::mem::size_of;

use crate::{arch, console};

use super::{keyboard, pic};

const IDT_ENTRIES: usize = 256;
const KERNEL_CODE_SELECTOR: u16 = 0x18;
const INTERRUPT_GATE_FLAGS: u8 = 0x8E;

const IRQ_BASE: u8 = pic::PIC1_OFFSET;
const IRQ_KEYBOARD: u8 = IRQ_BASE + 1;

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    flags: u8,
    offset_mid: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    const fn missing() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            flags: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    fn set_handler(&mut self, handler: unsafe extern "C" fn()) {
        let addr = handler as usize as u64;
        self.offset_low = addr as u16;
        self.selector = KERNEL_CODE_SELECTOR;
        self.ist = 0;
        self.flags = INTERRUPT_GATE_FLAGS;
        self.offset_mid = (addr >> 16) as u16;
        self.offset_high = (addr >> 32) as u32;
        self.reserved = 0;
    }
}

#[repr(C, packed)]
struct IdtPointer {
    limit: u16,
    base: u64,
}

static mut IDT: [IdtEntry; IDT_ENTRIES] = [IdtEntry::missing(); IDT_ENTRIES];

unsafe extern "C" {
    fn isr_divide_by_zero();
    fn isr_invalid_opcode();
    fn isr_double_fault();
    fn isr_general_protection_fault();
    fn isr_page_fault();
    fn isr_irq1_keyboard();
}

pub fn init() {
    arch::x86_64::disable_interrupts();

    unsafe {
        set_gate(0, isr_divide_by_zero);
        set_gate(6, isr_invalid_opcode);
        set_gate(8, isr_double_fault);
        set_gate(13, isr_general_protection_fault);
        set_gate(14, isr_page_fault);
        set_gate(IRQ_KEYBOARD, isr_irq1_keyboard);
        load_idt();
    }

    pic::remap();
    pic::set_masks(0b1111_1101, 0xff);
}

unsafe fn set_gate(index: u8, handler: unsafe extern "C" fn()) {
    unsafe {
        IDT[index as usize].set_handler(handler);
    }
}

unsafe fn load_idt() {
    let idtr = IdtPointer {
        limit: (size_of::<IdtEntry>() * IDT_ENTRIES - 1) as u16,
        base: (&raw const IDT) as *const _ as u64,
    };

    unsafe {
        asm!(
            "lidt [{0}]",
            in(reg) &idtr,
            options(readonly, nostack, preserves_flags)
        );
    }
}

#[unsafe(no_mangle)]
extern "C" fn interrupt_dispatch(vector: u64, error_code: u64) {
    match vector as u8 {
        0 => fatal_exception(b"EXC: divide by zero"),
        6 => fatal_exception(b"EXC: invalid opcode"),
        8 => fatal_exception(b"EXC: double fault"),
        13 => {
            let _ = error_code;
            fatal_exception(b"EXC: general protection fault")
        }
        14 => {
            let _ = read_cr2();
            let _ = error_code;
            fatal_exception(b"EXC: page fault")
        }
        IRQ_KEYBOARD => {
            keyboard::handle_irq();
        }
        _ => fatal_exception(b"EXC: unhandled vector"),
    }

    if (IRQ_BASE..IRQ_BASE + 16).contains(&(vector as u8)) {
        pic::send_eoi((vector as u8) - IRQ_BASE);
    }
}

fn fatal_exception(message: &[u8]) -> ! {
    console::write_line(message);
    arch::x86_64::hang();
}

fn read_cr2() -> u64 {
    let value: u64;
    unsafe {
        asm!("mov {}, cr2", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

global_asm!(
    r#"
.macro ISR_NOERR handler_name, vector
.global \handler_name
\handler_name:
    push 0
    push \vector
    jmp isr_common
.endm

.macro ISR_ERR handler_name, vector
.global \handler_name
\handler_name:
    push \vector
    jmp isr_common
.endm

.global isr_common
isr_common:
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15

    mov rdi, [rsp + 120]
    mov rsi, [rsp + 128]
    call interrupt_dispatch

    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax

    add rsp, 16
    iretq

ISR_NOERR isr_divide_by_zero, 0
ISR_NOERR isr_invalid_opcode, 6
ISR_ERR   isr_double_fault, 8
ISR_ERR   isr_general_protection_fault, 13
ISR_ERR   isr_page_fault, 14
ISR_NOERR isr_irq1_keyboard, 33
"#
);
