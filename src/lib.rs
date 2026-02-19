#![cfg_attr(eres_kernel, no_std)]
#![cfg_attr(not(eres_kernel), allow(dead_code))]

mod arch;
mod console;
mod memory;
#[cfg(eres_kernel)]
mod panic_handler;
mod shell;

#[cfg(eres_kernel)]
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(boot_info_ptr: *const memory::bootinfo::BootInfoRaw) -> ! {
    console::clear();
    console::write_line(b"Eres OS: Rust kernel reached long mode.");
    memory::bootinfo::set_boot_info(boot_info_ptr);
    if let Some(info) = memory::bootinfo::boot_info() {
        if info.entries().is_empty() {
            console::write_line(b"Eres OS: boot info map empty.");
        } else {
            console::write_line(b"Eres OS: boot info map OK.");
            memory::frame_allocator::init_from_memory_map(info.entries());
            if memory::frame_allocator::alloc_frame().is_some() {
                console::write_line(b"Eres OS: frame allocator OK.");
            } else {
                console::write_line(b"Eres OS: frame allocator empty.");
            }
        }
    } else {
        console::write_line(b"Eres OS: boot info invalid.");
    }
    arch::x86_64::interrupts::init();
    arch::x86_64::enable_interrupts();
    console::write_line(b"Eres OS: IDT/PIC initialized.");

    #[cfg(feature = "qemu-test")]
    {
        keyboard_smoke_test();
        if shell::run_command_self_tests() {
            console::write_line(b"Eres OS: command tests OK.");
            arch::x86_64::qemu_exit_success();
        } else {
            console::write_line(b"Eres OS: command tests FAILED.");
            arch::x86_64::qemu_exit_failure();
        }
    }

    #[cfg(not(feature = "qemu-test"))]
    {
        keyboard_smoke_test();
        shell::run();
    }
}

#[cfg(eres_kernel)]
fn keyboard_smoke_test() {
    use arch::x86_64::keyboard;

    keyboard::clear_buffer();
    for scancode in [0x23_u8, 0x12, 0x26, 0x26, 0x18] {
        keyboard::inject_scancode(scancode);
    }

    let mut ok = true;
    for expected in b"hello" {
        if keyboard::try_read_char() != Some(*expected) {
            ok = false;
            break;
        }
    }

    if keyboard::try_read_char().is_some() {
        ok = false;
    }

    keyboard::inject_scancode(0x15);
    if keyboard::try_read_char() != Some(b'z') {
        ok = false;
    }

    keyboard::inject_scancode(0x2c);
    if keyboard::try_read_char() != Some(b'y') {
        ok = false;
    }

    keyboard::clear_buffer();

    if ok {
        console::write_line(b"Eres OS: keyboard decode OK.");
    } else {
        console::write_line(b"Eres OS: keyboard decode FAILED.");
    }
}
