use alloc::{collections::BTreeMap, string::String, vec::Vec};
use riscv::{asm::sfence_vma_all, register::satp};
use virtio_drivers::PAGE_SIZE;

use crate::{
    debug, info,
    mm::{
        layout::{
            __bss_end, __bss_start, __data_end, __data_start, __heap_end, __heap_start,
            __kernel_stack_end, __kernel_stack_start, __rodata_end, __rodata_start, __text_end,
            __text_start, __trampoline_start, MAX_VA, TRAMPOLINE_BASE_VA, TRAPFRAME_BASE_USER_VA,
            TRAPFRAME_SIZE,
        },
        memory::FrameGuard,
    },
    process::process::init_code_bytes,
};

use super::{
    layout::{
        CLINT_BASE, CLINT_SIZE, PLIC_BASE, PLIC_SIZE, TEXT_BASE_USER_VA, UART_BASE, UART_SIZE,
        VIRTIO_BASE, VIRTIO_SIZE,
    },
    memory::{Frame, FrameRange, PhysAddr, VirtAddr, VirtFrameGuard, VirtFrameRange},
    page_table::{PageFlags, PageTableGuard},
};

// ------------------------- Address Space -------------------------------------
// an abstraction of a logical address space it owns
// (1) a `PageTable` with its `node_frames`
// (2) Its`VirtArea`s with their underlying `data_frames`
#[derive(Debug)]
pub struct AddrSpace {
    page_table: PageTableGuard,
    virt_areas: Vec<VirtArea>, // TODO: refactor into sections???
}

impl AddrSpace {
    /// load address space directly
    /// execution may be corrupted if not careful!
    pub fn load(&self) {
        // let ptr = self.page_table.make_satp();
        unsafe {
            // satp::write(ptr);
            satp::set(satp::Mode::Sv39, 0, self.page_table.get_root_frame().number);
            // asm!("sfence.vma"); // memory fence to flush TLB
            sfence_vma_all();
        };
        // these are pretty much a wrapper function to the underlying RISC-V instructions
    }

    /// It is recommended to verify before loading the page table
    pub fn verify(&self) {
        for virt_area in self.virt_areas.iter().rev() {
            self.page_table.verify_virt_area_mapping(virt_area);
        }
    }

    pub fn make_satp(&self) -> usize {
        self.page_table.make_satp()
    }

    pub fn translate(&self, va: VirtAddr) -> Option<(PhysAddr, PageFlags)> {
        let (pa, pte_flags) = self.page_table.translate(va)?;
        Some((pa, pte_flags.into()))
    }

    /// lock the space by making the node frames of its page table in the kernel space read-only
    pub fn lock_space(&mut self) {
        let another_space = Self::make_kernel();
        another_space.load();
        self.page_table.lock_table();
        self.load();
        drop(another_space);
    }

    /// lock the space by making the node frames of its page table in the kernel space writable
    pub fn unlock_space(&mut self) {
        let another_space = Self::make_kernel();
        another_space.load();
        self.page_table.unlock_table();
        self.load();
        drop(another_space)
    }
}

