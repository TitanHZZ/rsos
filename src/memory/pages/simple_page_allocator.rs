use crate::memory::{frames::{simple_frame_allocator::SimpleFrameAllocator, FrameAllocator}, AddrOps, VirtualAddress, FRAME_PAGE_SIZE};
use core::{alloc::{GlobalAlloc, Layout}, cmp::max, ptr::NonNull};
use spin::Mutex;

struct FreedBlock {
    curr_size: usize,
    next_freed_block: Option<NonNull<FreedBlock>>
}

pub struct SimplePageAllocatorInner<A: FrameAllocator + 'static> {
    heap_start: VirtualAddress,
    heap_size: usize,

    next_block: VirtualAddress,
    freed_blocks: Option<NonNull<FreedBlock>>,

    frame_allocator: Option<&'static A>,
}

unsafe impl<A: FrameAllocator + 'static> Send for SimplePageAllocatorInner<A> {}

pub struct SimplePageAllocator<A: FrameAllocator + 'static>(Mutex<SimplePageAllocatorInner<A>>);

/*
 * This just sets some defualt values that will get initialized in init().
 */
#[global_allocator]
pub static PAGE_ALLOCATOR: SimplePageAllocator<SimpleFrameAllocator> = SimplePageAllocator (Mutex::new(SimplePageAllocatorInner {
    heap_start  : 0x0,
    heap_size   : 0,
    next_block  : 0x0,
    freed_blocks: None,
    frame_allocator: None,
}));

impl<A: FrameAllocator + 'static> SimplePageAllocator<A> {
    /*
     * Safety: init() can only be called once or the allocator might get into an inconsistent state.
     * However, it must be called.
     */
    pub unsafe fn init(&self, heap_start: VirtualAddress, heap_size: usize, frame_allocator: &mut A) {
        debug_assert!(heap_start % FRAME_PAGE_SIZE == 0);
        debug_assert!(heap_size % FRAME_PAGE_SIZE == 0);

        let allocator = &mut *self.0.lock();
        allocator.heap_start = heap_start;
        allocator.heap_size  = heap_size;
        allocator.next_block = heap_start;

        // allocator.frame_allocator = Some(frame_allocator);
    }
}

unsafe impl<A: FrameAllocator + 'static> GlobalAlloc for SimplePageAllocator<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let allocator = &mut *self.0.lock();

        // try to find a free block first
        if let Some(freed_blocks) = allocator.freed_blocks {
            unimplemented!()
        }

        // make sure that alloc_start is always aligned for FreedBlock and Layout
        let alloc_start = allocator.next_block.align_up(max(align_of::<FreedBlock>(), layout.align()));

        if alloc_start + layout.size() > allocator.heap_start + allocator.heap_size {
            panic!("Out of heap memory!");
        }

        alloc_start as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unimplemented!()
    }
}
