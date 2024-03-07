use alloc::sync::Arc;
use riscv::register::sscratch;
use spin::RwLock;

use crate::allocator;

use self::address_space::AddrSpace;

pub mod address_space;
pub mod arithmetics;
pub mod layout;
pub mod memory;
pub mod page_table;

// their kernel address space can be accessed by multiple cores
// and heavily read dominated.

lazy_static::lazy_static! {
    pub static ref KERNEL_ADDRESS_SPACE: Arc<RwLock<AddrSpace>> = {
        let kernel_space = AddrSpace::make_kernel();
        kernel_space.verify();
        Arc::new(RwLock::new(kernel_space))
    };
}

pub fn init() {
    allocator::init();
    // invoke init
    KERNEL_ADDRESS_SPACE.read();
}

pub fn hart_init() {
    // load the kernel page table: this is the first time paging is enabled
    // try setting a breakpoint here and use `info mem` in QEMU to see what happens
    KERNEL_ADDRESS_SPACE.read().load();

    // set `sscratch` to point to the TRAPFRAME in user space
    // We map each proc's TRAPFRAME to the same address, and makes sure
    // that each process sees the TRAPFRAME of its own
    sscratch::write(layout::TRAPFRAME_BASE_USER_VA);
    assert_eq!(sscratch::read(), layout::TRAPFRAME_BASE_USER_VA);
}
