use crate::{cpu, info};
use primitive_enum::primitive_enum;

primitive_enum! {
Syscall usize;
    SysFork = 0,
    SysExit = 1,
    SysWait = 2,
    SysPipe = 3,
    SysRead = 4,
    SysWrite = 5,
    SysClose = 6,
    SysKill = 7,
    SysExec = 8,
    SysOpen = 9,
    SysMknod = 10,
    SysUnlink = 11,
    SysFstat = 12,
    SysLink = 13,
    SysMkdir = 14,
    SysChdir = 15,
    SysDup = 16,
    SysGetpid = 17,
    SysSbrk = 18,
    SysSleep = 19,
    SysUptime = 20,
}
pub struct SystemCallHandler {}

impl SystemCallHandler {
    /// It requires inner read lock!

    pub fn handle() {
        let pcb = cpu::current_process().unwrap();
        let mut inner = pcb.inner.write();
        inner.write_trap_context(|ctx| {
            // we move the return address to the next instruction
            // otherwies it's an infinite loop
            ctx.incr_user_space_pc(4);
        });

        let ctx = inner.get_context_ref_or_else_panic();
        let call = ctx.get_syscall().unwrap();
        info!("SYSCALL: {:?}", call);
    }
}
