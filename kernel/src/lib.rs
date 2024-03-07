#![no_std] // do not use the rust standard library
#![feature(panic_info_message)]
#![feature(format_args_nl)]
#![feature(sync_unsafe_cell)]
#![feature(alloc_error_handler)]
#![feature(custom_test_frameworks)]
#![feature(map_try_insert)]

#[macro_use] // allows macros like `vec`
extern crate alloc;

pub mod allocator;
pub mod arch;
pub mod clint;
pub mod cpu;
pub mod fs;
pub mod mm;
pub mod plic;
pub mod print;
pub mod process;
pub mod start;
pub mod symbols;
pub mod trap;
pub mod uart;

#[no_mangle]
extern "C" fn eh_personality() {}

/// Panic handler
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // panic_println!("hart {} aborting: ", arch::hart_id());
    if let Some(p) = info.location() {
        panic_println!(
            "line {}, file {}: {}",
            p.line(),
            p.file(),
            info.message().unwrap()
        );
    } else {
        panic_println!("no information available.");
    }
    abort();
}

/// Abort function
#[no_mangle]
extern "C" fn abort() -> ! {
    // arch::wait_forever();
    use core::arch::asm;
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}