// Create address spaces
impl AddrSpace {
    pub fn make_kernel() -> Self {
        debug!("AddrSpace::make_kernel: making address space for the kernel");
        let mut virt_areas: Vec<VirtArea> = Vec::new();
        // Trampoline page
        virt_areas.push({
            let area = VirtArea::make_trampoline();
            area.print_info();
            area
        });

        // Identically mapped physical memory

        // heap region: readable, writable
        // (the frame allocator manages this region)
        virt_areas.push({
            let pa_begin = PhysAddr::new(__heap_start());
            let pa_end = PhysAddr::new(__heap_end());
            let perms = PageFlags::READABLE | PageFlags::WRITABLE;
            let mut area = VirtArea::identically_mapped(pa_begin, pa_end, perms);
            area.set_name("heap");
            area.print_info();
            area
        });

        // kernel boot stack
        virt_areas.push({
            let pa_begin = PhysAddr::new(__kernel_stack_start());
            let pa_end = PhysAddr::new(__kernel_stack_end());
            let perms = PageFlags::READABLE | PageFlags::WRITABLE;
            let mut area = VirtArea::identically_mapped(pa_begin, pa_end, perms);
            area.set_name("kernel boot stack");
            area.print_info();
            area
        });

        // bss: readable, writable
        virt_areas.push({
            let pa_begin = PhysAddr::new(__bss_start());
            let pa_end = PhysAddr::new(__bss_end());
            let perms = PageFlags::READABLE | PageFlags::WRITABLE;
            let mut area = VirtArea::identically_mapped(pa_begin, pa_end, perms);
            area.set_name(".bss (+kernel heap)");
            area.print_info();
            area
        });

        // data: readable, writable
        virt_areas.push({
            let pa_begin = PhysAddr::new(__data_start());
            let pa_end = PhysAddr::new(__data_end());
            let perms = PageFlags::READABLE | PageFlags::WRITABLE;

            let mut area = VirtArea::identically_mapped(pa_begin, pa_end, perms);
            area.set_name(".data");
            area.print_info();
            area
        });

        // rodata: readable
        virt_areas.push({
            let pa_begin = PhysAddr::new(__rodata_start());
            let pa_end = PhysAddr::new(__rodata_end());
            let perms = PageFlags::READABLE;
            let mut area = VirtArea::identically_mapped(pa_begin, pa_end, perms);
            area.set_name(".rodata");
            area.print_info();
            area
        });

        // text: readable, executable
        virt_areas.push({
            let pa_begin = PhysAddr::new(__text_start());
            let pa_end = PhysAddr::new(__text_end());
            let perms = PageFlags::READABLE | PageFlags::EXECUTABLE;
            let mut area = VirtArea::identically_mapped(pa_begin, pa_end, perms);
            area.set_name(".text");
            area.print_info();
            area
        });

        // map memory-mapped registers
        // virt-io
        virt_areas.push({
            let pa_begin = PhysAddr::new(VIRTIO_BASE);
            let pa_end = PhysAddr::new(VIRTIO_BASE + VIRTIO_SIZE);
            let perms = PageFlags::READABLE | PageFlags::WRITABLE;
            let mut area = VirtArea::identically_mapped(pa_begin, pa_end, perms);
            area.set_name("virtio");
            area.print_info();
            area
        });

        // uart
        virt_areas.push({
            let pa_begin = PhysAddr::new(UART_BASE);
            let pa_end = PhysAddr::new(UART_BASE + UART_SIZE);
            let perms = PageFlags::READABLE | PageFlags::WRITABLE;
            let mut area = VirtArea::identically_mapped(pa_begin, pa_end, perms);
            area.set_name("uart");
            area.print_info();
            area
        });

        // plic
        virt_areas.push({
            let pa_begin = PhysAddr::new(PLIC_BASE);
            let pa_end = PhysAddr::new(PLIC_BASE + PLIC_SIZE);
            let perms = PageFlags::READABLE | PageFlags::WRITABLE;
            let mut area = VirtArea::identically_mapped(pa_begin, pa_end, perms);
            area.set_name("plic");
            area.print_info();
            area
        });

        // clint
        virt_areas.push({
            let pa_begin = PhysAddr::new(CLINT_BASE);
            let pa_end = PhysAddr::new(CLINT_BASE + CLINT_SIZE);
            let perms = PageFlags::READABLE | PageFlags::WRITABLE;
            let mut area = VirtArea::identically_mapped(pa_begin, pa_end, perms);
            area.set_name("clint");
            area.print_info();
            area
        });

        let mut page_table = PageTableGuard::allocate();
        for virt_area in &virt_areas {
            // info!("mapping virtual area: {:?}", virt_area);
            page_table.map_virt_area_allocate(virt_area);
        }

        Self {
            page_table,
            virt_areas,
        }
    }

