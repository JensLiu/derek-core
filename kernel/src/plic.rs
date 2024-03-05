use core::cell::SyncUnsafeCell;

use crate::{
    arch::hart_id,
    mm::layout::{PLIC_BASE, PLIC_PENDING},
};

pub const PLIC_MENABLE_BASE: usize = PLIC_BASE + 0x2000;
// M-mode enabled
pub const PLIC_SENABLE_BASE: usize = PLIC_BASE + 0x2080;
// S-mode enabled
pub const PLIC_MPRIORITY_BASE: usize = PLIC_BASE + 0x200000;
// M-mode priority
pub const PLIC_SPRIORITY_BASE: usize = PLIC_BASE + 0x201000;
// S-mode priority
pub const PLIC_MCLAIM_BASE: usize = PLIC_BASE + 0x200004;
// M-mode claim
pub const PLIC_SCLAIM_BASE: usize = PLIC_BASE + 0x201004; // S-mode claim

#[inline]
#[allow(non_snake_case)]
pub const fn PLIC_MENABLE(hart: usize) -> usize {
    PLIC_MENABLE_BASE + hart * 0x100
}

#[inline]
#[allow(non_snake_case)]
pub const fn PLIC_SENABLE(hart: usize) -> usize {
    PLIC_SENABLE_BASE + hart * 0x100
}

#[inline]
#[allow(non_snake_case)]
pub const fn PLIC_MPRIORITY(hart: usize) -> usize {
    PLIC_MPRIORITY_BASE + hart * 0x2000
}

#[inline]
#[allow(non_snake_case)]
pub const fn PLIC_SPRIORITY(hart: usize) -> usize {
    PLIC_SPRIORITY_BASE + hart * 0x2000
}

#[inline]
#[allow(non_snake_case)]
pub const fn PLIC_MCLAIM(hart: usize) -> usize {
    PLIC_MCLAIM_BASE + hart * 0x2000
}

#[inline]
#[allow(non_snake_case)]
pub const fn PLIC_SCLAIM(hart: usize) -> usize {
    PLIC_SCLAIM_BASE + hart * 0x2000
}

pub const URT0_IRQ: u32 = 10;
pub const VIRTIO0_IRQ: u32 = 1;

pub struct Plic {}

impl Plic {
    pub fn new() -> Self {
        Plic {}
    }

    /// retreve the next interrupt id available in S-mode.
    pub fn next(&self) -> Option<u32> {
        // claim register holds the enabled highest-privliged interrupt
        let claim_reg = PLIC_SCLAIM(hart_id()) as *const u32;
        let int_id = unsafe { claim_reg.read_volatile() };
        if int_id == 0 {
            None // 0 means no interrupt pending
        } else {
            Some(int_id)
        }
    }

    /// complete handling the pending interrupt
    /// by the id got from `next`
    pub fn complete(&self, id: u32) {
        // NOTE: the memory mapped register can distinguish between read and write operations.
        //  read -> claims the interrupt
        //  write -> finishes the interrupt
        let complete_reg = PLIC_SCLAIM(hart_id()) as *mut u32;
        unsafe { complete_reg.write_volatile(id) };
    }

    /// set the priority of the given interrupt id
    /// must be [0..7]
    pub fn set_priority(&self, id: u32, prio: u8) {
        // PLIC_SPRIORITY_BASE + hart_id * 0x2000
        let prio_reg = PLIC_SPRIORITY(hart_id()) as *mut u32;
        // PLIC_SPRIORITY + id * 4
        let int_prio_bit = unsafe { prio_reg.add(id as usize) };

        // write the priority. priority must be [0..7], `& 7` makes sure of it.
        let actual_prio = prio as u32 & 7;
        unsafe { int_prio_bit.write_volatile(actual_prio) }
    }

    /// set the global threshold. must be [0..7]
    /// PLIC will mask off all interrupts <= the threshold
    /// by setting to 7, we mask ALL interrupts
    /// by setting to 0, we allow ALL interrupts
    pub fn set_threshold(&self, tsh: u8) {
        let actual_tsh = tsh as u32 & 7;
        let tsh_reg = PLIC_SPRIORITY(hart_id()) as *mut u32;
        unsafe { tsh_reg.write_volatile(actual_tsh) }
    }

    pub fn enable(&self, id: u32) {
        let enables = PLIC_SENABLE(hart_id()) as *mut u32;
        // NOTE: the plic_int_enable register is bitset mapped.
        //  thus each bit [0..21] represents the stauts of interrupt
        let actual_id = 1 << id; // calculate the id bit
        unsafe {
            enables.write_volatile(enables.read_volatile() | actual_id);
        }
    }

    pub fn is_pending(&self, id: u32) -> bool {
        let pending = PLIC_PENDING as *const u32;
        let int_pending_bit = 1 << id;
        let pending_bits = unsafe { pending.read_volatile() };
        pending_bits & int_pending_bit != 0
    }

    /// enable interrupt by setting its priority to non-zero
    pub unsafe fn init(&self, id: u32) {
        let enables = PLIC_BASE as *mut u32;
        enables.add(id as usize).write_volatile(1); // write non-zero to enable
    }
}

// Driver instance
lazy_static::lazy_static! {
    pub static ref PLIC: SyncUnsafeCell<Plic> = SyncUnsafeCell::new(Plic::new());
}

/// init once
pub fn init() {
    unsafe {
        let plic = &mut *PLIC.get();
        plic.init(URT0_IRQ);
        plic.init(VIRTIO0_IRQ);
    }
}

// for core specific initialisation
pub fn hart_init() {
    unsafe {
        let plic = &mut *PLIC.get();
        plic.enable(URT0_IRQ);
        plic.enable(VIRTIO0_IRQ);
        plic.set_threshold(0);
        plic.set_priority(URT0_IRQ, 1);
        plic.set_priority(VIRTIO0_IRQ, 1);
    }
}
