use core::marker::PhantomData;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysAddr(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtAddr(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageSize4K;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageSize2M;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Page<S> {
    pub base: VirtAddr,
    _size: PhantomData<S>,
}

impl<S> Page<S> {
    pub const fn new(base: VirtAddr) -> Self {
        Self {
            base,
            _size: PhantomData,
        }
    }
}

pub const fn align_down(addr: u64, align: u64) -> u64 {
    addr & !(align - 1)
}

pub const fn align_up(addr: u64, align: u64) -> u64 {
    (addr + align - 1) & !(align - 1)
}

pub const FLAG_PRESENT: u64 = 1 << 0;
pub const FLAG_WRITABLE: u64 = 1 << 1;
pub const FLAG_USER: u64 = 1 << 2;
pub const FLAG_PAGE_SIZE: u64 = 1 << 7;
pub const FLAG_NO_EXEC: u64 = 1 << 63;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageTableEntry(pub u64);

impl PageTableEntry {
    pub const fn empty() -> Self {
        Self(0)
    }

    pub fn is_present(self) -> bool {
        (self.0 & FLAG_PRESENT) != 0
    }

    pub fn addr(self) -> PhysAddr {
        PhysAddr(self.0 & 0x000f_ffff_ffff_f000)
    }

    pub fn flags(self) -> u64 {
        self.0 & !0x000f_ffff_ffff_f000
    }

    pub fn set(&mut self, phys: PhysAddr, flags: u64) {
        let addr = align_down(phys.0, 4096);
        self.0 = addr | flags;
    }
}

pub trait Mapper2M {
    fn map_2m_identity(&mut self, index: usize, phys: PhysAddr, writable: bool);
    fn entry(&self, index: usize) -> PageTableEntry;
}

pub struct BootPageDirectoryMapper<'a> {
    entries: &'a mut [u64; 512],
}

impl<'a> BootPageDirectoryMapper<'a> {
    pub fn new(entries: &'a mut [u64; 512]) -> Self {
        Self { entries }
    }
}

impl Mapper2M for BootPageDirectoryMapper<'_> {
    fn map_2m_identity(&mut self, index: usize, phys: PhysAddr, writable: bool) {
        let mut flags = FLAG_PRESENT | FLAG_PAGE_SIZE;
        if writable {
            flags |= FLAG_WRITABLE;
        }
        let addr = align_down(phys.0, 2 * 1024 * 1024);
        self.entries[index] = addr | flags;
    }

    fn entry(&self, index: usize) -> PageTableEntry {
        PageTableEntry(self.entries[index])
    }
}

#[cfg(eres_kernel)]
unsafe extern "C" {
    static mut pd_table: [u64; 512];
}

#[cfg(eres_kernel)]
pub fn boot_mapper() -> BootPageDirectoryMapper<'static> {
    let table = unsafe { &mut *(&raw mut pd_table) };
    BootPageDirectoryMapper::new(table)
}

#[cfg(test)]
mod tests {
    use super::{
        align_down, align_up, BootPageDirectoryMapper, Mapper2M, PageTableEntry, PhysAddr,
        FLAG_PAGE_SIZE, FLAG_PRESENT, FLAG_WRITABLE,
    };

    #[test]
    fn aligns_addresses() {
        assert_eq!(align_down(0x12345, 0x1000), 0x12000);
        assert_eq!(align_up(0x12345, 0x1000), 0x13000);
    }

    #[test]
    fn page_table_entry_roundtrip() {
        let mut entry = PageTableEntry::empty();
        entry.set(PhysAddr(0x12345_6789), FLAG_PRESENT | FLAG_WRITABLE);
        assert!(entry.is_present());
        assert_eq!(entry.addr().0, 0x12345_6000);
        assert_eq!(entry.flags() & FLAG_WRITABLE, FLAG_WRITABLE);
    }

    #[test]
    fn maps_2m_entries() {
        let mut table = [0_u64; 512];
        let mut mapper = BootPageDirectoryMapper::new(&mut table);
        mapper.map_2m_identity(1, PhysAddr(0x2345_6789), true);
        let entry = mapper.entry(1);
        assert!(entry.is_present());
        assert_eq!(entry.addr().0, 0x2340_0000);
        assert_eq!(entry.flags() & FLAG_PAGE_SIZE, FLAG_PAGE_SIZE);
        assert_eq!(entry.flags() & FLAG_WRITABLE, FLAG_WRITABLE);
    }
}
