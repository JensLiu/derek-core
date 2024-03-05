use core::{
    arch::asm,
    ops::{Add, Sub},
    slice,
};

use alloc::sync::Arc;

use crate::{
    allocator::frame_allocator::{allocate_one_frame, deallocate_one_frame},
    debug, impl_address_arithmetics, info,
};

use super::{
    arithmetics::{SimpleRange, StepByOne, PG_ROUND_DOWN, PG_ROUND_UP},
    layout::PAGE_SIZE,
};

// --------------------------- Physical Address ------------------------ //
// It does not own the underlying memory, just an representation
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysAddr(usize);

impl_address_arithmetics!(PhysAddr);

impl PhysAddr {
    pub fn new(pa: usize) -> Self {
        Self(pa)
    }

    pub fn as_mut_ptr<T>(&self) -> *mut T {
        self.0 as *mut T
    }

    pub fn as_ptr<T>(&self) -> *const T {
        self.0 as *const T
    }

    pub fn as_ref_mut<T>(&mut self) -> &'static mut T {
        unsafe { &mut *self.as_mut_ptr() }
    }

    pub fn as_ref<T>(&self) -> &'static T {
        unsafe { &*self.as_ptr() }
    }

    pub fn with_offset(self, offset: usize) -> Self {
        let offset_mask = (1 << VA_OFFSET_WIDTH) - 1;
        let addr = (self.0 & !offset_mask) | (offset & offset_mask);
        Self(addr)
    }
}

// ------------------------ Virtual Address ---------------------------------
// Virtual Address Organisation of RISC-V
//                                9 bits   9 btis   9 bits    12 bits
// +--------------------------+---------+--------+--------+------------+
// |           EXT            |   L2    |   L1   |   L0   |   Offset   |
// +--------------------------+---------+--------+--------+------------+
const VA_OFFSET_WIDTH: usize = 12; // 12-bit offset
const VA_INDEX_WIDTH: usize = 9; // 9-bit index

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct VirtAddr(usize);

impl_address_arithmetics!(VirtAddr);

impl VirtAddr {
    pub fn new(addr: usize) -> Self {
        Self(addr)
    }

    pub fn from_identical(addr: PhysAddr) -> Self {
        Self(addr.0)
    }

    pub fn pte_index(&self, level: usize) -> usize {
        if level > 2 {
            panic!("VirtualAddress::index");
        }

        let vpn = VirtFrame::from_virt_addr(self.clone()).number;
        let shift = level * VA_INDEX_WIDTH;
        let index_mask = (1 << VA_INDEX_WIDTH) - 1;
        (vpn >> shift) & index_mask
    }

    pub fn offset(&self) -> usize {
        let va = self.0;
        va & VA_OFFSET_WIDTH
    }
}

// --------------------------- Physical Page (Frame) ------------------------ //
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Frame {
    pub number: usize, // physical page number
}

impl Frame {
    pub fn from_ppn(ppn: usize) -> Self {
        Self { number: ppn }
    }

    pub fn from_phys_addr(pa: PhysAddr) -> Self {
        Self {
            number: pa.get_number(PAGE_SIZE),
        }
    }

    pub fn get_base_phys_addr(&self) -> PhysAddr {
        PhysAddr(self.number * PAGE_SIZE)
    }

    pub unsafe fn get_bytes(&self) -> &'static mut [u8] {
        let pa = self.get_base_phys_addr().as_mut_ptr();
        let len = PAGE_SIZE;
        slice::from_raw_parts_mut(pa, len)
    }

    pub fn zero(&mut self) {
        for byte in unsafe { self.get_bytes() } {
            *byte = 0;
        }
    }
}

impl From<PhysAddr> for Frame {
    fn from(pa: PhysAddr) -> Self {
        Self::from_phys_addr(pa.align_down())
    }
}

impl StepByOne for Frame {
    fn step_one(&mut self) {
        self.number += 1;
    }
}

// TODO: simialr to `VirtFrameRange`, macro?
pub type FrameRange = SimpleRange<Frame>;

impl FrameRange {
    pub fn n_pages(&self) -> usize {
        self.get_end().number - self.get_begin().number
    }
}

// impl Clone for FrameRange {
//     fn clone(&self) -> Self {
//         Self::new(self.get_begin().clone(), self.get_end().clone())
//     }
// }

// impl Copy for FrameRange {}

#[repr(C)]
#[derive(Debug)]
pub struct FrameGuard {
    inner: Option<Frame>,
    // unmapped: bool, // Guard against premature drop
}

