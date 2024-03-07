pub mod frame_allocator;
pub mod heap_allocator;
pub mod identifier_allocator;

pub fn init() {
    // we should first init the heap allocator
    // because frame allocator uses Rust containers
    heap_allocator::init();
    // heap_allocator::heap_test();
    frame_allocator::init();
}
