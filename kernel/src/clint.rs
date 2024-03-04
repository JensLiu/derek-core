use riscv::register::*;
use crate::{arch::hart_id, symbols::{timervec, NCPUS, SCHEDULER_INTERVAL}};

// core local interruptor (CLINT), which contains the timer.
pub const CLINT_BASE: usize = 0x200_0000;
pub const CLINT_MTIMECMP_BASE: usize = CLINT_BASE + 0x4000; // mechine-level time compare
pub const CLINT_MTIME_BASE: usize = CLINT_BASE + 0xbff8;

#[allow(non_snake_case)]
pub const fn CLINT_TIMECMP(hart: usize) -> usize { CLINT_MTIMECMP_BASE + 8*hart }

/// scratch area for timer trap to save information: 64 bytes per core
static mut TIMER_SCRATCH: [[u64; 8]; NCPUS] = [[0; 8]; NCPUS];

pub unsafe fn timer_init() {
    let id = hart_id();
    let mtime = CLINT_MTIME_BASE as *mut u64;

    // ask CLITN for timer interrupt
    let interval = SCHEDULER_INTERVAL as u64;   // cycles; about 1/10th second in qemu
    let mtimecmp = CLINT_TIMECMP(id) as *mut u64;
    // our timer interrupt will occur at `mtime` + `interval`,
    //  where `mtime` is the current time
    mtimecmp.write_volatile(mtime.read_volatile() + interval);
    
    // prepare information in scratch[] for timervec
    // scratch[0..2]: space for timervec to save registers(3 * size): because timervec uses these registers
    // scratch[3]: adress for CLINT MTIME register
    // scratch[4]: address for CLINT MTIMECMP register
    // scratch[5]: desired interval (in cycles) between timer interrupts
    let scratch = &mut TIMER_SCRATCH[id];
    mscratch::write(scratch.as_mut_ptr() as usize); // mscratch register is only accessable in M-mode
    scratch[3] = mtime as u64;
    scratch[4] = mtimecmp as u64;
    scratch[5] = interval;

    // set M-mode trap handler to `timervec` in `kernelvec.S`
    mtvec::write(timervec as usize, mtvec::TrapMode::Direct);

    // enable M-mode interrupts
    mstatus::set_mie(); // `mie` (machine interrupt enabled) bit in `mstatus` register

    // enable M-mode timer interrupt
    mie::set_mtimer()   // `mtimer` bit in `mie` register

}