use riscv::register::stvec;

use crate::symbols::__kernelvec;

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

    pub fn set_trap_handler(&mut self, addr: usize) {
        self.trap_handler = addr;
    }

    pub fn set_user_space_execution_addr(&mut self, addr: usize) {
        self.sepc = addr;
    }

    pub fn set_user_stack(&mut self, addr: usize) {
        self.user_regs[SP] = addr;
    }

    pub fn set_kernel_stack(&mut self, addr: usize) {
        self.kernel_sp = addr;
    }

    pub fn set_kernel_page_table(&mut self, satp: usize) {
        self.kernel_satp = satp;
    }
}

/// set stvec to kernelvec
/// It will be set to uservec in user_return
pub fn trap_init_hart() {
    unsafe { stvec::write(__kernelvec as usize, stvec::TrapMode::Direct) };
}
