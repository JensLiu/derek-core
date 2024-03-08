use core::slice;

use alloc::vec::Vec;
use bitflags::bitflags;

#[allow(unused)]
use crate::{debug, info};

use super::{
    address_space::VirtArea,
    arithmetics::PTE2PA,
    memory::{Frame, FrameGuard, PhysAddr, VirtAddr, VirtFrameGuard},
};

#[allow(unused)]
const ENTRY_PER_TABLE: usize = 512;

// This is a managing instance of a page table node
#[repr(transparent)]
#[derive(Debug)]
pub struct PageTableNode {
    base_addr: PhysAddr,
}

impl PageTableNode {
    /// SAFETY:
    /// (1) There can only be one accessing instance
    /// (2) No one can read/write this memory while a reference is already held
    unsafe fn table(&self) -> &'static mut [PageTableEntry] {
        let page_table_frame: Frame = self.base_addr.clone().into();
        let first_entry = page_table_frame.get_base_phys_addr().as_mut_ptr();

        slice::from_raw_parts_mut(first_entry, ENTRY_PER_TABLE)
    }

    // pub unsafe fn entry_at(&self, index: usize) -> PageTableEntry {
    //     if index > ENTRY_PER_TABLE {
    //         panic!("PageTableNode: invalid index");
    //     }
    //     let table = self.table();
    //     table[index] // uses Copy
    // }

    pub unsafe fn entry_at(&self, index: usize) -> &PageTableEntry {
        if index > ENTRY_PER_TABLE {
            panic!("PageTableNode: invalid index");
        }
        let table = self.table();
        table.get(index).unwrap()
    }

    pub unsafe fn set_entry(&self, index: usize, entry: PageTableEntry) {
        if index > ENTRY_PER_TABLE {
            panic!("PageTableNode: invalid index");
        }
        let table = self.table();
        table[index] = entry; // copies the underlying bits
    }

    pub unsafe fn from_frame(frame: &Frame) -> Self {
        Self {
            base_addr: frame.get_base_phys_addr(),
        }
    }
}

// represents a PTE
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct PageTableEntry {
    bits: usize,
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]  // TODO: otherwise moved and cannot be used in a loop, better solution?
    pub struct PTEFlags: u16 {
        const VALID = 1 << 0;
        const READABLE = 1 << 1;
        const WRITABLE = 1 << 2;
        const EXECUTABLE = 1 << 3;
        const USER = 1 << 4;
        const GLOBAL = 1 << 5;
        const ACCESSED = 1 << 6;
        const DIRTY = 1 << 7;
        // custom
        const COW = 1 << 8;
    }
}

impl From<PageFlags> for PTEFlags {
    fn from(perms: PageFlags) -> Self {
        Self::from_bits_retain(perms.bits())
    }
}

bitflags! {
    /// a subset of `PTEFlags` that are exposed
    #[derive(Clone, Copy, Debug)]
    pub struct PageFlags: u16 {
        const READABLE = 1 << 1;
        const WRITABLE = 1 << 2;
        const EXECUTABLE = 1 << 3;
        const USER = 1 << 4;
    }
}

impl From<PTEFlags> for PageFlags {
    fn from(flags: PTEFlags) -> Self {
        Self::from_bits_retain(flags.bits())
    }
}

impl PageTableEntry {
    pub fn new(pa: PhysAddr, flags: PTEFlags) -> Self {
        Self {
            bits: Self::make_entry(pa, flags),
        }
    }

    pub fn empty() -> Self {
        Self { bits: 0 }
    }

    fn make_entry(pa: PhysAddr, flags: PTEFlags) -> usize {
        let frame: Frame = pa.into();
        frame.number << 10 | flags.bits() as usize
        // ((frame.number & !0xfff) >> 2) | flags.bits() as usize
    }

    pub fn flags(&self) -> PTEFlags {
        let flags_raw = self.bits & (1 << 10) - 1;
        PTEFlags::from_bits(flags_raw as u16).unwrap()
    }

