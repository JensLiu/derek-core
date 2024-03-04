# the timer interrupt handler in M-mode
.global timervec
.align 4
timervec:
    # kinit.c has set up the memory that mscratch points to:
    # scratch[0, 8, 16]: register save area
    # scratch[24]: address for CLINT's MTIME register
    # scratch[32]: address for CLINT's MTIMECMP register
    # scratch[40]: declared interval between interrupts

    csrrw a0, mscratch, a0 # swap a0 and mscratch, now a0 is the start of `scratch`
    # save a1, a2, a3 because we are using it later in the code
    sd a1, 0(a0)
    sd a2, 8(a0)
    sd a3, 16(a0)

    # schedule the next timer interrupt
    ld a1, 32(a0)   # MTIMECMP
    ld a2, 40(a0)   # interval
    ld a3, 0(a1)    # load value in MTIMECMP (mem-mapped)
    add a3, a3, a2  # next_trigger = current_time + interval
    sd a3, 0(a1)    # R[a3] = next_trigger

    # raise a S-mode software interrupt
    li a1, 2
    csrw sip, a1

    # restore registers
    ld a1, 0(a0)
    ld a2, 8(a0)
    ld a3, 16(a0)
    csrrw a0, mscratch, a0

    # return from machine mode
    mret