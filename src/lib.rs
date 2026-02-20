#![cfg_attr(eres_kernel, no_std)]
#![cfg_attr(not(eres_kernel), allow(dead_code))]

//! Kernellib von **Eres OS**.
//!
//! Diese Datei ist der Einstiegspunkt für den eigentlichen Kernel (wenn `eres_kernel`
//! aktiv ist) und enthält zusätzlich kleine Smoke-Tests für wichtige Subsysteme.
//!
//! Nützliche Hintergründe:
//! - Boot-Prozess allgemein: <https://wiki.osdev.org/Boot_Sequence>
//! - Long Mode (x86_64): <https://wiki.osdev.org/Setting_Up_Long_Mode>
//! - Speicherverwaltung: <https://wiki.osdev.org/Memory_Management>

extern crate alloc;

mod arch;
mod console;
mod memory;
#[cfg(eres_kernel)]
mod panic_handler;
mod shell;

/// Einstiegspunkt, den der Bootloader nach dem Wechsel in den 64-Bit-Modus aufruft.
///
/// Ablauf in groben Schritten:
/// 1. Bildschirmausgabe initialisieren.
/// 2. Boot-Informationen (v. a. Speicherkarte) übernehmen.
/// 3. Frame-Allocator und Heap initialisieren.
/// 4. Interrupts (IDT/PIC/PIT) aktivieren.
/// 5. Tastatur-/Shell-Loop starten.
#[cfg(eres_kernel)]
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(boot_info_ptr: *const memory::bootinfo::BootInfoRaw) -> ! {
    console::clear();
    console::write_line(b"Eres OS: Rust kernel reached long mode.");

    // Vom Bootloader übergebene Daten global registrieren.
    memory::bootinfo::set_boot_info(boot_info_ptr);

    if let Some(info) = memory::bootinfo::boot_info() {
        if info.entries().is_empty() {
            console::write_line(b"Eres OS: boot info map empty.");
        } else {
            console::write_line(b"Eres OS: boot info map OK.");

            // Physische Seitenrahmen aus der BIOS/UEFI-Speicherkarte verwalten.
            memory::frame_allocator::init_from_memory_map(info.entries());
            if memory::frame_allocator::alloc_frame().is_some() {
                console::write_line(b"Eres OS: frame allocator OK.");
            } else {
                console::write_line(b"Eres OS: frame allocator empty.");
            }

            // Dynamische Allokationen (`Vec`, `Box`, ...) vorbereiten.
            memory::heap::init();
            heap_smoke_test();
        }
    } else {
        console::write_line(b"Eres OS: boot info invalid.");
    }

    // IDT/PIC/PIT aufsetzen und danach CPU-Interrupt-Flag aktivieren.
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

/// Sehr einfacher Tastatur-Selbsttest mit künstlich eingespeisten Scancodes.
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

    // QWERTZ-Prüfung: Z und Y sind gegenüber QWERTY vertauscht.
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

/// Mini-Test für den Heap mittels `Vec`.
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
