use crate::{memory::{frames::{simple_frame_allocator::FRAME_ALLOCATOR, Frame}, pages::page_table::page_table_entry::EntryFlags}, println, print};
use crate::memory::{AddrOps, MemoryError, VirtualAddress, FRAME_PAGE_SIZE};
use core::{alloc::{GlobalAlloc, Layout}, cmp::max, ptr::NonNull};
use super::paging::ActivePagingContext;
use spin::Mutex;

struct FreedBlock {
    curr_size: usize,
    next_freed_block: Option<NonNull<FreedBlock>>
}

struct SimplePageAllocatorInner {
    heap_start: VirtualAddress,
    heap_size: usize,

    next_block: VirtualAddress,
    freed_blocks: Option<NonNull<FreedBlock>>,

    apc: Option<&'static ActivePagingContext>,
}

unsafe impl Send for SimplePageAllocatorInner {}

pub struct SimplePageAllocator(Mutex<SimplePageAllocatorInner>);

/*
 * This just sets some defualt values that will get initialized in init().
 */
#[global_allocator]
pub static HEAP_ALLOCATOR: SimplePageAllocator = SimplePageAllocator(Mutex::new(SimplePageAllocatorInner { 
    heap_start  : 0x0,
    heap_size   : 0,
    next_block  : 0x0,
    freed_blocks: None,
    apc: None,
}));

impl SimplePageAllocator {
    /*
     * Safety: init() can only be called once or the allocator might get into an inconsistent state.
     * However, it must be called as the allocator expects it.
     */
    pub unsafe fn init(&self, heap_start: VirtualAddress, heap_size: usize, apc: &'static ActivePagingContext) -> Result<(), MemoryError> {
        debug_assert!(heap_start % FRAME_PAGE_SIZE == 0);
        debug_assert!(heap_size % FRAME_PAGE_SIZE == 0);

        let allocator = &mut *self.0.lock();
        allocator.heap_start = heap_start;
        allocator.heap_size  = heap_size;
        allocator.next_block = heap_start;

        allocator.apc = Some(apc);

        // we are going to lazily allocate the required frames (for now we allocate just the first one)
        apc.map(heap_start, &FRAME_ALLOCATOR, EntryFlags::PRESENT | EntryFlags::WRITABLE | EntryFlags::NO_EXECUTE)
    }
}

unsafe impl GlobalAlloc for SimplePageAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let allocator = &mut *self.0.lock();

        // try to find a free block first
        if let Some(freed_blocks) = allocator.freed_blocks {
            unimplemented!()
        }

        let real_align = max(align_of::<FreedBlock>(), layout.align());
        let real_size = layout.size().align_up(real_align); // buffer overflows??

        let alloc_start = allocator.next_block.align_up(real_align);



        let alloc_end = alloc_start + layout.size() - 1;
        let align = max(align_of::<FreedBlock>(), layout.align());
        let real_alloc_size = layout.size().align_up(align);
        println!("{} -- {} -- {}", layout.size(), layout.align(), real_alloc_size);






        // make sure that alloc_start is always aligned for FreedBlock and Layout
        let alloc_start = allocator.next_block.align_up(max(align_of::<FreedBlock>(), layout.align()));
        let alloc_end = alloc_start + layout.size() - 1;

        if alloc_start + layout.size() > allocator.heap_start + allocator.heap_size {
            panic!("Out of heap memory!");
        }

        // check if we need to allocate more frames to hold the new heap allocated data
        if allocator.next_block.align_down(FRAME_PAGE_SIZE) != alloc_end.align_down(FRAME_PAGE_SIZE) {
            let start_addr = allocator.next_block.align_up(FRAME_PAGE_SIZE);
            let end_addr = alloc_end.align_up(FRAME_PAGE_SIZE) - 1;

            for addr in (start_addr..=end_addr).step_by(FRAME_PAGE_SIZE) {
                allocator.apc.unwrap().map(addr, &FRAME_ALLOCATOR, EntryFlags::PRESENT | EntryFlags::WRITABLE | EntryFlags::NO_EXECUTE)
                    .expect("Could not allocate more frame for the heap memory.");
            }
        }

        alloc_start as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unimplemented!()
    }
}
