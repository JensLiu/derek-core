// timer interrupt should be enabled in machine mode
// hence not in this module, see `src/clint.rs` for its initialisation

pub mod page_fault;
pub mod syscall;

use riscv::register::{
    satp,
    scause::{self, Trap},
    sscratch, sstatus, stval, stvec,
};

use crate::{
    arch, cpu, info,
    mm::{layout::TRAMPOLINE_BASE_VA, KERNEL_ADDRESS_SPACE},
    symbols::{__kernelvec, __userret, __uservec},
};
use crate::{
    mm::{layout::TRAPFRAME_BASE_USER_VA, memory::VirtAddr},
    trap::{page_fault::InstructionPageFaultHandler, syscall::SystemCallHandler},
};

// dispatchers

#[no_mangle]
pub fn kerneltrap() {
    let hartid = arch::hart_id();
    match scause::read().cause() {
        Trap::Interrupt(intr) => {
            match intr {
                scause::Interrupt::SupervisorSoft => {
                    // info!("hart-{:?} kerneltrap: S-mode software", hartid);
                }
                scause::Interrupt::SupervisorTimer => {
                    info!("hart-{:?} kerneltrap: S-mode timer", hartid);
                    panic!("We use CLINT to provide software interrupt for timer! What's this???")
                }
                scause::Interrupt::SupervisorExternal => {
                    info!("hart-{:?} kerneltrap: S-mode external", hartid);
                }
                scause::Interrupt::Unknown => {
                    panic!("hart-{:?} kerneltrap: Unknown S-mode interrupt", hartid);
                }
            }
        }
        Trap::Exception(ex) => {
            panic!("trap::kerneltrap: unexpected exception: {:?}", ex);
        }
    }
}

#[no_mangle]
pub fn usertrap() {
    arch::intr_off();
    unsafe {
        // change interrupt handler to the kernel trap handler
        stvec::write(__kernelvec as usize, stvec::TrapMode::Direct)
    };

    // let pcb = cpu::current_process().unwrap();
    // let pid = pcb.get_pid();
    // let hartid = arch::hart_id();

    // info!("trap::usertrap: core: {:?} PID: {:?}", hartid, pid);

    match scause::read().cause() {
        Trap::Interrupt(intr) => match intr {
            scause::Interrupt::SupervisorSoft => {
                // TODO: schedule
                info!("Supervisor Software Interrupt");
                panic!();
            }
            _ => {
                panic!("Unsupported exception: {:?}", intr);
            }
        },

        Trap::Exception(ex) => match ex {
            scause::Exception::UserEnvCall => {
                SystemCallHandler::handle();
            }
            scause::Exception::InstructionPageFault => {
                InstructionPageFaultHandler::handle();
            }
            scause::Exception::LoadPageFault => {
                panic!(
                    "trap::usertrap: Load Page Fault: trying to load {:?}",
                    stval::read() as *const usize
                );
            }
            _ => {
                panic!(
                    "Unsupported exception: {:?}, stval: {:?}",
                    ex,
                    stval::read() as *const usize
                );
            }
        },
    }

    usertrapret();
}

/// return from the kernel thread
/// this call does not return and anything used before will not be dealllocated
/// drop them manually or use a scope!!!
pub fn usertrapret() -> ! {
    // We need to set stvec to uservec in the trampoline!
    unsafe {
        // Note that we use TRAMPOLINE_BASE_VA here to denote the universal mapping of
        // the trampoline page accross user and kernel space.
        // Do not use __usertrap since it is the actual physical location of the trampoline
        // and user address space does not contain it!

        stvec::write(TRAMPOLINE_BASE_VA, stvec::TrapMode::Direct);

        // make sure that sscratch holds the value of the trapframe
        assert_eq!(sscratch::read(), TRAPFRAME_BASE_USER_VA);

        // set Supervisor Previous Privilege bit to user mode
        // otherwise we would still be in the supervisor mode!
        sstatus::set_spp(sstatus::SPP::User);

        // also enable interrupts in the user mode by
        // enabling Supervisor Previous Interrupt Enabled bit
        // otherwise we would be able not to preemptively trap into the kernel mode
        sstatus::set_spie();
    }

    let satp = {
        // note that it's scoped to prevent holding on to resource
        let hartid = arch::hart_id();
        let pcb = cpu::current_process().expect("trap::userret: No runable process");
        // we set its `tp` to the current hartid
        let mut inner = pcb.inner.write();
        inner.write_trap_context(|ctx| ctx.set_tp(hartid));
        let inner = inner.downgrade();
        // let context = inner.get_context_ref_or_else_panic();

        // debug!(
        //     "trap::trapret: core: {:?}, PID: {:?}",
        //     hartid,
        //     pcb.get_pid(),
        // );

        // we test if the kernel's satp is the same as the context's kernel_satp
        assert_eq!(
            satp::read().bits(),
            inner
                .get_context_ref_or_else_panic()
                .get_kernel_page_table()
        );

        inner.get_user_space_ref_or_else_panic().make_satp()
    };

    userret_on_trampoline(satp);
}

#[inline]
fn userret_on_trampoline(satp: usize) -> ! {
    // we now turn the kernel interrupt off!!!
    arch::intr_off();

    // Check for `__uservec` and `__userret` in `src/asm/trampoline.S`
    let addr = {
        let uservec_pa = __uservec as usize;
        let userret_pa = __userret as usize;
        let offset = userret_pa - uservec_pa;
        let addr = TRAMPOLINE_BASE_VA + offset;

        // tests
        {
            // make sure that we can execute `userret` in user space
            let pcb = cpu::current_process().unwrap();
            let inner = pcb.inner.read();
            let (userret_user_translated_pa, _) = inner
                .get_user_space_ref_or_else_panic()
                .translate(VirtAddr::new(addr))
                .unwrap();
            let (userret_kernel_translated_pa, _) = KERNEL_ADDRESS_SPACE
                .read()
                .translate(VirtAddr::new(addr))
                .unwrap();
            assert_eq!(userret_user_translated_pa, userret_kernel_translated_pa);

            // make sure that user space can execute `uservec`
            let (trampoline_user_translated_pa, _) = inner
                .get_user_space_ref_or_else_panic()
                .translate(VirtAddr::new(TRAMPOLINE_BASE_VA))
                .unwrap();
            assert_eq!(
                trampoline_user_translated_pa.as_usize() as *const usize,
                __uservec as *const usize
            );
        }

        addr
    };

    let userret_virtual: extern "C" fn(usize) -> ! = unsafe { core::mem::transmute(addr) };

    // NOTE: we cannot directly call __userret(satp), here's the reason
    //  - the __userret is a linker symbol represents the physical position of the __userret function
    //  - the user-space does not have an idential mapping to the physical memory
    //  - the execution will fail
    // SO WE NEED TO USE THE UNIVERSALLY MAPPED SECTION ON THE TRAMPOLINE PAGE!
    userret_virtual(satp);
    // NOTE: when debugging, make sure to remove old breakpoints in the kernel space!
    // otherwise after the page table switch and memory fence, the debugger would not
    // be able to insert breakpoints in the kernel space!!!!
}

pub fn init_hart() {
    unsafe { stvec::write(__kernelvec as usize, stvec::TrapMode::Direct) };
}
