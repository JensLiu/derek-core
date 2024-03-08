use alloc::vec::Vec;

use lazy_static::lazy_static;
use spin::mutex::Mutex;

use crate::{
    info,
    mm::{
        arithmetics::PG_ROUND_UP,
        layout::{__heap_size, __heap_start, PAGE_SIZE},
        memory::PhysAddr,
    },
};

// Since we already have an allocator in the kernel heap space (1MB)
// we can now use dynamiclly allocate kernel data structures,
// including Rust containers!!!
pub struct FrameAllocator {
    /// records whether a page is allocated:
    ///     0: non-allocated
    ///     non-zero: allocated
    /// each entry records how many page are required
    /// for an allocation. The deallocator should know
    /// how many contiguous blocks it should free
    pub page_allocated: Vec<usize>,

    /// start of the heap
    pub base_addr: usize,
}

impl FrameAllocator {
    /// create a new allocator instalce
    /// NOTE: base_addr should be initialised later
    pub fn new(base_addr: usize, n_pages: usize) -> Self {
        Self {
            page_allocated: vec![0; n_pages],
            base_addr,
        }
    }

    fn allocate(&mut self, size: usize) -> *mut u8 {
        // we can only allocate `PAGE_SIZE` aligned
        let npages = PG_ROUND_UP(size) / PAGE_SIZE;
        for i in 0..self.page_allocated.len() {
            // find the first unallocated spot
            if self.page_allocated[i] == 0 {
                // find contiguois memory that fits
                let mut found = true;
                for j in 0..npages {
                    if !self.page_allocated[i + j] == 0 {
                        found = false;
                        break;
                    }
                }
                if found {
                    // allocate these pages by setting their entries to non-zero
                    for j in 0..npages {
                        self.page_allocated[i + j] = npages;
                    }
                    let ptr = (self.base_addr + i * size) as *mut u8;
                    // debug!("FrameAllocator::allocate: allocated page with pa: {:?}", ptr);
                    return ptr;
                }
                // if we cannot find this round, we find the next unallocated memory and try again
            }
        }
        panic!("FrameAllocator::allocate: no available page!");
    }

    /// deallocate address
    fn deallocate(&mut self, addr: *mut u8) {
        let begin_idx = (addr as usize - self.base_addr) / PAGE_SIZE;
        let npages = self.page_allocated[begin_idx];
        for id in begin_idx..begin_idx + npages {
            assert_eq!(self.page_allocated[id], npages);
            self.page_allocated[id] = 0;
        }
    }
}

lazy_static! {
    pub static ref FRAME_ALLOCATOR: Mutex<FrameAllocator> = {
        let n_pages = __heap_size() / PAGE_SIZE; // if it cannot fit inside the kernel heap, an alloc error will occur
        let allocator = FrameAllocator::new(__heap_start(), n_pages);
        Mutex::new(allocator)
    };
}
pub fn init() {
    // invoke init
    FRAME_ALLOCATOR.lock();
    info!("Frame allocator initialised");
}

// public interface
pub fn allocate_one_frame() -> PhysAddr {
    let pa = FRAME_ALLOCATOR.lock().allocate(PAGE_SIZE) as usize;
    info!(
        "frame_allocator::allocate_one_frame: allocated frame at pa {:?}",
        pa as *const usize
    );
    PhysAddr::new(pa)
}

pub fn deallocate_one_frame(pa: PhysAddr) {
    let pa = pa.as_usize();
    info!(
        "frame_allocator::deallocate_one_frame: deallocated frame at pa {:?}",
        pa as *const usize
    );
    FRAME_ALLOCATOR.lock().deallocate(pa as *mut u8);
}