    /// get the referencing physical address (page-aligned)
    /// from PTE
    pub fn referencing_address(&self) -> PhysAddr {
        PhysAddr::new(PTE2PA(self.bits))
    }

    /// get the referencing frame from PTE
    pub fn referencing_frame(&self) -> Frame {
        // let ppn = self.bits >> 10 & ((1usize << 44) - 1);
        // Frame::from_ppn(ppn)
        let pa = self.referencing_address();
        assert!(
            pa.is_page_aligned(),
            "PageTableEntry::referencing_frame: is not page aligned"
        );
        Frame::from_phys_addr(pa)
    }

    pub fn is_valid(&self) -> bool {
        self.flags().contains(PTEFlags::VALID)
    }
}

// impl Into<PhysicalAddress> for PageTableEntry {
//     fn into(self) -> PhysicalAddress {
//         self.referencing_address()
//     }
// }

impl Into<Frame> for PageTableEntry {
    fn into(self) -> Frame {
        self.referencing_frame()
    }
}

/// The RAII managing instance of the page table
/// Use smart pointers to track the page table, otherwise
/// It will be released
#[derive(Debug)]
pub struct PageTableGuard {
    /// its root node. Note that a `PageTableNode` instance
    /// only stores its `PhysAddr`, it does not manage any resources
    root_node: PageTableNode,

    /// Physical `FrameGuard` that deallocates when this page table gets dropped
    /// Only contains physical `Frame`s that are allocated for this `PageTable`.
    /// Does not include physical `Frame`s that it maps to
    node_frames: Vec<FrameGuard>,
}

impl PageTableGuard {
    pub fn get_root_frame(&self) -> Frame {
        Frame::from_phys_addr(self.root_node.base_addr)
    }

    pub fn make_satp(&self) -> usize {
        let ptr = self.root_node.base_addr.as_usize();
        const SATP_SV39: usize = 8 << 60;
        SATP_SV39 | ptr >> 12
    }

    /// `PageTableGuard::allocate` allocates the root node of the page table
    /// From there use `PageTableGuard::map_one_allocate` can allocate its interior node
    pub fn allocate() -> Self {
        let root_node_frame_guard = FrameGuard::allocate_zeroed();

        // safety: it is allocated, hence valid
        let root_node = unsafe { PageTableNode::from_frame(root_node_frame_guard.inner_ref()) };

        Self {
            root_node,
            node_frames: vec![root_node_frame_guard],
        }
    }

    /// Interior function to allocate one `PageTableNode` frame
    /// and tracks it as its interior `node_frame`
    fn allocate_node(&mut self) -> Frame {
        let node_frame = FrameGuard::allocate_zeroed();
        let frame = node_frame.get_frame();
        self.node_frames.push(node_frame);
        frame
    }

    pub fn translate(&self, va: VirtAddr) -> Option<(PhysAddr, PTEFlags)> {
        let pte = self.find(va)?;
        Some((
            pte.referencing_address().with_offset(va.offset()),
            pte.flags(),
        ))
    }

    pub fn find_allocate(&mut self, va: VirtAddr) -> &'static mut PageTableEntry {
        // debug!(
        //     "PageTableGuard::find_allocate: find PTE for virtaddr: {:?}",
        //     va.as_usize() as *const usize
        // );
        let mut table = unsafe { self.root_node.table() };

