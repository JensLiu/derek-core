use alloc::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::process::process::ProcessControlBlock;

use super::process;
lazy_static! {
    pub static ref PROCESS_MANAGER: Mutex<ProcessManager> = Mutex::new(ProcessManager::new());
}

pub struct ProcessManager {
    // maps pid to pcb.
    pcb_map: BTreeMap<usize, Arc<ProcessControlBlock>>,
    // runnable processes
    ready_queue: VecDeque<Arc<ProcessControlBlock>>,
}

unsafe impl Sync for ProcessManager {}

impl ProcessManager {
    fn new() -> Self {
        Self {
            pcb_map: BTreeMap::new(),
            ready_queue: VecDeque::new(),
        }
    }

    pub fn create_process(&mut self) -> Arc<ProcessControlBlock> {
        let pcb = Arc::new(ProcessControlBlock::allocate());
        match self.pcb_map.try_insert(pcb.get_pid(), pcb.clone()) {
            Ok(_) => pcb.clone(),
            Err(_) => panic!("ProcessManager::create_process: PID conflict"),
        }
    }

    pub fn pop_one(&mut self) -> Option<Arc<ProcessControlBlock>> {
        self.ready_queue.pop_front()
    }

    pub fn push_one(&mut self, pid: usize) {
        let pcb = self
            .pcb_map
            .get(&pid)
            .expect("ProcessManager::push_one: invalid PID");

        assert_eq!(pcb.get_pid(), pid);
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

pub fn init() {
    // create the first user-space process init
    // and prepare for its execution environment

    let pcb = process::make_init();

    let mut guard = PROCESS_MANAGER.lock();
    guard.pcb_map.insert(pcb.get_pid(), pcb.clone());
    guard.ready_queue.push_front(pcb.clone());
}
