.section .text.trampoline
.global __uservec
.global __userret

#include "macros.S"

# by now, RISC-V has already done its trap handling routine:
# FOR NONE DEVICE INTERRUPT  
#   - disable interrupt: clear sstatus.SIE (Supervisor Interrupt Enabled bit)
#   - save pc in sepc
#   - save current mode (U/S) in sstatus.SPP (Supervisor Previous Privilege bit)
#   - set scause to reflect the trap's cause
#   - set mode to supervisor
#   - copy stvec to pc (which should the trampoline code)
#
# Note that: We are still using the stack and page table from the process's user mode.
# Now we need to save the execution context and switch to its kernel thread

# Debug: make sure `TrapContext` in src/process/context.rs is exactly the same as the following defination
# pub struct TrapContext {
#     pub user_regs: [usize; 32],   // 0-31 Byte: general purpose registers
#     pub kernel_satp: usize,       // 32 Byte: kernel page table. Supervisor Address & Translation Protection Register
#     pub kernel_sp: usize,         // 33 Byte: kernel stack. Stack Pointer Register
#     pub kernel_hardid: usize,     // 34 Byte: kernel hartid (in tp)
#     pub sepc: usize,              // 35 Byte: Return address from the kernel space to the user space
#     pub kernel_trap: usize,        // 36 Byte: entry point of the handler in the kernel space
# }

__uservec:
    # We've stored the virtual address of trapframe inside the
    # sscratch register beforehand.
    # This instruction does a swap between values in t6 (x31) and sscratch registers
    csrrw t6, sscratch, t6

    # now t6 points to the trapframe

    # save x1 to x30, skipping t6: x31
    # Note that user-programmes do not use the tp register,
    # we could technically skip it but it would result in exception for tp: x4
    .set i, 1
    .rept (30-1)+1    # save x1 to x30 (repeat 30 times)
        SAVE_USER_REG %i, i*8(t6)
        .set i, i+1
    .endr



    # now since t0-t2 (x5-x7) are saved, we can use them
    csrr t0, sscratch   # now t0 holds the original value of t6
    sd t0, 31*8(t6)     # save the original value of t6
    csrw sscratch, t6   # save/restore TRAPFRAME to sscratch

    # load kernel page table to t0 for later use (reused t0)
    # (we are not done with the TRAPFRAME yet, cannot swtich page table now)
    ld t0, 32*8(t6)

    # load kernel stack pointer
    # it is invalid now since we haven't switched to the kernel pagetable
    ld sp, 33*8(t6)

    # save kernel hartid (it is always stored in tp, see initialisation)
    sd tp, 34*8(t6)

    # save the return address when sret
    csrr t1, sepc
    sd t1, 35*8(t6)

    # load `handler_entry` to t1 (reuse t1)
    ld t1, 36*8(sp)

    # -------------------- switch page table begin ---------------------------
    
    # wait for any previous memory operations to complete, so that
    # they use the user page table.
    sfence.vma zero, zero

    # install the kernel page table.
    csrw satp, t0

    # flush now-stale user entries from the TLB.
    sfence.vma zero, zero

    # -------------------- switch page table end ---------------------------

    # jump to the trap handler, which does not return
    jr t1

.align 4
__userret:
    # __userret(satp)
    # It is involked by passing its user-space page table in satp format
    # The first argument is passed to a0

    # -------------------- switch page table begin ---------------------------
    sfence.vma zero, zero
    csrw satp, a0
    sfence.vma zero, zero
    # -------------------- switch page table end ---------------------------

    # we are still in supervisor mode, so we can use sscratch
    csrr a0, sscratch   # note that we restored sscratch to TRAPFRAME in usertrap
    
    # restore x2-x31, which includes sp: x2, tp: x4
    # Note that a process may be scheduled to another hart.
    # the old tp value stored in TrapContext::x_regs[tp] may be wrong, if not modified.
    # the caller of `userret` should update it to the current hartid of the CPU.
    mv t6, a0   # use t6: x31 as our base pointer to the TRAPFRAME (a0 is x10!)
    .set i, 2
    .rept (30-2)+1
        LOAD_USER_REG %i, i*8(t6)
        .set i, i+1
    .endr

    # we are not using a0 as the base pointer to the TRAPFRAME,
    # now it's safe to restore a6
    ld t6, 31*8(t6)

    # since we did not modify sepc (Supervisor Exception Programme Counter)
    # sret will jump to this location
    sret
