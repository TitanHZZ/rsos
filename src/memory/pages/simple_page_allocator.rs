use core::{alloc::{GlobalAlloc, Layout}, ptr::NonNull};
use crate::memory::VirtualAddress;
use spin::Mutex;

struct FreedBlock {
    curr_size: usize,
    next_freed_block: Option<NonNull<FreedBlock>>
}

pub struct SimplePageAllocatorInner {
    heap_start: VirtualAddress,
    heap_end: VirtualAddress,

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
    heap_end    : 0x0,
    next_block  : 0x0,
    freed_blocks: None,
}));

// impl Deref for SimplePageAllocator {
//     type Target = Mutex<SimplePageAllocatorInner>;
//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }
// impl DerefMut for SimplePageAllocator {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.0
//     }
// }

impl SimplePageAllocator {
    /*
     * Safety: init() can only be called once or the allocator might get into an inconsistent state.
     */
    pub unsafe fn init(&self, heap_start: VirtualAddress, heap_end: VirtualAddress) {
        let page_allocator = &mut *self.0.lock();
        page_allocator.heap_start = heap_start;
        page_allocator.heap_end = heap_end;
        page_allocator.next_block = heap_start;
    }
}

unsafe impl GlobalAlloc for SimplePageAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        todo!()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        todo!()
    }
}
