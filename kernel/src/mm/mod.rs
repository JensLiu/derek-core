use alloc::sync::Arc;
use spin::{Once, RwLock};

use crate::{allocator, info};

use self::address_space::AddrSpace;

pub mod address_space;
pub mod arithmetics;
pub mod layout;
pub mod memory;
pub mod page_table;

// ther kernel address space can be accessed by multiple cores
// and heavily read domonated.

lazy_static::lazy_static! {
    pub static ref KERNEL_ADDRESS_SPACE: Arc<RwLock<AddrSpace>> =
        Arc::new(RwLock::new(AddrSpace::make_kernel()));
}

// const KERNEL_ADDRESS_SPACE: Once<Arc<RwLock<AddrSpace>>> = Once::new();

pub fn init() {
    arithmetics::arithmetics_done_right();

    allocator::init();

    page_table::test();
    info!("page table implementation test done");
    // initialise the kernel address space
}

pub fn hart_init() {
    // KERNEL_ADDRESS_SPACE.call_once(|| {
    //     Arc::new(RwLock::new(AddrSpace::make_kernel()))
    // });

    // load the kernel page table
    // this is the first time paging is enabled
    KERNEL_ADDRESS_SPACE
        // .get()
        // .unwrap()
        .read()
        .load();
    // let kernel_space = KERNEL_ADDRESS_SPACE.get();
}