        for level in (0..=2).rev() {
            // debug!(
            //     "----------------------- level-{:?} page table node at: {:?} -----------------------------",
            //     level,
            //     table.as_ptr(),
            // );
            let index = va.pte_index(level);

            // info!("index at {:?}", index);
            let pte = table
                .get_mut(index)
                .expect("PageTable::map: invalid entry index");

            if level == 0 {
                // debug!(
                //     "0-level PTE: bits:{:?} referencing_physaddr: {:?}, flags: {:?}",
                //     pte.bits,
                //     pte.referencing_address().as_usize() as *const u32,
                //     pte.flags()
                // );
                return pte;
            }

            if !pte.is_valid() {
                // for interior nodes, allocate its next-level node
                // and fill the corresponding PTE
                let node_pa = self.allocate_node().get_base_phys_addr();
                // debug!(
                //     "Invalid PTE: allocated next-level node as: {:?}",
                //     node_pa.as_usize() as *const usize
                // );
                *pte = PageTableEntry::new(node_pa, PTEFlags::VALID);
                assert_eq!(pte.referencing_address(), node_pa);
                assert_eq!(pte.flags().bits(), PTEFlags::VALID.bits());
            }
            // else {
            // debug!(
            //     "Valid PTE: bits:{:?} referencing_physaddr: {:?}, flags: {:?}",
            //     pte.bits,
            //     pte.referencing_address().as_usize() as *const usize,
            //     pte.flags()
            // );
            // }

            // next-level node as a slice
            table = unsafe { PageTableNode::from_frame(&pte.referencing_frame()).table() };
            // info!("next page table node at {:?}", table.as_ptr());
        }
        unreachable!()
    }

    pub fn find(&self, va: VirtAddr) -> Option<&'static mut PageTableEntry> {
        let mut table = unsafe { self.root_node.table() };

        for level in (0..=2).rev() {
            let index = va.pte_index(level);
            let pte = table
                .get_mut(index)
                .expect("PageTable::map: invalid entry index");

            if level == 0 {
                return Some(pte);
            }

            if !pte.is_valid() {
                return None;
            }
            // next-level node as a slice
            table = unsafe { PageTableNode::from_frame(&pte.referencing_frame()).table() };
        }
        unreachable!()
    }

    /// The virtual and physical addresses must be valid
    pub fn map_one(&self, va: VirtAddr, pa: PhysAddr, flags: PTEFlags) -> Option<()> {
        let pte = self.find(va)?;
        let flags = flags | PTEFlags::VALID;
        assert!(
            !pte.is_valid(),
            "PageTable::map: overwritting original mapping!"
        );
        *pte = PageTableEntry::new(pa, flags);
        return Some(());
    }

    pub fn map_one_allocate(&mut self, va: VirtAddr, pa: PhysAddr, flags: PTEFlags) {
        // debug!(
        //     "PageTableGuard::map_one_allocate: try mapping {:?} -> {:?}",
        //     va.as_usize() as *const usize,
        //     pa.as_usize() as *const usize
        // );
        let pte = self.find_allocate(va);
        let flags = flags | PTEFlags::VALID;
        assert!(
            !pte.is_valid(),
            "PageTable::map: overwritting original mapping!"
        );
        *pte = PageTableEntry::new(pa, flags);
        assert_eq!(pte.referencing_address(), pa);
        assert_eq!(pte.flags().bits(), flags.bits());
        // debug!(
        //     "0-level PTE: bits:{:?} referencing_physaddr: {:?}, flags: {:?}",
        //     pte.bits,
        //     pte.referencing_address().as_usize() as *const u32,
        //     pte.flags()
        // );
        // debug!(
        //     "PageTableGuard::mep_one_allocate: mapped {:?} -> {:?}",
        //     va.as_usize() as *const usize,
        //     pa.as_usize() as *const usize
        // );
    }

    /// map the given `virt_area` into the page table.
    pub fn map_virt_area_allocate(&mut self, virt_area: &VirtArea) {
        let flags: PTEFlags = virt_area.permissions().into();
        if virt_area.is_identically_mapped {
            let rng = virt_area.virt_frame_range; // Copied
            for v_frame in rng.into_iter() {
                let va = v_frame.get_base_virt_addr();
                let pa = PhysAddr::new(va.as_usize());
                assert_eq!(va.as_usize(), pa.as_usize());
                assert!(va.is_page_aligned());
                assert!(pa.is_page_aligned());
                self.map_one_allocate(va, pa, flags);
            }
        } else {
            for (va, virt_frame_guard) in &virt_area.virt_frames {
                match virt_frame_guard {
                    VirtFrameGuard::ExclusivelyAllocated(phys_frame_guard) => {
                        let pa = phys_frame_guard.inner_ref().get_base_phys_addr();
                        assert!(va.is_page_aligned());
                        assert!(pa.is_page_aligned());
                        self.map_one_allocate(*va, pa, flags);
                    }
                    VirtFrameGuard::CowShared(_phys_frame_guard_arc) => {
                        panic!("kernel does not support copy-on-write at the moment...");
                    }
                    VirtFrameGuard::PhysBorrowed(phys_frame) => {
                        let pa = phys_frame.get_base_phys_addr();
                        assert!(va.is_page_aligned());
                        assert!(pa.is_page_aligned());
                        self.map_one_allocate(*va, pa, flags);
                    }
                }
            }
        }
    }
}

