# Next Steps

## Milestone 1: Interrupt Foundation (Current)

- [x] Add basic x86_64 I/O port helpers (`inb`, `outb`, `io_wait`).
- [x] Add PIC remap and IRQ mask control.
- [x] Add IDT structures and `lidt` loader.
- [x] Add assembly interrupt stubs for core exceptions and IRQ1.
- [x] Add Rust interrupt dispatcher with visible fault messages.
- [x] Enable interrupts after initialization in `kernel_main`.
- [x] Verify boot still reaches Rust and does not triple-fault.

## Milestone 2: Keyboard Input

- [x] Handle IRQ1 keyboard scancodes from port `0x60`.
- [x] Add simple scancode-set-1 decoder (US layout subset first).
- [x] Track shift state.
- [x] Push decoded keys into a static ring buffer.
- [x] Add basic key polling API for console.

## Milestone 3: Console + REPL

- [x] Add VGA cursor + line editing helpers.
- [x] Implement blocking `read_line`.
- [x] Implement command parser (space-delimited).
- [x] Add commands: `help`, `echo`, `clear`, `panic`, `halt`, `reboot`.
- [x] Keep shell loop in kernel main thread.

## Milestone 4: Memory Foundations

- [ ] Define memory map handoff format from boot stages.
- [ ] Implement physical frame allocator.
- [ ] Add kernel heap allocator and `alloc` crate integration.
- [ ] Move dynamic buffers/strings in shell to heap-backed forms.

## Milestone 5: Quality and Debugging

- [ ] Add structured logging levels over debugcon.
- [ ] Add reusable test boot profile (`--headless` smoke checks).
- [ ] Document architecture and boot flow updates in README.
