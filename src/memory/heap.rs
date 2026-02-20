//! Sehr einfacher Bump-Allocator für frühe Kernel-Entwicklung.
//!
//! Der Allocator kann Speicher nur nach vorne vergeben und nie freigeben.
//! Das ist für frühe Boot- und Prototyp-Phasen oft ausreichend.

use core::alloc::Layout;

#[derive(Clone, Copy, Debug)]
/// Interner Zustand des linearen Allocators.
struct BumpCursor {
    start: usize,
    end: usize,
    next: usize,
}

impl BumpCursor {
    const fn new() -> Self {
        Self {
            start: 0,
            end: 0,
            next: 0,
        }
    }

    /// Setzt den verwalteten Heap-Bereich.
    fn init(&mut self, start: usize, size: usize) {
        self.start = start;
        self.end = start.saturating_add(size);
        self.next = start;
    }

    /// Allokiert einen Block mit gewünschter Größe/Ausrichtung.
    fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let aligned = align_up(self.next, layout.align());
        let next = aligned.saturating_add(layout.size());
        if next > self.end {
            core::ptr::null_mut()
        } else {
            self.next = next;
            aligned as *mut u8
        }
    }
}

const fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

#[cfg(eres_kernel)]
mod kernel_heap {
    use core::alloc::{GlobalAlloc, Layout};
    use core::cell::UnsafeCell;
    use core::sync::atomic::{AtomicBool, Ordering};

    use super::BumpCursor;

    const HEAP_SIZE: usize = 256 * 1024;

    struct LockedCursor {
        lock: AtomicBool,
        cursor: UnsafeCell<BumpCursor>,
    }

    unsafe impl Sync for LockedCursor {}

    impl LockedCursor {
        const fn new() -> Self {
            Self {
                lock: AtomicBool::new(false),
                cursor: UnsafeCell::new(BumpCursor::new()),
            }
        }

        fn with_lock<T>(&self, f: impl FnOnce(&mut BumpCursor) -> T) -> T {
            while self
                .lock
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_err()
            {}

            let result = unsafe { f(&mut *self.cursor.get()) };
            self.lock.store(false, Ordering::Release);
            result
        }
    }

    /// `GlobalAlloc`-Wrapper um den gesperrten Cursor.
    pub struct KernelAllocator {
        state: LockedCursor,
    }

    impl KernelAllocator {
        const fn new() -> Self {
            Self {
                state: LockedCursor::new(),
            }
        }
    }

    unsafe impl GlobalAlloc for KernelAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            self.state.with_lock(|cursor| cursor.alloc(layout))
        }

        unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
    }

    #[global_allocator]
    static KERNEL_ALLOCATOR: KernelAllocator = KernelAllocator::new();
    static HEAP_READY: AtomicBool = AtomicBool::new(false);
    static mut HEAP_SPACE: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

    /// Initialisiert den statischen Kernel-Heap einmalig.
    pub fn init() {
        if HEAP_READY.load(Ordering::Acquire) {
            return;
        }

        let start = core::ptr::addr_of_mut!(HEAP_SPACE) as *mut u8 as usize;
        KERNEL_ALLOCATOR
            .state
            .with_lock(|cursor| cursor.init(start, HEAP_SIZE));
        HEAP_READY.store(true, Ordering::Release);
    }
}

#[cfg(eres_kernel)]
pub use kernel_heap::init;

#[cfg(test)]
mod tests {
    use core::alloc::Layout;

    use super::BumpCursor;

    #[test]
    fn cursor_allocates_with_alignment() {
        let mut buf = [0u8; 128];
        let start = buf.as_mut_ptr() as usize;

        let mut cursor = BumpCursor::new();
        cursor.init(start, buf.len());

        let first = cursor.alloc(Layout::from_size_align(1, 1).expect("valid layout")) as usize;
        let second = cursor.alloc(Layout::from_size_align(8, 8).expect("valid layout")) as usize;
        assert_eq!(first, start);
        assert_eq!(second % 8, 0);
    }

    #[test]
    fn cursor_returns_null_when_exhausted() {
        let mut buf = [0u8; 16];
        let start = buf.as_mut_ptr() as usize;

        let mut cursor = BumpCursor::new();
        cursor.init(start, buf.len());

        let _ = cursor.alloc(Layout::from_size_align(12, 1).expect("valid layout"));
        let exhausted = cursor.alloc(Layout::from_size_align(8, 1).expect("valid layout"));
        assert!(exhausted.is_null());
    }
}
