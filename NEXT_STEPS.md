# Next Steps

This file tracks execution order for the next milestones.  
Rule: each milestone should end with passing `cargo test --workspace` and a QEMU check.

## Milestone A: Writable SimpleFS

- [ ] Extend `simplefs-core` with write-side primitives:
  - free-space discovery
  - directory entry allocation/reuse
  - file growth/shrink rules
- [ ] Add kernel-side write support in `src/fs/simplefs.rs`.
- [ ] Introduce a basic fs transaction/error model for partial write safety.
- [ ] Add unit tests for create/write/read/delete behavior on generated images.

## Milestone B: Shell File Commands

- [ ] Add commands:
  - `write <path> <text>`
  - `rm <path>`
  - `touch <path>` (or implicit create via write)
- [ ] Improve `ls` formatting (type + size).
- [ ] Add command parser tests for new syntax.

## Milestone C: Host Tool Improvements

- [ ] Add `simplefs-tool verify` to validate superblock + directory + block bounds.
- [ ] Add `simplefs-tool ls` and `simplefs-tool cat` for host-side debugging.
- [ ] Keep tool and kernel behavior aligned through shared `simplefs-core` rules.

## Milestone D: Integration and Reliability

- [ ] Add script-driven end-to-end test:
  - generate image with known files
  - boot in QEMU test mode
  - verify expected command outputs
- [ ] Add structured kernel log levels over debugcon.
- [ ] Reduce dead-code warnings by gating modules/features more precisely.

## Milestone E: Before Moving to a Richer FS

- [ ] Decide whether simplefs remains flat-root only or gets directories.
- [ ] Define on-disk compatibility policy/versioning.
- [ ] Add minimal consistency checks on mount (bounds, overlap, duplicate names).
