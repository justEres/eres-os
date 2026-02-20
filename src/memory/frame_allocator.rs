//! Sehr einfacher physischer Frame-Allocator.
//!
//! Er durchläuft die vom Bootloader gemeldeten nutzbaren Speicherbereiche (`entry_type == 1`)
//! und vergibt 4-KiB-Rahmen sequenziell. Das reicht für frühe Kernel-Phasen.

use super::bootinfo::MemoryMapEntry;

/// Größe eines physischen Frames (4 KiB).
pub const FRAME_SIZE: u64 = 4096;
/// Untergrenze für allokierbaren Speicher (2 MiB, um niedrige Bereiche zu meiden).
pub const MIN_ALLOCATABLE_ADDR: u64 = 0x20_0000;

/// Ein physischer Speicherrahmen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalFrame {
    /// Startadresse des Frames.
    pub start: u64,
}

/// Laufzeitstatistik des globalen Allocators.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameStats {
    pub total_frames: u64,
    pub allocated_frames: u64,
    pub free_frames: u64,
}

/// Sequentieller Iterator über allokierbare Speicherrahmen.
pub struct FrameAllocator<'a> {
    regions: &'a [MemoryMapEntry],
    region_index: usize,
    next_addr: u64,
    region_end: u64,
    min_addr: u64,
}

impl<'a> FrameAllocator<'a> {
    /// Erzeugt einen neuen Allocator über gegebene Speicherregionen.
    pub fn new(regions: &'a [MemoryMapEntry], min_addr: u64) -> Self {
        let mut allocator = Self {
            regions,
            region_index: 0,
            next_addr: 0,
            region_end: 0,
            min_addr,
        };
        allocator.select_next_region();
        allocator
    }

    /// Reserviert den nächsten verfügbaren Frame.
    pub fn alloc(&mut self) -> Option<PhysicalFrame> {
        loop {
            if self.next_addr >= self.region_end {
                self.select_next_region();
                if self.next_addr >= self.region_end {
                    return None;
                }
            }

            let frame_start = align_up(self.next_addr, FRAME_SIZE);
            if frame_start + FRAME_SIZE > self.region_end {
                self.next_addr = self.region_end;
                continue;
            }

            self.next_addr = frame_start + FRAME_SIZE;
            return Some(PhysicalFrame { start: frame_start });
        }
    }

    /// Springt zur nächsten nutzbaren Speicherregion.
    fn select_next_region(&mut self) {
        while self.region_index < self.regions.len() {
            let region = self.regions[self.region_index];
            self.region_index += 1;

            // E820: Typ 1 steht für "usable RAM".
            if region.entry_type != 1 || region.length == 0 {
                continue;
            }

            let start = region.base.max(self.min_addr);
            let end = region.base.saturating_add(region.length);
            if start >= end {
                continue;
            }

            self.next_addr = start;
            self.region_end = end;
            return;
        }

        self.next_addr = 0;
        self.region_end = 0;
    }
}

const fn align_up(value: u64, align: u64) -> u64 {
    let mask = align - 1;
    (value + mask) & !mask
}

#[cfg(eres_kernel)]
use core::cell::UnsafeCell;
#[cfg(eres_kernel)]
use core::sync::atomic::{AtomicBool, Ordering};

