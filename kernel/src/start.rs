use crate::arch::hart_id;
use crate::{clint, info, mm, plic};
use crate::{println, uart};
use core::arch::asm;
use riscv::asm::wfi;
use riscv::register::*;

// external
const SIE_SEIE: usize = 1 << 9;
// timer
const SIE_STIE: usize = 1 << 5;
// software
const SIE_SSIE: usize = 1 << 1;

#[no_mangle]
unsafe extern "C" fn kstart() {
    // we are now in machine mode

    // `mret` to supervisor mode
    mstatus::set_mpp(mstatus::MPP::Supervisor); // M-mode previous privilege bits

    // `mret` to `kmain` function
    mepc::write(kmain as usize); // M-mode exception program counter register

    // disable paging for now
    asm!("csrw satp, zero"); // S-mode address translation and protection register

    // designate all interrupts and exceptions to the supervisor mode
    asm!("li t0, 0xffff"); // all-ones: all interruptions
    asm!("csrw medeleg, t0"); // M-mode exception deligate
    asm!("csrw mideleg, t0"); // M-mode interrupt deligate
                              // allow external, timer and software interruption in M-mode

    let sie: usize;
    asm!("csrr {}, sie", out(reg) sie);
    asm!("csrw sie, {}", in(reg) sie | SIE_SEIE | SIE_STIE | SIE_SSIE);

    // physical memory protection: give S-mode access to all the physical memory
    // TODO

    // save cpuid to tp register
    asm!("csrr a1, mhartid");
    asm!("mv tp, a1");

    // timer interrupt init
    clint::timer_init();

    // return to `kmain` in S-Mode
    asm!("mret");
}

/// Controls weather other harts may start boot procedure
/// (They should wait for hear-0 to finish initialising)
static mut HART0_STARTED: bool = false;

#[no_mangle]
extern "C" fn kmain() {
    // we are now in supervisor mode
    if hart_id() == 0 {
        uart::init(); // init uart for printing
        info!("booting derek-core on hart {}...", hart_id());
        info!("UART initialised");

        mm::init(); // init allocators and kernel page table
        mm::hart_init(); // turn on paging

        // process table init
        // trap vectors init (counter for timer)
        // install kernel trap vector

        plic::init(); // set up interrupt controller
        plic::hart_init(); // ask for PLIC for device interrupts
        info!("PLIC initialised");

        unsafe {
            HART0_STARTED = true;
        }
    } else {
        // wait until hart-0 finishes
        loop {
            if unsafe { HART0_STARTED } {
                break;
            }
        }
        info!("hart {} booting...", hart_id());

        plic::hart_init(); // turn
        mm::hart_init(); // turn on pagning
    }

    // for now, just do nothing...
    loop {
        wfi(); // RISC-V intstruction: wait for interrupt
        info!("interrupt");
    }
}
