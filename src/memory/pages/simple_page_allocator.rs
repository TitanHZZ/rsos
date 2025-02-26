use crate::memory::{frames::simple_frame_allocator::FRAME_ALLOCATOR, pages::page_table::page_table_entry::EntryFlags};
use crate::memory::{AddrOps, MemoryError, VirtualAddress, FRAME_PAGE_SIZE};
use core::{alloc::{GlobalAlloc, Layout}, cmp::max, ptr::NonNull};
use super::paging::ActivePagingContext;
use spin::Mutex;

struct FreedBlock {
    size: usize,
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
 * This just sets some default values that will get initialized in init().
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

impl SimplePageAllocatorInner {
    // Safety: The caller must ensure that `addr` is valid and points to usable memory.
    unsafe fn add_to_list(&mut self, addr: VirtualAddress, size: usize) {
        debug_assert!(size >= size_of::<FreedBlock>());

        let block = &mut *(addr as *mut FreedBlock);
        block.next_freed_block = None;
        block.size = size;

        match self.freed_blocks {
            Some(mut addr_first_block) => addr_first_block.as_mut().add_to_list(block),
            // we assume that `addr` is valid
            None => self.freed_blocks = Some(NonNull::new_unchecked(block as _)),
        }
    }

    fn get_from_list(&self, align: usize, size: usize) -> Option<*mut u8> {
        debug_assert!(align >= align_of::<FreedBlock>());
        debug_assert!(size >= size_of::<FreedBlock>());

        match self.freed_blocks {
            Some(_) => todo!(),
            None => None
        }
    }
}

impl FreedBlock {
    // Safety: The caller must ensure that `block` is valid and points to usable memory.
    unsafe fn add_to_list(&mut self, block: &mut FreedBlock) {
        // recursively add to the linked list
        match self.next_freed_block {
            Some(mut addr_next_block) => {
                let next_block = addr_next_block.as_mut();

                if addr_next_block.as_ptr() > block as _ {
                    self.next_freed_block = Some(NonNull::new_unchecked(block as _));
                    block.next_freed_block = Some(NonNull::new_unchecked(addr_next_block.as_ptr()));
                    return;
                }

                next_block.add_to_list(block);
            },
            // we assume that `addr` is valid
            None => self.next_freed_block = Some(NonNull::new_unchecked(block as _)),
        }
    }
}

unsafe impl GlobalAlloc for SimplePageAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let allocator = &mut *self.0.lock();

        // try to find a free block first
        if let Some(freed_blocks) = allocator.freed_blocks {
            unimplemented!()
        }

        debug_assert!(allocator.next_block % size_of::<FreedBlock>() == 0);

        let real_align = max(align_of::<FreedBlock>(), layout.align());
        let real_size = layout.size().align_up(real_align); // buffer overflows??

        let mut freed_block_needed = false;
        let freed_block_addr = allocator.next_block;
        let mut alloc_start = allocator.next_block.align_up(real_align);

        // check if we need padding
        if alloc_start != allocator.next_block {
            // we need to make space for the FreedBlock and realign the start (the space used for padding will be a new FreedBlock)
            freed_block_needed = true;
            alloc_start += size_of::<FreedBlock>();
            alloc_start = alloc_start.align_up(real_align);
        }

        if alloc_start + real_size > allocator.heap_start + allocator.heap_size {
            panic!("Out of heap memory!");
        }

        let alloc_end = alloc_start + real_size - 1;

        // check if we need to allocate more frames to hold the new heap allocated data
        if allocator.next_block.align_down(FRAME_PAGE_SIZE) != alloc_end.align_down(FRAME_PAGE_SIZE) {
            let start_addr = allocator.next_block.align_up(FRAME_PAGE_SIZE);
            let end_addr = alloc_end.align_up(FRAME_PAGE_SIZE) - 1;

            for addr in (start_addr..=end_addr).step_by(FRAME_PAGE_SIZE) {
                allocator.apc.unwrap().map(addr, &FRAME_ALLOCATOR, EntryFlags::PRESENT | EntryFlags::WRITABLE | EntryFlags::NO_EXECUTE)
                    .expect("Could not allocate more frame for the heap memory.");
            }
        }

        allocator.next_block = alloc_start + real_size;
        debug_assert!(allocator.next_block % size_of::<FreedBlock>() == 0);

        if freed_block_needed {
            // add the FreedBlock
            allocator.add_to_list(freed_block_addr, alloc_start - freed_block_addr);
        }

        alloc_start as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        debug_assert!(ptr as usize % size_of::<FreedBlock>() == 0);
        debug_assert!(ptr as usize % layout.align() == 0);

        let allocator = &mut *self.0.lock();
        let real_align = max(align_of::<FreedBlock>(), layout.align());
        let real_size = layout.size().align_up(real_align); // buffer overflows??

        allocator.add_to_list(ptr as VirtualAddress, real_size);
    }
}
