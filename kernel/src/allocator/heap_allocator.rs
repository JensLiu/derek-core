//! We use buddy alocator here

use buddy_system_allocator::LockedHeap;

use crate::info;
use crate::mm::layout::{__kernel_heap_end, __kernel_heap_start, KERNEL_HEAP_SIZE};
use crate::mm::memory::PhysAddr;

// we define the KERNEL_HEAP_SIZE here, may be move to another file
// in Bytes

// the global allocator for the kernel
// Note that kernel threads share the same page table
#[global_allocator]
static KERNEL_HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

// Since it is uninitialised and staically allocated,
// It lives in the .bss section of the kernel binary
// and will be mounted to the physical memory when loaded
#[link_section = ".bss.kernel_heap"]
static mut KERNEL_HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    info!("{:?}", layout);
    panic!("HEAP_ALLOCATOR: kernel heap allocation failed\n");
}

pub fn init() {
    // safety: HEAP_START and HEAP_SIZE are calculated by the linker
    //  and are garanteed to be valid

    unsafe {
        let start = KERNEL_HEAP_SPACE.as_ptr() as usize;

        assert_eq!(start, __kernel_heap_start());
        if !PhysAddr::new(start).is_page_aligned() {
            panic!("heap_allocator::init: heap start address not page aligned!");
        }

        KERNEL_HEAP_ALLOCATOR.lock().init(start, KERNEL_HEAP_SIZE);
        info!(
            "Kernel heap space initialised: start:{:?}, size:{:?}",
            start, KERNEL_HEAP_SIZE
        );
    }
}

#[allow(unused)]
pub fn heap_test() {
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    let bss_range = __kernel_heap_start()..__kernel_heap_end() as usize;
    let a = Box::new(5);
    assert_eq!(*a, 5);
    assert!(bss_range.contains(&(a.as_ref() as *const _ as usize)));
    drop(a);
    let mut v: Vec<usize> = Vec::new();
    for i in 0..500 {
        v.push(i);
    }
    for (i, val) in v.iter().take(500).enumerate() {
        assert_eq!(*val, i);
    }
    assert!(bss_range.contains(&(v.as_ptr() as usize)));
    drop(v);
    info!("heap_test passed!");
}