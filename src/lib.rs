#![cfg_attr(eres_kernel, no_std)]
#![cfg_attr(not(eres_kernel), allow(dead_code))]

extern crate alloc;

mod arch;
mod console;
mod fs;
mod memory;
mod storage;
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
            memory::heap::init();
            heap_smoke_test();
            block_device_smoke_test();
            vm_smoke_test();
            fs_smoke_test();
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

#[cfg(eres_kernel)]
fn heap_smoke_test() {
    use alloc::vec::Vec;

    let mut values = Vec::new();
    values.push(1_u8);
    values.push(2_u8);
    values.push(3_u8);

    if values.as_slice() == [1, 2, 3] {
        console::write_line(b"Eres OS: heap allocator OK.");
    } else {
        console::write_line(b"Eres OS: heap allocator FAILED.");
    }
}

#[cfg(eres_kernel)]
fn block_device_smoke_test() {
    use storage::ata_pio::AtaPio;
    use storage::block::BlockDevice;

    let mut dev = AtaPio::primary_master();
    let mut sector = [0_u8; 512];
    match dev.read_sector(0, &mut sector) {
        Ok(()) if sector[510] == 0x55 && sector[511] == 0xAA => {
            console::write_line(b"Eres OS: block device OK.");
        }
        Ok(()) => {
            console::write_line(b"Eres OS: block device invalid signature.");
        }
        Err(_) => {
            console::write_line(b"Eres OS: block device read failed.");
        }
    }
}

#[cfg(eres_kernel)]
fn vm_smoke_test() {
    use memory::vm::Mapper2M;
    let mapper = memory::vm::boot_mapper();
    let entry0 = mapper.entry(0);
    if entry0.is_present() {
        console::write_line(b"Eres OS: vm mapper OK.");
    } else {
        console::write_line(b"Eres OS: vm mapper invalid.");
    }
}

#[cfg(eres_kernel)]
fn fs_smoke_test() {
    use fs::simplefs::SimpleFs;
    use storage::ata_pio::AtaPio;
    use storage::cache::CachedBlockDevice;

    let dev = CachedBlockDevice::new(AtaPio::primary_master(), 16);
    match SimpleFs::mount(dev) {
        Ok(fs) => {
            let _ = fs.superblock();
            console::write_line(b"Eres OS: simplefs mounted.");
        }
        Err(_) => {
            console::write_line(b"Eres OS: simplefs not present.");
        }
    }
}
