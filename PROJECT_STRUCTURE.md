# Eres OS - Project Structure and Roadmap

## Goals

- Build a small x86_64 hobby OS in Rust.
- Keep low-level CPU/boot setup in assembly where required.
- Run and test every iteration in QEMU.
- Keep filesystem format logic shared between kernel and host tooling.

## Current Status

- BIOS stage1 bootloader (`boot/boot.S`) loads stage2 from disk.
- Stage2 (`boot/stage2.S`) gathers E820 map, enters protected mode, enables long mode, and jumps to Rust.
- Rust kernel initializes:
  - memory map handoff + frame allocator
  - heap allocator
  - IDT/PIC/PIT + keyboard input
  - shell with command parsing/history
  - ATA PIO block reads
  - read-only simplefs mount from a second disk image
- Host toolchain generates both:
  - `build/os.img` (boot disk)
  - `build/simplefs.img` (filesystem disk from `fs/root`)

## Repository Layout

```text
eres-os/
  boot/
    boot.S                 # BIOS boot sector (stage1)
    stage2.S               # mode switching + kernel handoff
  build/
    linker.ld              # kernel/stage2 link script
  crates/
    simplefs-core/         # shared on-disk format and helpers
    simplefs-tool/         # Linux CLI to build simplefs images
  docs/
    github-pages.md        # docs publishing notes
  fs/
    root/                  # input files for generated simplefs image
  scripts/
    build_image.sh         # builds kernel image (+ simplefs image)
    run_qemu.sh            # QEMU runner (GUI/headless/test)
    test_qemu_commands.sh
  src/
    arch/                  # x86_64 architecture code
    console/               # VGA + debugcon output
    fs/                    # VFS traits + simplefs mount/read
    memory/                # bootinfo, frame allocator, heap, vm helpers
    storage/               # block traits, ATA PIO, cache
    lib.rs                 # kernel entry and smoke checks
    shell.rs               # interactive REPL and commands
```

## Build and Run Flow

1. Build Rust kernel staticlib for `x86_64-unknown-none`.
2. Assemble and link stage2 with kernel.
3. Assemble stage1 with computed stage2 sector count.
4. Build `build/os.img`.
5. Build `build/simplefs.img` from `fs/root` via `simplefs-tool`.
6. Run QEMU with `os.img` as first IDE disk and `simplefs.img` as second IDE disk.

## Near-Term Direction

1. Make simplefs writable (create/remove/update file data + metadata).
2. Add shell file-manipulation commands (`write`, `rm`, `mkdir` if supported).
3. Add integration tests for file I/O roundtrips using tool-generated images.
4. Stabilize error handling/logging around storage and fs paths.
