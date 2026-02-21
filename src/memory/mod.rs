//! Speicherbezogene Kernel-Komponenten.

/// Validierung und Zugriff auf Boot-Informationen (u. a. E820-Map).
pub mod bootinfo;
/// Einfacher physischer Frame-Allocator.
pub mod frame_allocator;
/// Kleiner Heap-Allocator für dynamische Rust-Datenstrukturen.
pub mod heap;
pub mod vm;