    pub fn make_init() -> Self {
        debug!("AddrSpace::make_init: making address space for the init process");
        let init_text = init_code_bytes(); // it is in the kernel binary
        let mut virt_areas = Vec::new();

        let text_va_begin = VirtAddr::new(TEXT_BASE_USER_VA);
        let text_va_end = (text_va_begin + init_text.len()).align_up();
        let user_stack_va = text_va_end + PAGE_SIZE;

        // trampoline
        virt_areas.push({
            let area = VirtArea::make_trampoline();
            area.print_info();
            area
        });

        // We skip mapping the trapframe to simplify the API
        // it should be allocated in `init_trapframe` to make things more clear
        info!("AddrSpace::make_init: skipping trapframe, remember to call AddrSpace::init_trapframe if you don't see it");

        // user stack
        virt_areas.push({
            let (area, _) = VirtArea::make_initial_user_stack(user_stack_va);
            area.print_info();
            area
        });

        //text
        virt_areas.push({
            let va_begin = text_va_begin;
            let va_end = text_va_end;
            let pa_start = PhysAddr::new(init_text.as_ptr() as usize);
            let perms = PageFlags::READABLE | PageFlags::EXECUTABLE | PageFlags::USER;

            let mut virt_area = VirtArea::new(va_begin, va_end, perms);
            // Note: the init code is compiled into the kernel binary, so we do not own it
            let phys_frame = Frame::from_phys_addr(pa_start);
            virt_area.track_frame(va_begin, VirtFrameGuard::PhysBorrowed(phys_frame));
            virt_area.set_name(".text");
            virt_area.print_info();
            virt_area
        });

        let mut page_table = PageTableGuard::allocate();

        for virt_area in &virt_areas {
            page_table.map_virt_area_allocate(virt_area);
        }

        Self {
            page_table,
            virt_areas,
        }
    }

    /// Don't forget to call it to allocate a trapframe!!
    /// User address space need it!!! (not the kernel though)
    pub fn init_trapframe(&mut self) -> PhysAddr {
        info!("AddrSpace::init_trapframe: initialising trapframe, you should see this when allocating for user-spaces");
        let (area, pa) = VirtArea::make_trapframe();
        area.print_info();
        self.page_table.map_virt_area_allocate(&area);
        self.virt_areas.push(area);
        pa
    }
}

impl Drop for AddrSpace {
    fn drop(&mut self) {
        let pa = self
            .page_table
            .get_root_frame()
            .get_base_phys_addr()
            .as_usize();
        debug!(
            "AddrSpace::drop: address space with page table at pa {:?} deallocated",
            pa as *const usize
        );
    }
}

// ------------------------- Virtual Area ---------------------------------------
// a Virtual Area is a logically contiguous region in the virtual address space

// I cannot think of a case where a logically contiguous span of pages can be provided
// by different providers... So for now let's factor the provider into

#[derive(Debug)]
pub struct VirtArea {
    /// an `SimpleRange<VirtFrame>` instance,
    /// has `get_begin` and `n_pages` methods
    pub virt_frame_range: VirtFrameRange,

    // TODO: unfriendly towards large chunk of mapping
    //  abstract a guard for `VirtArea` instead???
    /// maintains a `VirtAddr` -> (Physical) `Frame` mapping
    pub virt_frames: BTreeMap<VirtAddr, VirtFrameGuard>,

    // TODO: permissions? Doesn't it duplicate what we already have
    //  in the page table??
    pub permissions: PageFlags,

    // TODO: maybe use an enum?
    pub is_identically_mapped: bool,

    // debug
    pub name: String,
}

// ExclusivelyAllocated ----- COW Read -----> CowShared
//         /\                                   |
//         |                                    |
//         + --------- COW Write --------------+

impl VirtArea {
    pub fn new(va_begin: VirtAddr, va_end: VirtAddr, perms: PageFlags) -> Self {
        let begin = va_begin.align_down().into();
        let end = va_end.align_up().into();
        Self {
            virt_frame_range: VirtFrameRange::new(begin, end),
            virt_frames: BTreeMap::new(),
            permissions: perms,
            is_identically_mapped: false,
            name: "".into(),
        }
    }

    /// Identically map the memory.
    /// It does not track the physical frame because no-one owns it.
    /// It just IS...
    /// The only use case so far is to construct the kernel address space
    pub fn identically_mapped(pa_begin: PhysAddr, pa_end: PhysAddr, perms: PageFlags) -> Self {
        // debug!(
        //     "VirtArea::identically_mapped: pa_begin={:?}, pa_end={:?}, length={:?}",
        //     pa_begin.as_usize() as *const usize,
        //     pa_end.as_usize() as *const usize,
        //     pa_end - pa_begin
        // );

        let pa_begin: Frame = pa_begin.align_down().into();
        let pa_end: Frame = pa_end.align_up().into();
        // debug!(
        //     "VirtArea::identically_mapped: pa_begin={:?}, pa_end={:?}, length={:?} (PAGES)",
        //     pa_begin.get_base_phys_addr().as_usize() as *const usize,
        //     pa_end.get_base_phys_addr().as_usize() as *const usize,
        //     (pa_end.get_base_phys_addr() - pa_begin.get_base_phys_addr()) / PAGE_SIZE
        // );

        let phys_rng = FrameRange::new(pa_begin, pa_end);

        // NOTE: We do not track unnecessary maps since dropping it doesn't effect anything
        // Besides, identically mapping the physical memory is A LOT of pages!!!
        // Which will soon take all the space in the kernel heap

        Self {
            // uses `Copy` since it is implemented for SimpleRange<Frame>
            virt_frame_range: VirtFrameRange::from_identical(phys_rng),
            virt_frames: BTreeMap::new(),
            permissions: perms,
            is_identically_mapped: true,
            name: "".into(),
        }
    }

