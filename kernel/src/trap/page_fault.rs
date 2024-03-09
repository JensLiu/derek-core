use riscv::register::stval;

use crate::{
    cpu, info,
    mm::{memory::VirtAddr, page_table::PageFlags},
};

pub struct InstructionPageFaultHandler {}

impl InstructionPageFaultHandler {
    pub fn handle() {
        let va = stval::read();
        info!("Instruction Page Fault: accessing {:?}", va as *const usize);
        // let's check if this is mapped as executable
        let pcb = cpu::current_process().unwrap();
        let inner = pcb.inner.read();
        let user_space = inner.get_user_space_ref_or_else_panic();
        match user_space.translate(VirtAddr::new(va)) {
            Some((pa, flags)) => {
                info!(
                    "va: {:?} -> pa: {:?}, flags: {:?}",
                    va as *const usize,
                    pa.as_usize() as *const usize,
                    flags
                );
                if !flags.contains(PageFlags::EXECUTABLE) {
                    panic!("trap::usertrap: it's not even executable, what are you doing???");
                }
                if !flags.contains(PageFlags::USER) {
                    panic!("trap::usertrap: did you rememeber to set the U-bit???");
                }
            }
            None => {
                panic!("NOT MAPPED?");
            }
        }
    }
}
