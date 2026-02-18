# Eres OS - 64-bit Rust Hobby OS Plan

## 1) Project goals

- Build a 64-bit (x86_64) hobby OS.
- Keep core OS logic in Rust (`no_std`, `no_main`).
- Use assembly only where required:
  - Early boot stages.
  - CPU mode transitions.
  - Small low-level routines (inline asm where needed).
- Boot and test with QEMU on every iteration.

## 2) Today’s milestone

Create a working boot path:

1. Assembly bootloader starts from BIOS boot sector.
2. Bootloader loads/enters 64-bit mode and jumps to Rust kernel entry.
3. Rust kernel executes a visible proof (for example writing to VGA text buffer).
4. Entire binary boots in QEMU reproducibly via one command.

## 3) High-level architecture

- Platform: `x86_64`
- Firmware path for first version: BIOS + boot sector (simple and educational).
- Kernel binary style:
  - `#![no_std]`
  - `#![no_main]`
  - custom panic handler
- Link model:
  - Assembly stage sets up CPU state and jumps to Rust symbol (example: `kernel_main`).
  - A linker script controls physical/virtual layout.
- Output artifact:
  - Bootable disk image (`.img`) run in QEMU.

## 4) Suggested repository structure

```text
eres-os/
  Cargo.toml
  PROJECT_STRUCTURE.md
  rust-toolchain.toml
  .cargo/
    config.toml
  build/
    linker.ld
  boot/
    boot.asm
  src/
    main.rs            # kernel entry + core init
    vga.rs             # early text output
    panic.rs           # panic handler
  target/
  scripts/
    build_image.sh
    run_qemu.sh
```

## 5) Toolchain and build decisions

- Rust nightly (for low-level flags/features as needed).
- Target: custom `x86_64` bare-metal JSON target or `x86_64-unknown-none`.
- Assembler: NASM (or GAS; NASM recommended for clarity).
- Linker: `ld.lld` (or GNU ld), controlled by `build/linker.ld`.
- Image creation:
  - Option A: direct flat binary layout.
  - Option B: staged ELF + objcopy + disk image.
  - Start with the simplest reliable flow, then refactor.

## 6) Concrete build flow (today)

1. Build Rust kernel object/staticlib for bare-metal target.
2. Assemble `boot/boot.asm`.
3. Link boot + kernel with `linker.ld` to final bootable binary.
4. Write binary into disk image (with valid boot signature).
5. Run in QEMU:
   - `qemu-system-x86_64 -drive format=raw,file=build/os.img`
6. Verify:
   - CPU reaches Rust entry.
   - Visible output confirms execution.

## 7) Minimum technical checklist

- [ ] `no_std` + `no_main` Rust kernel compiles.
- [ ] Panic handler implemented.
- [ ] Boot asm has valid 512-byte boot sector signature (`0x55AA`) if using BIOS sector entry.
- [ ] GDT + long mode transition done before jumping to 64-bit Rust code.
- [ ] Stack initialized before calling Rust.
- [ ] Rust entry symbol exported with stable ABI (`extern "C"` + `#[unsafe(no_mangle)]` as required by edition/lints).
- [ ] Linker script aligns sections correctly.
- [ ] QEMU command scripted.

## 8) Expected risks (early)

- Triple fault from incorrect GDT/page tables/long-mode enable sequence.
- Wrong symbol names between assembly and Rust.
- Linker section placement mistakes.
- Rust code accidentally pulling in std/runtime pieces.

## 9) Definition of done (for today)

- Running one script/command produces `os.img`.
- QEMU boots `os.img` without manual steps.
- Rust kernel code is definitely executing (screen/message/halt loop).
- Build instructions are documented in README (next step after milestone).

## 10) Immediate next execution plan

1. Convert current crate into kernel crate (`no_std`, `no_main`).
2. Add assembly bootloader skeleton in `boot/boot.asm`.
3. Add linker script in `build/linker.ld`.
4. Add build/run scripts.
5. Run QEMU and iterate until Rust entry works.

---

If this structure looks good, next step is to implement section **10** directly in this repo.
