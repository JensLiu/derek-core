# Define a .text.init section.
.section .text.init

.global _start
.global __kernel_stack_start
.global kstart

# Execution starts here.
_start:
	# initialise .bss section with 0's
	la 		a0, __bss_start
	la		a1, __bss_end
	bgeu	a0, a1, 2f
1:
	sd		zero, (a0)
	addi	a0, a0, 8
	bltu	a0, a1, 1b
2:
	# Allocate stack for each hart
	la sp, __kernel_stack_start
	li a0, 0x10000	# 64KB
	csrr a1, mhartid
	addi a1, a1, 1
    mul a0, a0, a1
    add sp, sp, a0
    # jump to kinit in lib.rs
    call kstart
spin:
	# lock up the core if kinit fails
	j spin