    pub fn make_trampoline() -> Self {
        let va_begin = VirtAddr::new(TRAMPOLINE_BASE_VA);
        let va_end = VirtAddr::new(MAX_VA);
        let perms = PageFlags::READABLE | PageFlags::EXECUTABLE;
        let mut virt_area = VirtArea::new(va_begin, va_end, perms);

        // Note: the trampoline is not owned by anyone, it is inside the kernel binary
        let phys_frame = Frame::from_phys_addr(PhysAddr::new(__trampoline_start()));
        virt_area.track_frame(va_begin, VirtFrameGuard::PhysBorrowed(phys_frame));
        virt_area.set_name("trampoline");
        virt_area
    }

    pub fn make_trapframe() -> (Self, PhysAddr) {
        let va_begin = VirtAddr::new(TRAPFRAME_BASE_USER_VA);
        let va_end = VirtAddr::new(TRAPFRAME_BASE_USER_VA + TRAPFRAME_SIZE).align_up();
        let perms = PageFlags::READABLE | PageFlags::WRITABLE;
        let mut virt_area = VirtArea::new(va_begin, va_end, perms);

        // Note: the trapframe is allocated specifically for the process, and should
        // be managed by the user address space
        let phys_frame = FrameGuard::allocate_zeroed();
        let pa = phys_frame.get_frame().get_base_phys_addr();
        virt_area.track_frame(va_begin, VirtFrameGuard::ExclusivelyAllocated(phys_frame));
        virt_area.set_name("trapframe");
        (virt_area, pa)
    }

    pub fn make_initial_user_stack(user_stack_va: VirtAddr) -> (Self, PhysAddr) {
        let va_begin = user_stack_va;
        let va_end = user_stack_va + PAGE_SIZE;
        let perms = PageFlags::READABLE | PageFlags::WRITABLE | PageFlags::USER;
        let mut virt_area = VirtArea::new(va_begin, va_end, perms);

        // We own the user stack since we explicitly called for its allocation
        let phys_frame = FrameGuard::allocate_zeroed();
        let pa = phys_frame.get_frame().get_base_phys_addr();
        virt_area.track_frame(va_begin, VirtFrameGuard::ExclusivelyAllocated(phys_frame));
        virt_area.set_name("user stack");
        (virt_area, pa)
    }

    pub fn permissions(&self) -> PageFlags {
        self.permissions
    }

    pub fn track_frame(&mut self, va: VirtAddr, frame_guard: VirtFrameGuard) {
        // NOTE: move does a bitwise copy from the old instance to the new instance
        //       and invalidate the old one.
        //       The old one is forgotten and its desctructor will not be run!!!
        // See the similar consept in C++ Move Semantics

        // debug!(
        //     "VirtArea::track_frame: add mapping {:?} -> {:?}",
        //     va.as_usize() as *const usize,
        //     frame_guard.as_usize() as *const usize
        // );
        self.virt_frames.insert(va, frame_guard);
    }

    pub fn get_virt_frames(&mut self) -> &mut BTreeMap<VirtAddr, VirtFrameGuard> {
        &mut self.virt_frames
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.into();
    }

    pub fn print_info(&self) {
        let va_begin = self.virt_frame_range.get_begin().get_base_virt_addr();
        let va_end = self.virt_frame_range.get_end().get_base_virt_addr();
        info!(
            "\t{:13?}{:13?}\t{:?}\t{:?}",
            va_end.as_usize() as *const usize,
            va_begin.as_usize() as *const usize,
            self.permissions,
            self.name,
        );
    }
}
