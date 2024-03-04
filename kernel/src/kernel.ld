OUTPUT_ARCH(riscv)
ENTRY(_start)
KERNEL_OFFSET = 0x80000000;  /* the first location qemu jumps to */

MEMORY
{
  ram : ORIGIN = 0x80000000, LENGTH = 128M
}

/*  We are definling the layout of the kernel's ELF file         */
/*  It will be loaded into the physical memory when qemu starts  */
SECTIONS
{
    . = KERNEL_OFFSET;
    __kernel_binary_start = .;

    __text_start = .;
    .text : {
        /* init code starts at KERNEL_OFFSET */
        *(.text.init)
        /* trampoline */
        . = ALIGN(4K);
        __trampoline_start = .;
        *(.text.trampoline);
        . = ALIGN(4K);
        __trampoline_end = .;
        /* other text sections */
        *(.text .text.*)
    }
    . = ALIGN(4K);
    __text_end = .;
   

    __rodata_start = .;
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    . = ALIGN(4K);
    __rodata_end = .;


    __data_start = .;
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    . = ALIGN(4K);
    __data_end = .;


    __bss_start = .;
    .bss : {
        __kernel_heap_start = .;
        *(.bss.kernel_heap)
        __kernel_heap_end = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }
    . = ALIGN(4K);
    __bss_end = .;
    __kernel_binary_end = .;


    /DISCARD/ : {
        *(.eh_frame)
    }

  __memory_start = ORIGIN(ram);
  __memory_end = ORIGIN(ram) + LENGTH(ram);

  /* We define kernel stack to span 0x80000 B = 512KB */
  /* Since each hart is initialised with 64KB of kernel stack */
  /* a maximun of 8 cores */
  __kernel_stack_start = __bss_end;
  __kernel_stack_end = __kernel_stack_start + 0x80000;
  
  /* the heap is allocated for the rest of the space */
  __heap_start = __kernel_stack_end;
  __heap_end = __memory_end;
  __heap_size = __memory_end - __heap_start;
}