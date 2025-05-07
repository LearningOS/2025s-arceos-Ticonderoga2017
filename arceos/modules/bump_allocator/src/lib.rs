#![no_std]

use core::{ptr::NonNull, sync::atomic::{AtomicUsize, Ordering}};

use allocator::{BaseAllocator, ByteAllocator, PageAllocator};

/// Early memory allocator
/// Use it before formal bytes-allocator and pages-allocator can work!
/// This is a double-end memory range:
/// - Alloc bytes forward
/// - Alloc pages backward
///
/// [ bytes-used | avail-area | pages-used ]
/// |            | -->    <-- |            |
/// start       b_pos        p_pos       end
///
/// For bytes area, 'count' records number of allocations.
/// When it goes down to ZERO, free bytes-used area.
/// For pages area, it will never be freed!
///
pub struct EarlyAllocator<const PAGE_SIZE: usize> {
    start: AtomicUsize,
    size: AtomicUsize,
    b_pos: AtomicUsize,
    p_pos: AtomicUsize,
    count: AtomicUsize,
}

impl<const PAGE_SIZE: usize> EarlyAllocator<PAGE_SIZE> {
    pub const fn new() -> Self {
        Self {
            start: AtomicUsize::new(0),
            size: AtomicUsize::new(0),
            b_pos: AtomicUsize::new(0),
            p_pos: AtomicUsize::new(0),
            count: AtomicUsize::new(0),
        }
    }

    pub fn is_init(&self) -> bool {
        self.size.load(Ordering::Relaxed) > 0
    }
}

impl<const PAGE_SIZE: usize> BaseAllocator for EarlyAllocator<PAGE_SIZE> {
    fn init(&mut self, start: usize, size: usize) {
        self.start.store(start, Ordering::Relaxed);
        self.size.store(size, Ordering::Relaxed);
        self.b_pos.store(start, Ordering::Relaxed);
        self.p_pos.store(start + size, Ordering::Relaxed);
        self.count.store(0, Ordering::Relaxed);
    }
    fn add_memory(&mut self, _start: usize, _size: usize) -> allocator::AllocResult {
        Err(allocator::AllocError::NoMemory)
    }
}

impl<const PAGE_SIZE: usize> ByteAllocator for EarlyAllocator<PAGE_SIZE> {
    fn alloc(&mut self, layout: core::alloc::Layout) -> allocator::AllocResult<core::ptr::NonNull<u8>> {
        let size = layout.size();
        let align = layout.align();

        let b_pos = self.b_pos.load(Ordering::Relaxed);
        let new_b_pos = (b_pos + align - 1) & !(align - 1);
        let end_b_pos = new_b_pos + size;

        if end_b_pos > self.p_pos.load(Ordering::Relaxed) {
            return Err(allocator::AllocError::NoMemory);
        }

        let ptr = new_b_pos;
        self.b_pos.store(end_b_pos, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);

        Ok(NonNull::new(ptr as *mut u8).unwrap())
    }

    fn dealloc(&mut self, _pos: core::ptr::NonNull<u8>, _layout: core::alloc::Layout) {
        if self.count.load(Ordering::Relaxed) == 0 {
            return;
        }

        let count = self.count.fetch_sub(1, Ordering::Relaxed);
        if count == 0 {
            self.b_pos.store(self.start.load(Ordering::Relaxed), Ordering::Relaxed);
        }
    }
    fn total_bytes(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }
    fn used_bytes(&self) -> usize {
        self.b_pos.load(Ordering::Relaxed) - self.start.load(Ordering::Relaxed)
    }
    fn available_bytes(&self) -> usize {
        self.p_pos.load(Ordering::Relaxed) - self.b_pos.load(Ordering::Relaxed)
    }    
}

impl<const PAGE_SIZE: usize> PageAllocator for EarlyAllocator<PAGE_SIZE> {
    const PAGE_SIZE: usize = PAGE_SIZE;

    fn alloc_pages(&mut self, num_pages: usize, align_pow2: usize) -> allocator::AllocResult<usize> {
        let align = 1 << align_pow2;
        let size = num_pages * PAGE_SIZE;

        let current = self.p_pos.load(Ordering::Relaxed);
        let aligned = (current - size) & !(align - 1);

        if aligned < self.b_pos.load(Ordering::Relaxed) || aligned > self.p_pos.load(Ordering::Relaxed) {
            return Err(allocator::AllocError::NoMemory);
        }

        self.p_pos.store(aligned, Ordering::Relaxed);
        Ok(aligned)
    }
    fn dealloc_pages(&mut self, _pos: usize, _num_pages: usize) {
        
    }
    fn total_pages(&self) -> usize {
        self.size.load(Ordering::Relaxed) / PAGE_SIZE
    }
    fn used_pages(&self) -> usize {
        let start = self.start.load(Ordering::Relaxed);
        let size = self.size.load(Ordering::Relaxed);
        let p_pos = self.p_pos.load(Ordering::Relaxed);
        (start + size - p_pos) / PAGE_SIZE
    }
    fn available_pages(&self) -> usize {
        (self.p_pos.load(Ordering::Relaxed) - self.b_pos.load(Ordering::Relaxed)) / PAGE_SIZE
    }
}
