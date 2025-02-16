use core::alloc::{GlobalAlloc, Layout};
use crate::memory::VirtualAddress;

pub struct SimplePageAllocator {
    // heap_start: VirtualAddress,
    // heap_end: VirtualAddress,
}

#[global_allocator]
static SIMPLE_PAGE_ALLOCATOR: SimplePageAllocator = SimplePageAllocator{};

unsafe impl GlobalAlloc for SimplePageAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        todo!()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        todo!()
    }
}
