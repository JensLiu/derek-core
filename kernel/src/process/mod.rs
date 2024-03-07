use crate::{arch::hart_id, cpu::CPUS};

use self::manager::PROCESS_MANAGER;

pub mod context;
pub mod manager;
pub mod process;

pub fn init() {
    // init the process manager and create the first user-space process
    manager::init();

    // now let's fake that a scheduler has chosen `init` to run it on the core-0
    assert_eq!(hart_id(), 0);
    CPUS[0]
        .write()
        .set_executing_process(PROCESS_MANAGER.lock().pop_one().unwrap());
}

pub fn schedule() {}
