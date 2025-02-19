use core::{alloc::{GlobalAlloc, Layout}, ptr::{null_mut, NonNull}};
use crate::memory::VirtualAddress;
use spin::Mutex;

struct FreedBlock {
    curr_size: usize,
    next_freed_block: Option<NonNull<FreedBlock>>
}

pub struct SimplePageAllocatorInner {
    heap_start: VirtualAddress,
    heap_size: usize,

    next_block: VirtualAddress,
    freed_blocks: Option<NonNull<FreedBlock>>,
}

unsafe impl Send for SimplePageAllocatorInner {}

pub struct SimplePageAllocator(Mutex<SimplePageAllocatorInner>);

/*
 * This just sets some defualt values that will get initialized in init().
 */
#[global_allocator]
pub static SIMPLE_PAGE_ALLOCATOR: SimplePageAllocator = SimplePageAllocator (Mutex::new(SimplePageAllocatorInner {
    heap_start  : 0x0,
    heap_size   : 0,
    next_block  : 0x0,
    freed_blocks: None,
}));

impl SimplePageAllocator {
    /*
     * Safety: init() can only be called once or the allocator might get into an inconsistent state.
     */
    pub unsafe fn init(&self, heap_start: VirtualAddress, heap_size: usize) {
        let page_allocator = &mut *self.0.lock();
        page_allocator.heap_start = heap_start;
        page_allocator.heap_size  = heap_size;
        page_allocator.next_block = heap_start;
    }
}

unsafe impl GlobalAlloc for SimplePageAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let allocator = &mut *self.0.lock();

        // try to find a free block first
        if let Some(freed_blocks) = allocator.freed_blocks {
            unimplemented!()
        }

        if allocator.next_block % layout.align() != 0 {
            unimplemented!()
        }

        if allocator.next_block + layout.size() <= allocator.heap_start + allocator.heap_size {
            allocator.next_block += layout.size();
            return (allocator.next_block - layout.size()) as *mut u8;
        }

        null_mut()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unimplemented!()
    }
}
