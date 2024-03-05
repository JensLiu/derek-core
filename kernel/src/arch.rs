use crate::mm::layout::CLINT_MTIME_BASE;
use core::{arch::asm, time::Duration};

pub fn hart_id() -> usize {
    let hart_id: usize;
    unsafe {
        asm!("mv {}, tp", out(reg) hart_id);
    }
    hart_id
}

pub fn time() -> Duration {
    let mtime = CLINT_MTIME_BASE as *mut u64;
    let time = unsafe { mtime.read_volatile() };
    Duration::from_nanos(time)
}
