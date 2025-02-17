use core::alloc::{GlobalAlloc, Layout};
use crate::memory::VirtualAddress;

struct FreedBlock<'a> {
    curr_size: usize,
    next_freed_block: Option<&'a mut FreedBlock<'a>>,
}

pub struct SimplePageAllocator<'a> {
    heap_start: VirtualAddress,
    heap_end: VirtualAddress,

    next_block: VirtualAddress,
    freed_blocks: Option<&'a mut FreedBlock<'a>>,
}

// TODO: get the real start and end addrs
#[global_allocator]
static SIMPLE_PAGE_ALLOCATOR: SimplePageAllocator = SimplePageAllocator {
    heap_start: 0,
    heap_end: 0,
    next_block: 0,
    freed_blocks: None,
};

unsafe impl<'a> GlobalAlloc for SimplePageAllocator<'a> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        todo!()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        todo!()
    }
}
