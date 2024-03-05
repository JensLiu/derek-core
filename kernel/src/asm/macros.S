# macro for indexing into the corresponsing x-`i` registers
# into the TRAPFRAME pointed by `base`
.altmacro   # otherwise `SAVE_USER_REG %i, sp` won't work, it expands to `%i` not the value
.macro SAVE_USER_REG i, dest
    sd x\i, \dest
.endm

.macro LOAD_USER_REG i, dest
    ld x\i, \dest
.endm