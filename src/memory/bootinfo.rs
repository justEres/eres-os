//! Datentypen und Helfer für vom Bootloader übergebene Startinformationen.
//!
//! Im Fokus steht hier die BIOS-E820-Speicherkarte:
//! <https://wiki.osdev.org/Detecting_Memory_(x86)>

use core::slice;
use core::sync::atomic::{AtomicPtr, Ordering};

/// Erkennungswert, damit der Kernel die Struktur validieren kann.
pub const BOOT_INFO_MAGIC: u32 = 0x534f5245;

#[repr(C)]
#[derive(Clone, Copy)]
/// Rohe C-kompatible Struktur am bekannten Speicherort.
pub struct BootInfoRaw {
    pub magic: u32,
    pub version: u32,
    pub memory_map_entries: u32,
    pub memory_map_entry_size: u32,
    pub memory_map_ptr: u64,
    pub reserved: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Ein E820-ähnlicher Speicherbereichseintrag.
pub struct MemoryMapEntry {
    pub base: u64,
    pub length: u64,
    pub entry_type: u32,
    pub acpi_extended_attributes: u32,
}

/// Sichere Sicht auf validierte Boot-Informationen.
pub struct BootInfoView<'a> {
    memory_map: &'a [MemoryMapEntry],
}

impl<'a> BootInfoView<'a> {
    /// Validiert eine rohe Zeigeradresse und baut daraus eine sichere Sicht.
    pub fn from_ptr(ptr: *const BootInfoRaw) -> Option<Self> {
        if ptr.is_null() {
            return None;
        }

        let raw = unsafe { &*ptr };
        if raw.magic != BOOT_INFO_MAGIC {
            return None;
        }
        if raw.version != 1 {
            return None;
        }
        if raw.memory_map_entry_size != core::mem::size_of::<MemoryMapEntry>() as u32 {
            return None;
        }

        let count = raw.memory_map_entries as usize;
        let map_ptr = raw.memory_map_ptr as *const MemoryMapEntry;
        if map_ptr.is_null() && count != 0 {
            return None;
        }

        let memory_map = unsafe { slice::from_raw_parts(map_ptr, count) };
        Some(Self { memory_map })
    }

    /// Gibt die Speicherkarte als Slice zurück.
    pub fn entries(&self) -> &'a [MemoryMapEntry] {
        self.memory_map
    }
}

static BOOT_INFO_PTR: AtomicPtr<BootInfoRaw> = AtomicPtr::new(core::ptr::null_mut());

/// Speichert den vom Bootloader gelieferten Zeiger atomar global.
pub fn set_boot_info(ptr: *const BootInfoRaw) {
    BOOT_INFO_PTR.store(ptr as *mut BootInfoRaw, Ordering::Release);
}

/// Lädt und validiert die global registrierten Boot-Informationen.
pub fn boot_info() -> Option<BootInfoView<'static>> {
    let ptr = BOOT_INFO_PTR.load(Ordering::Acquire) as *const BootInfoRaw;
    BootInfoView::from_ptr(ptr)
}

#[cfg(test)]
mod tests {
    use super::{BOOT_INFO_MAGIC, BootInfoRaw, BootInfoView, MemoryMapEntry};

    #[test]
    fn parses_valid_boot_info() {
        let entries = [MemoryMapEntry {
            base: 0x100000,
            length: 0x200000,
            entry_type: 1,
            acpi_extended_attributes: 0,
        }];

        let raw = BootInfoRaw {
            magic: BOOT_INFO_MAGIC,
            version: 1,
            memory_map_entries: entries.len() as u32,
            memory_map_entry_size: core::mem::size_of::<MemoryMapEntry>() as u32,
            memory_map_ptr: entries.as_ptr() as u64,
            reserved: 0,
        };

        let parsed = BootInfoView::from_ptr(&raw as *const _).expect("valid boot info");
        assert_eq!(parsed.entries(), &entries);
    }

    #[test]
    fn rejects_invalid_magic() {
        let raw = BootInfoRaw {
            magic: 0,
            version: 1,
            memory_map_entries: 0,
            memory_map_entry_size: core::mem::size_of::<MemoryMapEntry>() as u32,
            memory_map_ptr: 0,
            reserved: 0,
        };

        assert!(BootInfoView::from_ptr(&raw as *const _).is_none());
    }
}
