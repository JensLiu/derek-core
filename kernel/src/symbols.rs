// external asm functions
extern "C" {
    /// `uservec` in `trampoline.S`
    pub fn uservec();
    /// `userret` in `trampoline.S`
    pub fn userret();
    /// `kernelvec` in `kernelvec.S`
    pub fn kernelvec();
    /// `timervec` in `kernelvec.S`
    pub fn timervec();
}

/// Maximum supported CPU on machine
/// Note that it is bounded by the kernel boot stack in
/// `linker.ld` and `boot.S`
pub const NCPUS : usize = 8;

/// Scheduler timer interrupt interval
pub const SCHEDULER_INTERVAL: usize = 1_000_000;