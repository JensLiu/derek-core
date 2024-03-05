use core::arch::asm;

use alloc::{collections::BTreeMap, vec::Vec};
use riscv::{
    asm::{fence, sfence_vma_all},
    register::satp,
};

use crate::{
    debug, info,
    mm::layout::{
        __bss_end, __bss_start, __data_end, __data_start, __heap_end, __heap_start,
        __kernel_binary_end, __kernel_heap_start, __kernel_stack_end, __kernel_stack_start,
        __rodata_end, __rodata_start, __text_end, __text_start, __trampoline_start, MAX_VA,
        PAGE_SIZE, TRAMPOLINE_VA,
    },
};

use super::{
    layout::{
        __kernel_heap_end, CLINT_BASE, CLINT_SIZE, PLIC_BASE, PLIC_SIZE, UART_BASE, UART_SIZE,
        VIRTIO0, VIRTIO_BASE, VIRTIO_SIZE,
    },
    memory::{Frame, FrameRange, PhysAddr, VirtAddr, VirtFrameGuard, VirtFrameRange},
    page_table::{PageFlags, PageTableGuard},
};

// ------------------------- Address Space -------------------------------------
// an abstraction of a logical address space it owns
// (1) a `PageTable` with its `node_frames`
// (2) Its`VirtArea`s with their underlying `data_frames`
pub struct AddrSpace {
    page_table: PageTableGuard,
    virt_areas: Vec<VirtArea>,
}

impl AddrSpace {
    pub fn load(&self) {
        let ptr = self.page_table.make_satp();
        unsafe {
            satp::write(ptr);
            asm!("sfence.vma"); // memory fence to flush TLB
        };
        // let pa = self.page_table.get_root_frame().get_base_phys_addr().as_ptr::<usize>();
        let access_time = crate::arch::time();
        // debug!("page table loaded: {:?}", ptr as *const usize);
        // these are pretty much a wrapper function to the underlying RISC-V instructions
    }

    /// It is recommended to verify before loading the page table
    pub fn verify(&self) {
        for virt_area in self.virt_areas.iter().rev() {
            self.page_table.verify_virt_area_mapping(virt_area);
        }
    }
}