impl PageTableGuard {
    pub fn verify_virt_area_mapping(&self, virt_area: &VirtArea) {
        let flags: PTEFlags = virt_area.permissions().into();
        if virt_area.is_identically_mapped {
            let rng = virt_area.virt_frame_range; // Copied
            for v_frame in rng.into_iter() {
                let va = v_frame.get_base_virt_addr();
                let pa = PhysAddr::new(va.as_usize());
                if let Some(pte) = self.find(va) {
                    assert_eq!(pte.referencing_address(), pa, "address mismatch");
                    assert_eq!(pte.flags(), flags | PTEFlags::VALID, "flag mismatch");
                }
            }
        } else {
            for (va, virt_frame_guard) in &virt_area.virt_frames {
                match virt_frame_guard {
                    VirtFrameGuard::ExclusivelyAllocated(phys_frame_guard) => {
                        let pa = phys_frame_guard.inner_ref().get_base_phys_addr();
                        if let Some(pte) = self.find(*va) {
                            assert_eq!(pte.referencing_address(), pa, "address mismatch");
                            assert_eq!(pte.flags(), flags | PTEFlags::VALID, "flag mismatch");
                        }
                    }
                    VirtFrameGuard::CowShared(_phys_frame_guard_arc) => {
                        panic!("kernel does not support copy-on-write at the moment...");
                    }
                    VirtFrameGuard::PhysBorrowed(phys_frame) => {
                        let pa = phys_frame.get_base_phys_addr();
                        if let Some(pte) = self.find(*va) {
                            assert_eq!(pte.referencing_address(), pa, "address mismatch");
                            assert_eq!(pte.flags(), flags | PTEFlags::VALID, "flag mismatch");
                        }
                    }
                }
            }
        }
    }

    /// lock the page table by making its node frames in the kernel space read-only
    /// so that accidental writing to itwill be caught
    pub fn lock_table(&self) {
        for frame in &self.node_frames {
            let node_pa = frame.get_frame().get_base_phys_addr();
            let pte = self.find(VirtAddr::from_identical(node_pa)).unwrap();
            // clear writable flag to lock the table page
            let flags = pte.flags() & (!PTEFlags::WRITABLE);
            *pte = PageTableEntry::new(pte.referencing_address(), flags)
        }
    }

    /// unlock the page table by making its node frames in the kernel space writable
    pub fn unlock_table(&self) {
        for frame in &self.node_frames {
            let node_pa = frame.get_frame().get_base_phys_addr();
            let pte = self.find(VirtAddr::from_identical(node_pa)).unwrap();
            // clear writable flag to lock the table page
            let flags = pte.flags() | PTEFlags::WRITABLE;
            *pte = PageTableEntry::new(pte.referencing_address(), flags)
        }
    }
}

pub fn test() {
    let pa = PhysAddr::new(12345).align_down();
    let flags = PTEFlags::VALID | PTEFlags::USER;
    let entry = PageTableEntry::new(pa, flags);
    assert_eq!(entry.referencing_address(), pa);
    assert_eq!(entry.flags().bits(), entry.flags().bits());
}
