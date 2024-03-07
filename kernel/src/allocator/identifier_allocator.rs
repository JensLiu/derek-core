use alloc::collections::BTreeSet;

/// Since the allocation of process id, file discriptor id etc
/// follows the same algorithm, we extract such allocator of identifier
/// for resources into a struct
pub struct IdentifierAllocator {
    tracker: BTreeSet<usize>,
}

impl IdentifierAllocator {
    pub fn new(capacity: usize) -> Self {
        Self {
            tracker: (0..capacity).into_iter().collect::<BTreeSet<usize>>(),
        }
    }

    pub fn allocate(&mut self) -> usize {
        match self.tracker.pop_first() {
            Some(id) => id,
            None => {
                panic!("IdentifierAllocator::allocate: no more ids available");
            }
        }
    }

    pub fn deallocate(&mut self, id: usize) {
        if !self.tracker.insert(id) {
            panic!("IdentifierAllocator::deallocate: deallocate twice!");
        }
    }
}
