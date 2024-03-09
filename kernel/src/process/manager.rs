use alloc::{collections::VecDeque, sync::Arc};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{allocator::resource_manager::ResourceManager, process::process::ProcessControlBlock};

use super::process;
lazy_static! {
    pub static ref PROCESS_MANAGER: Mutex<ProcessManager> = Mutex::new(ProcessManager::new());
}

const INTIIAL_MAX_N_PROCS: usize = 128;

pub struct ProcessManager {
    // maps pid to pcb.
    pcb_manager: ResourceManager<ProcessControlBlock>,
    // runnable processes
    ready_queue: VecDeque<Arc<ProcessControlBlock>>,
}

unsafe impl Sync for ProcessManager {}

impl ProcessManager {
    fn new() -> Self {
        Self {
            pcb_manager: ResourceManager::new(INTIIAL_MAX_N_PROCS),
            ready_queue: VecDeque::new(),
        }
    }

    pub fn create_process(&mut self) -> Arc<ProcessControlBlock> {
        let pid = self.pcb_manager.reserve();
        Arc::new(ProcessControlBlock::allocate(pid))
    }

    pub fn pop_one(&mut self) -> Option<Arc<ProcessControlBlock>> {
        self.ready_queue.pop_front()
    }

    pub fn push_one(&mut self, pid: usize) {
        let pcb = self.pcb_manager.get(pid).unwrap().get();
        assert_eq!(pcb.pid, pid);
        self.ready_queue.push_back(pcb.clone());
    }

    pub fn exit_process(&mut self, _pid: usize) {
        // if the process is running

        // if the process is blocked

        // if the process it not running
    }

    pub fn reap_process(&mut self, _pid: usize) {
        // a process cannot reap itself, check it!
    }
}

impl ProcessManager {
    pub fn create_initcode(&mut self) {
        let pid = self.pcb_manager.reserve();
        let pcb = process::make_initcode_uninitialised(pid);
        self.pcb_manager.initialise(pid, pcb.clone());
        self.ready_queue.push_back(pcb);
    }
}

pub fn init() {
    // create the first user-space process init
    // and prepare for its execution environment
    let mut process_manager = PROCESS_MANAGER.lock();
    process_manager.create_initcode();
}