#[cfg(eres_kernel)]
struct FrameAllocatorCell(UnsafeCell<Option<FrameAllocator<'static>>>);
#[cfg(eres_kernel)]
unsafe impl Sync for FrameAllocatorCell {}
#[cfg(eres_kernel)]
static FRAME_ALLOCATOR: FrameAllocatorCell = FrameAllocatorCell(UnsafeCell::new(None));
#[cfg(eres_kernel)]
static FRAME_ALLOCATOR_READY: AtomicBool = AtomicBool::new(false);
#[cfg(eres_kernel)]
static TOTAL_FRAMES: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
#[cfg(eres_kernel)]
static ALLOCATED_FRAMES: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

/// Initialisiert den globalen Frame-Allocator aus der Boot-Speicherkarte.
#[cfg(eres_kernel)]
pub fn init_from_memory_map(entries: &'static [MemoryMapEntry]) {
    let total = count_usable_frames(entries, MIN_ALLOCATABLE_ADDR);

    unsafe {
        *FRAME_ALLOCATOR.0.get() = Some(FrameAllocator::new(entries, MIN_ALLOCATABLE_ADDR));
    }

    TOTAL_FRAMES.store(total, Ordering::Release);
    ALLOCATED_FRAMES.store(0, Ordering::Release);
    FRAME_ALLOCATOR_READY.store(true, Ordering::Release);
}

/// Allokiert einen einzelnen physischen Frame aus dem globalen Allocator.
#[cfg(eres_kernel)]
pub fn alloc_frame() -> Option<PhysicalFrame> {
    if !FRAME_ALLOCATOR_READY.load(Ordering::Acquire) {
        return None;
    }

    let frame = unsafe {
        let allocator = &mut *FRAME_ALLOCATOR.0.get();
        allocator.as_mut().and_then(FrameAllocator::alloc)
    };

    if frame.is_some() {
        ALLOCATED_FRAMES.fetch_add(1, Ordering::AcqRel);
    }

    frame
}

/// Gibt statistische Informationen über den globalen Allocator zurück.
#[cfg(eres_kernel)]
pub fn stats() -> Option<FrameStats> {
    if !FRAME_ALLOCATOR_READY.load(Ordering::Acquire) {
        return None;
    }

    let total = TOTAL_FRAMES.load(Ordering::Acquire);
    let allocated = ALLOCATED_FRAMES.load(Ordering::Acquire);
    Some(FrameStats {
        total_frames: total,
        allocated_frames: allocated,
        free_frames: total.saturating_sub(allocated),
    })
}

/// Test-/Host-Build-Fallback ohne globalen Allocatorzustand.
#[cfg(not(eres_kernel))]
pub fn stats() -> Option<FrameStats> {
    None
}

#[cfg(not(eres_kernel))]
pub fn alloc_frame() -> Option<PhysicalFrame> {
    None
}

fn count_usable_frames(entries: &[MemoryMapEntry], min_addr: u64) -> u64 {
    let mut count = 0;

    for region in entries {
        if region.entry_type != 1 || region.length == 0 {
            continue;
        }

        let start = align_up(region.base.max(min_addr), FRAME_SIZE);
        let end = align_up(region.base.saturating_add(region.length), FRAME_SIZE);
        if start < end {
            count += (end - start) / FRAME_SIZE;
        }
    }

    count
}

#[cfg(test)]
mod tests {
    use super::{FRAME_SIZE, FrameAllocator};
    use crate::memory::bootinfo::MemoryMapEntry;

    #[test]
    fn allocates_consecutive_frames() {
        let regions = [MemoryMapEntry {
            base: 0x200000,
            length: FRAME_SIZE * 3,
            entry_type: 1,
            acpi_extended_attributes: 0,
        }];

        let mut alloc = FrameAllocator::new(&regions, 0x200000);
        assert_eq!(alloc.alloc().map(|f| f.start), Some(0x200000));
        assert_eq!(alloc.alloc().map(|f| f.start), Some(0x201000));
        assert_eq!(alloc.alloc().map(|f| f.start), Some(0x202000));
        assert_eq!(alloc.alloc().map(|f| f.start), None);
    }

    #[test]
    fn skips_reserved_regions() {
        let regions = [
            MemoryMapEntry {
                base: 0x200000,
                length: FRAME_SIZE,
                entry_type: 2,
                acpi_extended_attributes: 0,
            },
            MemoryMapEntry {
                base: 0x300000,
                length: FRAME_SIZE,
                entry_type: 1,
                acpi_extended_attributes: 0,
            },
        ];

        let mut alloc = FrameAllocator::new(&regions, 0x200000);
        assert_eq!(alloc.alloc().map(|f| f.start), Some(0x300000));
        assert_eq!(alloc.alloc().map(|f| f.start), None);
    }

    #[test]
    fn honors_min_allocatable_address() {
        let regions = [MemoryMapEntry {
            base: 0x100000,
            length: FRAME_SIZE * 4,
            entry_type: 1,
            acpi_extended_attributes: 0,
        }];

        let mut alloc = FrameAllocator::new(&regions, 0x102000);
        assert_eq!(alloc.alloc().map(|f| f.start), Some(0x102000));
        assert_eq!(alloc.alloc().map(|f| f.start), Some(0x103000));
        assert_eq!(alloc.alloc().map(|f| f.start), None);
    }

    #[test]
    fn counts_total_usable_frames() {
        let regions = [
            MemoryMapEntry {
                base: 0x100000,
                length: FRAME_SIZE * 2,
                entry_type: 1,
                acpi_extended_attributes: 0,
            },
            MemoryMapEntry {
                base: 0x200000,
                length: FRAME_SIZE * 3,
                entry_type: 1,
                acpi_extended_attributes: 0,
            },
        ];

        let total = super::count_usable_frames(&regions, 0x200000);
        assert_eq!(total, 3);
    }
}
