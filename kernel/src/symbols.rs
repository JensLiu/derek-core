// external asm functions
extern "C" {
    /// `__uservec` in `trampoline.S`
    pub fn __uservec();
    /// `__userret` in `trampoline.S`
    pub fn __userret(satp: usize);
    /// `__kernelvec` in `kernelvec.S`
    pub fn __kernelvec();
    /// `__timervec` in `kernelvec.S`
    pub fn __timervec();
}

/// Maximum supported CPU on machine
/// Note that it is bounded by the kernel boot stack in
/// `linker.ld` and `boot.S`
pub const N_CPUS: usize = 8;

pub const N_PROCS: usize = 128;

/// Scheduler timer interrupt interval
pub const SCHEDULER_INTERVAL: usize = 1_000_000;
