use lazy_static::lazy_static;

lazy_static! {
    pub static ref CPUS: Vec<RwLock<PercpuBlock>> = {
        (0..N_CPUS)
            .into_iter()
            .map(|hartid| {
                RwLock::new(PercpuBlock {
                    hartid,
                    running_process: None,
                })
            })
            .collect()
    };
}

use alloc::{sync::Arc, vec::Vec};
use spin::rwlock::RwLock;

use crate::{arch::hart_id, process::process::ProcessControlBlock, symbols::N_CPUS};

#[derive(Debug)]
pub struct PercpuBlock {
    hartid: usize,
    running_process: Option<Arc<ProcessControlBlock>>,
}

impl PercpuBlock {
    pub fn set_executing_process(&mut self, pcb: Arc<ProcessControlBlock>) {
        assert!(self.running_process.is_none());
        self.running_process = Some(pcb);
    }

    pub fn take_executing_process(&mut self) -> Option<Arc<ProcessControlBlock>> {
        self.running_process.take()
    }

    pub fn hartid(&self) -> usize {
        self.hartid
    }
}

/// returns the current process of the calling CPU
pub fn current_process() -> Option<Arc<ProcessControlBlock>> {
    let hartid = hart_id();
    let cpu = CPUS[hartid].read();
    assert_eq!(cpu.hartid, hartid);
    let pcb = cpu.running_process.as_ref()?;
    Some(pcb.clone())
}
