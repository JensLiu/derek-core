use alloc::sync::Arc;
use lazy_static::lazy_static;
use spin::{rwlock::RwLock, Mutex};

use crate::mm::layout::TRAPFRAME_BASE_USER_VA;
use crate::mm::memory::{PhysAddr, VirtAddr};
use crate::mm::KERNEL_ADDRESS_SPACE;
use crate::trap::usertrap;
use crate::{
    allocator::identifier_allocator::IdentifierAllocator,
    debug,
    mm::{
        address_space::AddrSpace,
        layout::TEXT_BASE_USER_VA,
        memory::{Frame, FrameGuard},
    },
    symbols::N_PROCS,
};

use super::context::TrapContext;

#[derive(Debug)]
pub enum ProcStatus {
    RUNNING,
    RUNNABLE,
    ZOMBIE,
}

#[repr(C)]
#[derive(Debug)]
pub struct ProcessControlBlock {
    pid: PIDGuard,
    // the kernel stack is not visible to its user address space, hence it is not managed by the `user_addr_space`
    // Dropping it results in the frame for its kernel stack being recycled
    pub kernel_stack: KernelStackGuard,
    pub inner: RwLock<PCBInner>,
}

#[repr(C)]
#[derive(Debug)]
/// Safety: Every `PCBInner` should be wrapped within a lock!!
pub struct PCBInner {
    // Note on trap_context:
    // If it were stored inside the PCBInner structure, its address may not be page aligned.
    // Plus, it may leak kernel data immediately adjacent to the trap_context to the user space!
    //
    // Here, we chose to allocate a whole page and store its address.
    // when the PCB is allocated, it's set to point to None
    trap_context: Option<PhysAddr>,

    // Dropping it reuslts in recycling of frames for page table and all user space
    // this includes the page containing `trap_context`
    user_addr_space: Option<AddrSpace>,

    pub status: ProcStatus,
}

impl PCBInner {
    pub fn initialise_trap_context(&mut self, f: impl FnOnce() -> PhysAddr) {
        let ctx = f();
        self.trap_context = Some(ctx);
    }

    pub fn modify_trap_context<T>(&mut self, f: impl FnOnce(&mut TrapContext) -> T) -> T {
        let ptr = self
            .trap_context
            .expect("PCBInner::modify_trap_context: uninitialised trap context");
        let ctx_ref_mut = unsafe {
            // safety: it is never exposed, and initialised to a valid place
            core::mem::transmute(ptr)
        };
        f(ctx_ref_mut)
    }

    pub fn modify_user_space<T>(&mut self, f: impl FnOnce(&mut AddrSpace) -> T) -> T {
        let user_space_ref_mut = self
            .user_addr_space
            .as_mut()
            .expect("PCBInner::modify_user_space: unitialised user address space");
        f(user_space_ref_mut)
    }

    pub fn get_user_space_ref_or_else_panic(&self) -> &AddrSpace {
        match &self.user_addr_space {
            Some(space_ref) => space_ref,
            None => {
                panic!("PCBInner::get_trap_context_mut_or_else_panic: uninitialised user address space");
            }
        }
    }

    pub fn get_context_ref_or_else_panic(&self) -> &TrapContext {
        let ptr = self
            .trap_context
            .expect("PCBInner::modify_trap_context: uninitialised trap context");
        unsafe {
            // safety: it is never exposed, and initialised to a valid place
            core::mem::transmute(ptr)
        }
    }

    /// Don't forget to call it!!!!
    /// It allocates page for the trapframe and set its content
    pub fn first_execution_init(&mut self, kernel_stack_pa: PhysAddr) {
        // allocate the trapframe as a whole page

        // we now allocate the trapframe here
        let trapframe_pa = self.modify_user_space(|space| space.init_trapframe());

        self.initialise_trap_context(|| {
            // Safety: since it is guarenteed to be allocated by the frame allocator
            //  and managed by the user space. It should be a valid physical address.
            //  Since the kernel has an identical mapping to the physical address space,
            //  it should be valid and safe to case
            trapframe_pa
        });

        // immutablly borrow inner, since we also mutabily borrowed inner
        // at `inner.trap_context = unsafe {...}`
        let uesr_space = self.get_user_space_ref_or_else_panic();
        // verify the address space (see if its content matches its page table). expensive
        uesr_space.verify();

        // test if the trapframe is mapped correctly
        {
            let (translated_trapframe_pa, _) = uesr_space
                .translate(VirtAddr::new(TRAPFRAME_BASE_USER_VA))
                .unwrap();
            assert_eq!(
                trapframe_pa.as_usize() as *const usize,
                translated_trapframe_pa.as_usize() as *const usize
            );
        }

        // initialise its execution context since it now knows the position of its kernel stack
        self.modify_trap_context(|ctx| {
            ctx.set_kernel_stack(kernel_stack_pa);
            // trap handler function: can use its physical address since it is only called
            // in the kernel address space
            ctx.set_trap_handler(VirtAddr::new(usertrap as usize));
            ctx.set_user_space_execution_addr(VirtAddr::new(TEXT_BASE_USER_VA)); // pc on sret

            // set kernel page table address
            // uservec reads this value and switches page table
            let satp = KERNEL_ADDRESS_SPACE.read().make_satp();
            ctx.set_kernel_page_table(satp)
        });
        // we do not set `tp` because we do not know on which core it will be scheduled
    }
}

