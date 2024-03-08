use riscv::register::stvec;
use virtio_drivers::PAGE_SIZE;

use crate::{mm::memory::{PhysAddr, VirtAddr}, symbols::__kernelvec};

/// Save user-space context of a process here.
/// We are swtiching altogether into its kernel thread.
/// Since we are switching to the kernel stack and
/// a trap may not necessarily be a function call,
/// we need to save all registers, be it caller or callee saved
#[repr(C)]
#[derive(Default, Clone, Debug)]
pub struct TrapContext {
    user_regs: [usize; 32], // 0-31 Byte: general purpose registers
    kernel_satp: usize, // 32 Byte: kernel page table. Supervisor Address & Translation Protection Register
    kernel_sp: usize,   // 33 Byte: kernel stack. Stack Pointer Register
    kernel_hardid: usize, // 34 Byte: kernel hartid (in tp)
    sepc: usize,        // 35 Byte: Return address from the kernel space to the user space
    trap_handler: usize, // 36 Byte: entry point of the handler in the kernel space
}

const TP: usize = 4;
const SP: usize = 2;

impl TrapContext {
    pub fn set_tp(&mut self, tp: usize) {
        self.user_regs[TP] = tp;
    }

    pub fn set_trap_handler(&mut self, addr: VirtAddr) {
        self.trap_handler = addr.as_usize();
    }

    pub fn set_user_space_execution_addr(&mut self, addr: VirtAddr) {
        self.sepc = addr.as_usize();
    }

    pub fn set_user_stack(&mut self, base_addr: PhysAddr) {
        // NOTE: since the stack grows downwards, we should convert
        // its base address to its top address
        assert!(base_addr.is_page_aligned());
        self.user_regs[SP] = base_addr.as_usize() + PAGE_SIZE;
    }

    pub fn set_kernel_stack(&mut self, base_addr: PhysAddr) {
        // NOTE: since the stack grows downwards, we should convert
        // its base address to its top address
        assert!(base_addr.is_page_aligned());
        self.kernel_sp = base_addr.as_usize() + PAGE_SIZE;
    }

    pub fn set_kernel_page_table(&mut self, satp: usize) {
        self.kernel_satp = satp;
    }

    pub fn get_kernel_page_table(&self) -> usize {
        self.kernel_satp
    }
}

/// set stvec to kernelvec
/// It will be set to uservec in user_return
pub fn trap_init_hart() {
    unsafe { stvec::write(__kernelvec as usize, stvec::TrapMode::Direct) };
}
