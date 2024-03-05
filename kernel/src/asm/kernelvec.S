# the interrupt handlers in M-mode

#include "macros.S"

.global __kernelvec
.align 4
__kernelvec:
    # since we are not swtiching stack, it is easier to store registers on the kernel stack
    # the ides here is to fake a function call in S-mode

    # as usual, we are storing the context for user registers
    
    # we first make space in its kernel stack
    addi sp, sp, -32*8 # remember that stack grows downwards
    
    .set i, 1
    .rept (31-1)+1
        SAVE_USER_REG %i, i*8(sp)
        .set i, i+1
    .endr

    # then we call the kernel trap handler
    # kerneltrap()
    call kerneltrap # function call preserves the stack pointer

    # we restore registers
    ld ra, 1*8(sp)  # x1
    ld sp, 2*8(sp)  # x2
    ld gp, 3*8(sp)  # x3
    # skip tp: x4 because we may move CPU
    .set i, 5
    .rept (31-5)+1
        LOAD_USER_REG %i, i*8(sp)
        .set i, i+1
    .endr

    addi sp, sp, 32*8   # pop stack

    sret # return to whatever we are doing in the kernel

.global __timervec
.align 4
__timervec:
    # kinit.c has set up the memory that mscratch points to:
    # scratch[0, 1, 2]: register save area
    # scratch[3]: address for CLINT's MTIME register (assume)
    # scratch[4]: address for CLINT's MTIMECMP register (assume)
    # scratch[5]: declared interval between interrupts  (assume)

    # we assume that mscratch is set to point to the `scratch` slice beforehand
    csrrw a0, mscratch, a0 # swap a0 and mscratch, now a0 is the start of the `scratch` slice

    # save a1, a2, a3 because we are using it later in the code
    sd a1, 0*8(a0)
    sd a2, 1*8(a0)
    sd a3, 2*8(a0)

    # schedule the next timer interrupt
    ld a1, 4*8(a0)  # now a1 has the address of the memory-mapped register MTIMECMP
    ld a2, 5*8(a0)  # interval
    ld a3, 0(a1)    # load value in MTIMECMP (mem-mapped)
    add a3, a3, a2  # next_trigger = current_time + interval
    sd a3, 0(a1)    # R[a3] = next_trigger

    # raise a S-mode software interrupt
    # after this handler returns from M-mode
    li a1, 2
    csrw sip, a1    # Supervisor Interrupt Pending

    # restore registers
    ld a1, 0*8(a0)
    ld a2, 1*8(a0)
    ld a3, 2*8(a0)
    csrrw a0, mscratch, a0

    # return from machine mode
    mret