// Create address spaces
impl AddrSpace {
    pub fn make_kernel() -> Self {
        let mut virt_areas: Vec<VirtArea> = Vec::new();
        // Trampoline page
        virt_areas.push({
            let va_begin = VirtAddr::new(TRAMPOLINE_VA);
            let va_end = VirtAddr::new(MAX_VA);
            let perms = PageFlags::READABLE | PageFlags::EXECUTABLE;
            // debug!(
            //     "trampoline: \t {:?} -- {:?},\t {:?},\t {:?}",
            //     va_begin.as_usize() as *const usize,
            //     va_end.as_usize() as *const usize,
            //     va_end - va_begin,
            //     perms,
            // );
            let mut virt_area = VirtArea::new(va_begin, va_end, perms);
            let phys_frame = Frame::from_phys_addr(PhysAddr::new(__trampoline_start()));
            virt_area.track_frame(va_begin, VirtFrameGuard::PhysBorrowed(phys_frame));

            virt_area
        });

        // Identically mapped physical memory

        // heap region: readable, writable
        // (the frame allocator manages this region)
        virt_areas.push({
            let pa_begin = PhysAddr::new(__heap_start());
            let pa_end = PhysAddr::new(__heap_end());
            let perms = PageFlags::READABLE | PageFlags::WRITABLE;
            // debug!(
            //     "kernel heap:\t {:?} -- {:?}, \t {:?}, \t {:?}",
            //     pa_begin.as_usize() as *const usize,
            //     pa_end.as_usize() as *const usize,
            //     pa_end - pa_begin,
            //     perms,
            // );
            VirtArea::identically_mapped(pa_begin, pa_end, perms)
        });

        // kernel boot stack
        virt_areas.push({
            let pa_begin = PhysAddr::new(__kernel_stack_start());
            let pa_end = PhysAddr::new(__kernel_stack_end());
            let perms = PageFlags::READABLE | PageFlags::WRITABLE;
            // debug!(
            //     "kernel stack:\t {:?} -- {:?}, \t {:?}, \t {:?}",
            //     pa_begin.as_usize() as *const usize,
            //     pa_end.as_usize() as *const usize,
            //     pa_end - pa_begin,
            //     perms,
            // );
            VirtArea::identically_mapped(pa_begin, pa_end, perms)
        });

        // bss: readable, writable
        virt_areas.push({
            let pa_begin = PhysAddr::new(__bss_start());
            let pa_end = PhysAddr::new(__bss_end());
            let perms = PageFlags::READABLE | PageFlags::WRITABLE;
            // debug!(
            //     ".bss: \t {:?} -- {:?}, \t {:?}, \t {:?}",
            //     pa_begin.as_usize() as *const usize,
            //     pa_end.as_usize() as *const usize,
            //     pa_end - pa_begin,
            //     perms,
            // );
            VirtArea::identically_mapped(pa_begin, pa_end, perms)
        });

        // data: readable, writable
        virt_areas.push({
            let pa_begin = PhysAddr::new(__data_start());
            let pa_end = PhysAddr::new(__data_end());
            let perms = PageFlags::READABLE | PageFlags::WRITABLE;
            // debug!(
            //     ".data: \t {:?} -- {:?}, \t {:?}, \t {:?}",
            //     pa_begin.as_usize() as *const usize,
            //     pa_end.as_usize() as *const usize,
            //     pa_end - pa_begin,
            //     perms,
            // );
            VirtArea::identically_mapped(pa_begin, pa_end, perms)
        });

        // rodata: readable
        virt_areas.push({
            let pa_begin = PhysAddr::new(__rodata_start());
            let pa_end = PhysAddr::new(__rodata_end());
            let perms = PageFlags::READABLE;
            // debug!(
            //     ".rodata: \t {:?} -- {:?}, \t {:?}, \t {:?}",
            //     pa_begin.as_usize() as *const usize,
            //     pa_end.as_usize() as *const usize,
            //     pa_end - pa_begin,
            //     perms,
            // );
            VirtArea::identically_mapped(pa_begin, pa_end, perms)
        });

        // text: readable, executable
        virt_areas.push({
            let pa_begin = PhysAddr::new(__text_start());
            let pa_end = PhysAddr::new(__text_end());
            let perms = PageFlags::READABLE | PageFlags::EXECUTABLE;
            // debug!(
            //     ".text: \t {:?} -- {:?}, \t {:?}, \t {:?}",
            //     pa_begin.as_usize() as *const usize,
            //     pa_end.as_usize() as *const usize,
            //     pa_end - pa_begin,
            //     perms,
            // );
            VirtArea::identically_mapped(pa_begin, pa_end, perms)
        });

        // map memory-mapped registers
        // virt-io
        virt_areas.push({
            let pa_begin = PhysAddr::new(VIRTIO_BASE);
            let pa_end = PhysAddr::new(VIRTIO_BASE + VIRTIO_SIZE);
            let perms = PageFlags::READABLE | PageFlags::WRITABLE;
            // debug!(
            //     "virtio: \t {:?} -- {:?}, \t {:?}, \t {:?}",
            //     pa_begin.as_usize() as *const usize,
            //     pa_end.as_usize() as *const usize,
            //     pa_end - pa_begin,
            //     perms,
            // );
            VirtArea::identically_mapped(pa_begin, pa_end, perms)
        });

        // uart
        virt_areas.push({
            let pa_begin = PhysAddr::new(UART_BASE);
            let pa_end = PhysAddr::new(UART_BASE + UART_SIZE);
            let perms = PageFlags::READABLE | PageFlags::WRITABLE;
            // debug!(
            //     "uart: \t {:?} -- {:?}, \t {:?}, \t {:?}",
            //     pa_begin.as_usize() as *const usize,
            //     pa_end.as_usize() as *const usize,
            //     pa_end - pa_begin,
            //     perms,
            // );
            VirtArea::identically_mapped(pa_begin, pa_end, perms)
        });

        // plic
        virt_areas.push({
            let pa_begin = PhysAddr::new(PLIC_BASE);
            let pa_end = PhysAddr::new(PLIC_BASE + PLIC_SIZE);
            let perms = PageFlags::READABLE | PageFlags::WRITABLE;
            // debug!(
            //     "plic: \t {:?} -- {:?}, \t {:?}, \t {:?}",
            //     pa_begin.as_usize() as *const usize,
            //     pa_end.as_usize() as *const usize,
            //     pa_end - pa_begin,
            //     perms,
            // );
            VirtArea::identically_mapped(pa_begin, pa_end, perms)
        });

        // clint
        virt_areas.push({
            let pa_begin = PhysAddr::new(CLINT_BASE);
            let pa_end = PhysAddr::new(CLINT_BASE + CLINT_SIZE);
            let perms = PageFlags::READABLE | PageFlags::WRITABLE;
            debug!(
                "clint: \t {:?} -- {:?}, \t {:?}, \t {:?}",
                pa_begin.as_usize() as *const usize,
                pa_end.as_usize() as *const usize,
                pa_end - pa_begin,
                perms,
            );
            VirtArea::identically_mapped(pa_begin, pa_end, perms)
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
        }
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
}
