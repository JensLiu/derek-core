//! We use buddy alocator here

use core::borrow::Borrow;
use core::ptr::addr_of;

use buddy_system_allocator::LockedHeap;

use crate::info;
use crate::mm::layout::{KERNEL_HEAP_SIZE};
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
        let start = addr_of!(KERNEL_HEAP_SPACE) as usize;
        
        // don't try this comparision, it should be disqualified and is invalid!!
        // assert_eq!(start, __kernel_heap_start());

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

pub fn kernel_heap_status() -> (usize, usize, usize) {
    let allocator = KERNEL_HEAP_ALLOCATOR.borrow().lock();
    let actual = allocator.stats_alloc_actual();
    let user = allocator.stats_alloc_user();
    let total = allocator.stats_total_bytes();
    (actual, user, total)
}

pub fn print_kernel_heap_status() {
    let (actual, _, total) = kernel_heap_status();
    info!(
        "---------------- KERNEL HEAP USAGE: {:?}% ---------------",
        actual * 100 / total,
    );
    info!("used: {:?} KB, total: {:?} KB", actual / 1024, total / 1024);
    info!("-------------------------------------------------------");
}