impl FrameGuard {
    /// `FrameGuard::allocate` allocates one frame from the frame allocator
    // pub fn allocate() -> Self {
    //     let frame: Frame = allocate_one_frame().into();
    //     Self { inner: Some(frame) }
    // }

    pub fn allocate_zeroed() -> Self {
        let mut frame: Frame = allocate_one_frame().into();
        frame.zero();
        Self { inner: Some(frame) }
    }

    /// start managing the frame
    pub fn from_frame(frame: Frame) -> Self {
        Self {
            inner: Some(frame),
            // unmapped: false,
        }
    }

    // Only `RaiiFrame::take_frame` can remove the content of `RaiiFrame.inner`
    // But it also consumes itself.
    // So `RailFrame::inner` always has a frame
    pub fn inner_ref_mut(&mut self) -> &mut Frame {
        self.inner
            .as_mut()
            .expect("FrameGuard::inner_ref_mut: no inner")
    }

    pub fn inner_ref(&self) -> &Frame {
        self.inner
            .as_ref()
            .expect("FrameGuard::inner_ref: no inner")
    }

    /// Copies its frame to use outside
    pub fn get_frame(&self) -> Frame {
        *self
            .inner
            .as_ref()
            .expect("FrameGuard::get_frame: no inner")
    }

    // ignore the recycling process by taking the inner frame
    // It can only be called once, otherwise panic
    // Using it may result in memory leak!
    pub unsafe fn take(mut self) -> Frame {
        self.inner
            .take()
            .expect("FrameGuard::take_frame: called more than once!")
    }

    // pub unsafe fn set_unmapped(&mut self) {
    // self.unmapped = true;
    // }
}

impl From<Frame> for FrameGuard {
    fn from(value: Frame) -> Self {
        Self {
            inner: Some(value),
            // unmapped: false
        }
    }
}

impl Drop for FrameGuard {
    fn drop(&mut self) {
        // TODO: How to make sure it is in sync with the page table???
        if let Some(frame) = self.inner.as_mut() {
            // assert!(self.unmapped, "FrameGuard::drop: still mapped by some page tables");
            debug!(
                "FrameGuard::drop: phys_addr: {:?}",
                frame.get_base_phys_addr()
            );
            deallocate_one_frame(frame.get_base_phys_addr());
            unsafe {
                // To expose bugs
                asm!("sfence.vma");
            }
        }
    }
}

// ------------------------- Vitural Page (Frame) -----------------------------------
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct VirtFrame {
    number: usize, // virtual page number
}

impl VirtFrame {
    pub fn from_identical(phys_frame: Frame) -> Self {
        Self {
            number: phys_frame.number,
        }
    }

    pub fn from_virt_addr(va: VirtAddr) -> Self {
        let base_addr = va.align_down().into_usize();
        let vpn = base_addr / PAGE_SIZE;
        Self { number: vpn }
    }

    pub fn get_base_virt_addr(&self) -> VirtAddr {
        VirtAddr(self.number * PAGE_SIZE)
    }
}

impl From<VirtAddr> for VirtFrame {
    fn from(va: VirtAddr) -> Self {
        assert!(
            va.is_page_aligned(),
            "VirtFrame::from<VirtAddr>: address {:?} not page aligned!",
            va.0,
        );
        Self {
            number: va.get_number(PAGE_SIZE),
        }
    }
}

impl StepByOne for VirtFrame {
    fn step_one(&mut self) {
        self.number += 1;
    }
}

pub type VirtFrameRange = SimpleRange<VirtFrame>;

impl VirtFrameRange {
    /// Shorthand for identically mapped virtual range
    pub fn from_identical(phys_rng: FrameRange) -> Self {
        Self::new(
            VirtFrame::from_identical(phys_rng.get_begin()),
            VirtFrame::from_identical(phys_rng.get_end()),
        )
    }

    pub fn n_pages(&self) -> usize {
        self.get_end().number - self.get_begin().number
    }
}

#[derive(Debug)]
pub enum VirtFrameGuard {
    ExclusivelyAllocated(FrameGuard),
    CowShared(Arc<FrameGuard>),
    PhysBorrowed(Frame),
}

impl VirtFrameGuard {
    pub fn as_usize(&self) -> usize {
        match &self {
            VirtFrameGuard::ExclusivelyAllocated(frame_guard) => {
                frame_guard.get_frame().get_base_phys_addr().as_usize()
            }
            VirtFrameGuard::CowShared(frame_guard_arc) => frame_guard_arc
                .as_ref()
                .get_frame()
                .get_base_phys_addr()
                .as_usize(),
            VirtFrameGuard::PhysBorrowed(frame) => frame.get_base_phys_addr().as_usize(),
        }
    }
}
