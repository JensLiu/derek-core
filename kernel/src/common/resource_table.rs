use core::borrow::Borrow;

use hashbrown::HashMap;

use alloc::{collections::BTreeSet, string::String, sync::Arc};
use spin::{Mutex, RwLock};

use crate::info;

/// Since the allocation of process id, file discriptor id etc
/// follows the same algorithm
pub struct ResourceTable<T> {
    // resource id -> (resource + ref count) map
    // the rwlock protects the structure of the hashmap, not the
    // integrity of its data
    active_slots: RwLock<HashMap<usize, Option<Arc<T>>>>, // read heavy?

    // available ids
    // the mutex makes sure its capacity and free_ids set are in sync,
    // that the update of capacity is atomic
    free_slots: Mutex<FreeSlotsInner>,
    // debug
    name: String,
}

impl<T> ResourceTable<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            active_slots: RwLock::new(HashMap::new()),
            free_slots: Mutex::new(FreeSlotsInner::new(capacity)),
            name: "Resource".into(),
        }
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.into();
    }

    pub fn reserve_entry(&mut self) -> usize {
        // allocate id
        let id = {
            let mut free_slots = self.free_slots.lock();
            free_slots.allocate_one()
        };

        // this copies `resource` from stack to the heap, expensive
        let mut active_slots = self.active_slots.write();
        match active_slots.insert(id, None) {
            Some(_) => {
                panic!(
                    "{:?}Table::reserve: id collision, id: {:?}",
                    self.name, id
                );
            }
            None => {
                info!("{:?}Table::reserve: reserved id: {:?}", self.name, id);
            }
        };
        id
    }

    pub fn initialise_entry(&self, id: usize, data: Arc<T>) {
        let mut active_slots  = self.active_slots.write();
        let entry = active_slots.get_mut(&id).unwrap();
        *entry = Some(data);
    }

    pub fn get(&self, id: usize) -> Arc<T> {
        let mut active_slots = self.active_slots.write();
        match active_slots
            .get_mut(&id)
            .expect("ResourceManager::get_data_ref_mut: internal error")
        {
            Some(slot) => slot.clone(),
            None => {
                panic!(
                    "{:?}Manager::get_data: uninitialised resource, id: {:?}",
                    self.name, id
                );
            }
        }
    }

    pub fn remove_entry(&mut self, id: usize) {
        let mut active_slots = self.active_slots.write();
        active_slots.remove(&id);
        let mut free_slots = self.free_slots.lock();
        free_slots.return_one(id);
    }
}

struct FreeSlotsInner {
    free_ids: BTreeSet<usize>,
    capacity: usize,
}

impl FreeSlotsInner {
    fn new(capacity: usize) -> Self {
        Self {
            free_ids: (0..capacity).collect(),
            capacity,
        }
    }
    fn allocate_one(&mut self) -> usize {
        if self.free_ids.is_empty() {
            (self.capacity..self.capacity * 2).for_each(|id| {
                self.free_ids.insert(id);
            });
            self.capacity *= 2;
        }
        assert!(!self.free_ids.is_empty());
        self.free_ids.pop_first().unwrap()
    }

    fn return_one(&mut self, id: usize) {
        assert!(self.free_ids.remove(&id));
    }
}