use crate::mm::layout::{CLINT_MTIMECMP_BASE, CLINT_MTIME_BASE};
use crate::{
    arch::hart_id,
    symbols::{__timervec, N_CPUS, SCHEDULER_INTERVAL},
};
use riscv::register::*;

// core local interruptor (CLINT), which contains the timer.
pub const CLINT_BASE: usize = 0x200_0000;

#[allow(non_snake_case)]
pub const fn CLINT_TIMECMP(hart: usize) -> usize {
    CLINT_MTIMECMP_BASE + 8 * hart
}

/// scratch area for timer trap to save information: 64 bytes per core
// static mut TIMER_SCRATCH: [[u64; 8]; NCPUS] = [[0; 8]; NCPUS];
/// this init the timer scratch for each cpu

/// refer to `__timervec` in `src/asm/kernelvec.S`
#[repr(C)]
#[derive(Clone, Copy)]
struct TimerScratch {
    tmp_regs: [usize; 3],
    mtime_addr: usize,
    mtimecmp_addr: usize,
    interval: usize,
}

// We allocate a `TimerScratch` for each CPU
// TODO: don't use static, other options?
static mut TIMER_SCRATCHES: [TimerScratch; N_CPUS] = [TimerScratch {
    tmp_regs: [0; 3],
    mtime_addr: 0,
    mtimecmp_addr: 0,
    interval: 0,
}; N_CPUS];

pub unsafe fn timer_init() {
    let id = hart_id();
    let mtime = CLINT_MTIME_BASE as *mut u64;

    // ask CLITN for timer interrupt
    let interval = SCHEDULER_INTERVAL as u64; // cycles; about 1/10th second in qemu
    let mtimecmp = CLINT_TIMECMP(id) as *mut u64;
    // our timer interrupt will occur at `mtime` + `interval`, where `mtime` is the current time
    mtimecmp.write_volatile(mtime.read_volatile() + interval);

    // prepare information in scratch[] for timervec
    // scratch[0..2]: space for timervec to save registers(3 * size): because timervec uses these registers
    // scratch[3]: adress for CLINT MTIME register
    // scratch[4]: address for CLINT MTIMECMP register
    // scratch[5]: desired interval (in cycles) between timer interrupts
    let scratch = &mut TIMER_SCRATCHES[id];
    mscratch::write(scratch as *const TimerScratch as usize); // mscratch register is only accessable in M-mode
    scratch.mtime_addr = mtime as usize;
    scratch.mtimecmp_addr = mtimecmp as usize;
    scratch.interval = interval as usize;

    // set M-mode trap handler to `__timervec` in `kernelvec.S`
    mtvec::write(__timervec as usize, mtvec::TrapMode::Direct);

    // enable M-mode interrupts
    mstatus::set_mie(); // `mie` (machine interrupt enabled) bit in `mstatus` register

    // enable M-mode timer interrupt
    mie::set_mtimer() // `mtimer` bit in `mie` register
}
