// we extract offsets from the linker
macro_rules! linker_symbols(
    ($($name:ident),*) => {
        $(
        #[inline]
        pub fn $name() -> usize {
            extern "C" {
                // TODO: UnsafeCell?
                static $name: u8;
            }
            unsafe { &$name as *const u8 as usize }
        }
        )*
    }
);

// pay close attention to the `heap` in the physical memory and the kernel heap:
// KERNEL_HEAP is a contiguous region in the .bss section of the kernel image that stores the kernel data sturctures
// HEAP refered here is the region (__kernel_end, PHYS_TOP]
linker_symbols!(
    __heap_size,
    __heap_end,
    __heap_start,
    __kernel_heap_start,
    __kernel_heap_end,
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
pub const TRAMPOLINE_VA: usize = MAX_VA - PAGE_SIZE;
pub const TRAPFRAME_USER_VA: usize = 1;

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