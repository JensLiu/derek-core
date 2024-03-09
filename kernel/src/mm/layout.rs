// we extract offsets from the linker
macro_rules! linker_symbols(
    ($($name:ident),*) => {
        $(
        #[inline]
        pub fn $name() -> usize {
            extern "C" {
                // static $name: usize;
                static $name: ();    // this allows us to break rust rules, it's dangerous
            }
            unsafe { &$name as *const _ as _ }
        }
        )*
    }
);

// pay close attention to the `heap` in the physical memory and the kernel heap:
// KERNEL_HEAP is a contiguous region in the .bss section of the kernel image that stores the kernel data sturctures
// HEAP referred here is the region (__kernel_end, PHYS_TOP]

// NOTE: Any statics that may have overlapping addresses should be `extern`
//  `static KERNEL_HEAP_SPACE` in `src/allocator/heap_allocator.rs` should NEVER share
//  the address with `__kernel_heap_start` symbol!! That's it's invalid
//  and causes `assert_eq(addr_of!(KERNEL_HEAP_SPACE) as usize, __kernel_heap_start() as usize)`
//  to be invalid and being optimised out by the optimiser!!!!!!!
linker_symbols!(
    __heap_size,
    __heap_end,
    __heap_start,
    // __kernel_heap_end,
    // __kernel_heap_start,
    __kernel_stack_end,
    __kernel_stack_start,
    __kernel_binary_end,
    __bss_end,
    __bss_start,
    __data_end,
    __data_start,
    __rodata_end,
    __rodata_start,
    __text_end,
    __trampoline_end,
    __trampoline_start,
    __text_start,
    __kernel_binary_start
);

// one beyond the highest possible virtual address.
// MAXVA is actually one bit less than the max allowed by
// Sv39, to avoid having to sign-extend virtual addresses
// that have the high bit set.
pub const MAX_VA: usize = 1 << (9 + 9 + 9 + 12 - 1);

// ------------------------------- kernel space layout -------------------------------
pub const TRAMPOLINE_BASE_VA: usize = MAX_VA - PAGE_SIZE;

// 4KB per page
pub const PAGE_ORDER: usize = 12;
// pub const PAGE_SIZE: usize = 1 << PAGE_ORDER;   // 4KB
pub const PAGE_SIZE: usize = 4096; // 4KB

// defined in `kernel.ld`
pub const KERNEL_BASE: usize = 0x8000_0000;
pub const PHYS_TOP: usize = KERNEL_BASE + 128 * 1024 * 1024; // 128 MB

// heap for kernel data structures
// It is allocated statically and are placed in
// .bss sections (it is an uninitialised array)
pub const KERNEL_HEAP_SIZE: usize = 1 * 1024 * 1024; // 1MB

// proc's kernel stack
// each process has its own kernel stack
// They are allocated by the `FRAME_ALLOCATOR`
// Their RAII managing instance are allocated in the KERNEL_HEAP by the `KERNEL_HEAP_ALLOCATOR`
pub const KERNEL_STACK_SIZE: usize = PAGE_SIZE * 2;

// proc's user stack
// each process has its own user stack
// They are allocated by the `FRAME_ALLOCATOR`
pub const USER_STACK_SIZE: usize = PAGE_SIZE * 2;

// memory mapped registers
// qemu puts UART registers here in physical memory.
pub const UART_BASE: usize = 0x1000_0000;
pub const UART0: usize = UART_BASE;
pub const UART_SIZE: usize = PAGE_SIZE;

// virtio mmio interface
pub const VIRTIO_BASE: usize = 0x1000_1000;
pub const VIRTIO0: usize = VIRTIO_BASE;
pub const VIRTIO_SIZE: usize = PAGE_SIZE;

// core local interruptor (CLINT), which contains the timer.
pub const CLINT_BASE: usize = 0x200_0000;
pub const CLINT_MTIMECMP_BASE: usize = CLINT_BASE + 0x4000; // mechine-level time compare
pub const CLINT_MTIME_BASE: usize = CLINT_BASE + 0xbff8;
pub const CLINT_SIZE: usize = 0x1_0000;

// qemu puts platform-level interrupt controller (PLIC) here.
pub const PLIC_BASE: usize = 0x0c000000;
pub const PLIC_PRIORITY: usize = PLIC_BASE + 0x0;
pub const PLIC_PENDING: usize = PLIC_BASE + 0x1000;
pub const PLIC_SIZE: usize = 0x40_0000;

// ------------------------------- user space layout -------------------------------
// they share the same trampoline mapping
pub const TRAPFRAME_BASE_USER_VA: usize = TRAMPOLINE_BASE_VA - 2 * PAGE_SIZE;
pub const TRAPFRAME_SIZE: usize = PAGE_SIZE; // even though it cannot span that much
pub const TEXT_BASE_USER_VA: usize = 0x1_0000;
