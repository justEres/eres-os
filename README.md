# Eres OS

Small x86_64 hobby operating system written in Rust with assembly-only boot stages.

## Current milestone

- BIOS boot sector (`boot/boot.S`) loads stage2 from disk.
- Stage2 (`boot/stage2.S`) switches to protected mode, enables long mode, and jumps to Rust.
- Rust kernel (`src/lib.rs`) runs in 64-bit mode and writes a proof message.
- Runs in QEMU from a generated raw disk image.

## Prerequisites

- `rustup`, `cargo`
- `ld.lld`, `ld`, `as`, `objcopy` (or `llvm-objcopy`)
- `qemu-system-x86_64`

Rust target used:

```bash
rustup target add x86_64-unknown-none
```

## Build image

```bash
./scripts/build_image.sh
```

Expected output includes:

- `build/os.img`
- stage2 size/sectors summary

## Run in QEMU

Cargo-native run (recommended):

```bash
cargo run -- --headless
```

Or call script directly:

```bash
./scripts/run_qemu.sh
```

Default mode opens a QEMU GUI window.

For headless mode (useful for automated checks):

```bash
./scripts/run_qemu.sh --headless
```

For command-parser test mode in QEMU:

```bash
./scripts/run_qemu.sh --test
```

Debug markers are printed to QEMU debug console (`port 0xE9`):

- `B` stage1 started
- `2` stage1 loaded stage2
- `S` stage2 started
- `L` long mode entry
- `R` right before Rust call

Then Rust prints:

- `Eres OS: Rust kernel reached long mode.`
- `Eres OS: IDT/PIC initialized.`
- `Eres OS: keyboard decode OK.`

After boot, a simple shell prompt is available:

- `help`
- `echo <text>`
- `clear`
- `panic` (triggers invalid opcode exception intentionally)
- `halt`
- `reboot`

Keyboard decoding uses a German QWERTZ-oriented scancode mapping with ASCII fallbacks.

## Tests

Host unit tests for command parsing:

```bash
cargo test
```

QEMU integration command tests:

```bash
./scripts/test_qemu_commands.sh
```

## Repo layout

```text
boot/
  boot.S
  stage2.S
build/
  linker.ld
scripts/
  build_image.sh
  run_qemu.sh
src/
  lib.rs
PROJECT_STRUCTURE.md
```

## Notes

- This is intentionally minimal and educational.
- Next logical steps: basic serial logger, IDT/exceptions, and memory map handoff.
