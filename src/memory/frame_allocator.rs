use super::bootinfo::MemoryMapEntry;

pub const FRAME_SIZE: u64 = 4096;
pub const MIN_ALLOCATABLE_ADDR: u64 = 0x20_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalFrame {
    pub start: u64,
}

pub struct FrameAllocator<'a> {
    regions: &'a [MemoryMapEntry],
    region_index: usize,
    next_addr: u64,
    region_end: u64,
    min_addr: u64,
}

impl<'a> FrameAllocator<'a> {
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

    fn select_next_region(&mut self) {
        while self.region_index < self.regions.len() {
            let region = self.regions[self.region_index];
            self.region_index += 1;

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
use core::sync::atomic::{AtomicBool, Ordering};
#[cfg(eres_kernel)]
use core::cell::UnsafeCell;

#[cfg(eres_kernel)]
struct FrameAllocatorCell(UnsafeCell<Option<FrameAllocator<'static>>>);
#[cfg(eres_kernel)]
unsafe impl Sync for FrameAllocatorCell {}
#[cfg(eres_kernel)]
static FRAME_ALLOCATOR: FrameAllocatorCell = FrameAllocatorCell(UnsafeCell::new(None));
#[cfg(eres_kernel)]
static FRAME_ALLOCATOR_READY: AtomicBool = AtomicBool::new(false);

#[cfg(eres_kernel)]
pub fn init_from_memory_map(entries: &'static [MemoryMapEntry]) {
    unsafe {
        *FRAME_ALLOCATOR.0.get() = Some(FrameAllocator::new(entries, MIN_ALLOCATABLE_ADDR));
    }
    FRAME_ALLOCATOR_READY.store(true, Ordering::Release);
}

#[cfg(eres_kernel)]
pub fn alloc_frame() -> Option<PhysicalFrame> {
    if !FRAME_ALLOCATOR_READY.load(Ordering::Acquire) {
        return None;
    }

    let interrupts_were_enabled = crate::arch::x86_64::save_and_disable_interrupts();
    let frame = unsafe { (*FRAME_ALLOCATOR.0.get()).as_mut().and_then(FrameAllocator::alloc) };
    crate::arch::x86_64::restore_interrupts(interrupts_were_enabled);
    frame
}

#[cfg(test)]
mod tests {
    use super::{FrameAllocator, FRAME_SIZE};
    use crate::memory::bootinfo::MemoryMapEntry;

    #[test]
    fn allocates_from_usable_region() {
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
}