impl ProcessControlBlock {
    pub fn allocate() -> Self {
        let zelf = Self {
            pid: PIDGuard::allocate(),
            kernel_stack: KernelStackGuard::allocate(),
            inner: RwLock::new(PCBInner {
                trap_context: None,
                user_addr_space: None,
                status: ProcStatus::RUNNABLE,
            }),
        };
        debug!(
            "ProcessControlBlock::allocate: PCB for PID {:?} allocated",
            zelf.pid.as_usize()
        );

        zelf
    }

    pub fn get_pid(&self) -> usize {
        self.pid.as_usize()
    }

    pub fn get_kernel_stack_phys_addr(&self) -> PhysAddr {
        self.kernel_stack.frame().get_base_phys_addr()
    }

    pub fn first_execution_init(&mut self) {
        self.inner
            .write()
            .first_execution_init(self.get_kernel_stack_phys_addr());
    }
}

impl Drop for ProcessControlBlock {
    fn drop(&mut self) {
        debug!(
            "ProcessControlBlock::drop: PCB for PID {:?} deallocated",
            self.pid.as_usize()
        );
    }
}

// Kernel stack for a process
#[derive(Debug)]
pub struct KernelStackGuard {
    inner: FrameGuard,
}

impl KernelStackGuard {
    pub fn allocate() -> Self {
        let zelf = Self {
            inner: FrameGuard::allocate_zeroed(),
        };
        let pa = zelf.inner.get_frame().get_base_phys_addr().as_usize();
        debug!(
            "KernelStackGuard::allocate: kernel stack at pa {:?} allocated",
            pa as *const usize
        );
        zelf
    }

    pub fn from_frame(frame: Frame) -> Self {
        Self {
            inner: FrameGuard::from_frame(frame),
        }
    }

    pub fn frame(&self) -> Frame {
        self.inner.get_frame()
    }
}

impl Drop for KernelStackGuard {
    fn drop(&mut self) {
        let pa = self.inner.get_frame().get_base_phys_addr().as_usize();
        debug!(
            "KernelStackGuard::drop: kernel stack at pa {:?} deallocated",
            pa as *const usize
        );
    }
}

// PID allocator
lazy_static! {
    static ref PID_ALLOCATOR: Mutex<IdentifierAllocator> =
        Mutex::new(IdentifierAllocator::new(N_PROCS));
}

#[repr(transparent)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct PIDGuard {
    id: usize,
}

impl PIDGuard {
    pub fn allocate() -> Self {
        let zelf = Self {
            id: PID_ALLOCATOR.lock().allocate(),
        };
        debug!("PIDGuard::allocate: PID {:?} allocated", zelf.id);
        zelf
    }

    pub fn as_usize(&self) -> usize {
        self.id
    }
}

impl PartialEq<usize> for PIDGuard {
    fn eq(&self, other: &usize) -> bool {
        self.id == *other
    }
}

impl PartialOrd<usize> for PIDGuard {
    fn partial_cmp(&self, other: &usize) -> Option<core::cmp::Ordering> {
        Some(self.id.cmp(other))
    }
}

impl Drop for PIDGuard {
    fn drop(&mut self) {
        PID_ALLOCATOR.lock().deallocate(self.id);
        debug!("PIDGuard::drop: PID {:?} dropped", self.id);
    }
}

/// It creates PCB for the first user-space process `init`
pub fn make_init() -> Arc<ProcessControlBlock> {
    let pcb = ProcessControlBlock::allocate();
    let mut inner = pcb.inner.write();

    inner.user_addr_space = Some(AddrSpace::make_init());

    // set its context
    inner.first_execution_init(pcb.get_kernel_stack_phys_addr());
    // specifically drop inner, otherwise the compiler will assume we may
    // mutabily change its content in the destructor after it's been moved to Arc::new(pcb)
    drop(inner);

    assert_eq!(pcb.pid.as_usize(), 0);
    Arc::new(pcb)
}

/// the first user-space process but compiled into the kernel
pub fn init_code_bytes() -> &'static [u8] {
    // compiler builtin macro
    let data: &'static [u8] = include_bytes!("../../../target/riscv64gc-unknown-none-elf/debug/initcode");
    info!("process::init_code_bytes: init code bytes are located at pa: {:?}", addr_of!(data));
    data
}
