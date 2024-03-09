use hashbrown::HashMap;

use alloc::{collections::BTreeSet, sync::Arc};
use spin::{Mutex, RwLock};

use crate::info;

/// Since the allocation of process id, file discriptor id etc
/// follows the same algorithm
pub struct ResourceManager<T> {
    // resource id -> (resource + ref count) map
    // the rwlock protects the structure of the hashmap, not the
    // integrity of its data
    active_resources: RwLock<HashMap<usize, Option<(Arc<T>, usize)>>>, // read heavy?
    // available ids
    // the mutex makes sure its capacity and free_ids set are in sync,
    // that the update of capacity is atomic
    free_ids: Mutex<FreeIdsInner>,
}

impl<T> ResourceManager<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            active_resources: RwLock::new(HashMap::new()),
            free_ids: Mutex::new(FreeIdsInner::new(capacity)),
        }
    }

    pub fn reserve(&mut self) -> usize {
        // allocate id
        let id = {
            let mut free_ids = self.free_ids.lock();
            free_ids.allocate_one()
        };

        // this copies `resource` from stack to the heap, expensive
        let mut active_resources = self.active_resources.write();
        match active_resources.insert(id, None) {
            Some(_) => {
                panic!("ResourceManager::reserve: id collision, id: {:?}", id);
            }
            None => {
                info!("ProcessManager::reserve: reserved id: {:?}", id);
            }
        };
        id
    }

    pub fn initialise(&self, id: usize, data: Arc<T>) {
        let mut active_resources = self.active_resources.write();
        // active_resources
        //     .entry_ref(&id)
        //     .and_replace_entry_with(|_, _| {
        //         let entry = Some((data, 1));
        //         Some(entry)
        //     });'
        let entry = active_resources.get_mut(&id).unwrap();
        *entry = Some((data, 1));
    }

    /// public interface:
    /// Get a `ResourceGuard` and increases its reference count by 1
    /// Semantics: The user gets reference to the resource
    pub fn get(&self, id: usize) -> Option<ResourceGuard<T>> {
        let mut active_resources = self.active_resources.write();
        match active_resources.get_mut(&id)? {
            Some((_, ref_cnt)) => {
                *ref_cnt += 1;
                info!(
                    "ResourceManager::get: id: {:?}, ref_cnt: {:?}",
                    id, *ref_cnt
                );
            }
            None => {
                panic!("ResourceManager::get: uninitialised resource, id: {:?}", id);
            }
        }

        drop(active_resources);

        Some(ResourceGuard { id, manager: self })
    }

    /// this should only be called by the `ResourceGuard` when it is dropped
    fn dec_ref_cnt(&self, id: usize) {
        let mut active_resources = self.active_resources.write();
        let (_, ref_cnt) = active_resources.get_mut(&id).unwrap().as_mut().unwrap();
        *ref_cnt -= 1;
        info!(
            "ResourceManager::dec_ref_cnt: id: {:?}, ref_cnt: {:?}",
            id, *ref_cnt
        );
        if *ref_cnt == 0 {
            // deallocation process
            active_resources.remove(&id).unwrap();

            // the resource should be dropped at this point
            let mut free_ids = self.free_ids.lock();
            free_ids.return_one(id);
            info!("ResourceManager::dec_ref_cnt: id: {:?} deallocated", id);
        }
    }

    /// this should only be called by the `ResourceGuard`
    /// Note that it does NOT increase the ref count, just as a retriever to the
    /// data inside our manager "table"
    fn get_data(&self, id: usize) -> Arc<T> {
        let mut active_resources = self.active_resources.write();
        match active_resources
            .get_mut(&id)
            .expect("ResourceManager::get_data_ref_mut: internal error")
        {
            Some((resource, _)) => resource.clone(),
            None => {
                panic!(
                    "ResourceManager::get_data: uninitialised resource, id: {:?}",
                    id
                );
            }
        }
    }
}

pub struct ResourceGuard<'a, T> {
    id: usize,
    manager: &'a ResourceManager<T>,
}

impl<'a, T> ResourceGuard<'a, T> {
    pub fn id(&self) -> usize {
        self.id
    }

    pub fn get(&self) -> Arc<T> {
        self.manager.get_data(self.id)
    }
}

impl<'a, T> Drop for ResourceGuard<'a, T> {
    fn drop(&mut self) {
        info!(
            "ResourceGuard::drop: resource guard with id {:?} dropped",
            self.id
        );
        self.manager.dec_ref_cnt(self.id);
    }
}

struct FreeIdsInner {
    free_ids: BTreeSet<usize>,
    capacity: usize,
}

impl FreeIdsInner {
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
