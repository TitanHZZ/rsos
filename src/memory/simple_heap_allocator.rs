use crate::{memory::{pages::PageAllocator, AddrOps, MemoryError, VirtualAddress, FRAME_PAGE_SIZE, MEMORY_SUBSYSTEM}, serial_print, serial_println};
use core::{alloc::{GlobalAlloc, Layout}, cmp::max, ptr::{addr_of, eq as ptr_eq, NonNull}};
use crate::memory::pages::page_table::page_table_entry::EntryFlags;
use spin::Mutex;

// TODO: this could probably use an 'initialized' flag

struct FreedBlock {
    size: usize,
    next_freed_block: Option<NonNull<FreedBlock>>
}

struct SimpleHeapAllocatorInner {
    heap_start: VirtualAddress,
    heap_size: usize,

    next_block: VirtualAddress,
    freed_blocks: Option<NonNull<FreedBlock>>,

    max_mapped_addr: VirtualAddress,
}

unsafe impl Send for SimpleHeapAllocatorInner {}

pub struct SimpleHeapAllocator(Mutex<SimpleHeapAllocatorInner>);

// This just sets some default values that will get initialized in init().
#[global_allocator]
pub static HEAP_ALLOCATOR: SimpleHeapAllocator = SimpleHeapAllocator(Mutex::new(SimpleHeapAllocatorInner { 
    heap_start     : 0x0,
    heap_size      : 0,
    next_block     : 0x0,
    freed_blocks   : None,
    max_mapped_addr: 0,
}));

impl SimpleHeapAllocator {
    /// Initialized the heap with `heap_page_size` pages.
    /// 
    /// # Safety
    /// 
    /// Can only be called once or the allocator might get into an inconsistent state.  
    /// However, it must be called as the allocator expects it.
    pub unsafe fn init(&self, heap_page_size: usize) -> Result<(), MemoryError> {
        assert!(heap_page_size > 0);

        let allocator = &mut *self.0.lock();
        let heap_start = MEMORY_SUBSYSTEM.page_allocator().allocate_contiguous(heap_page_size, false)?.addr();
        allocator.heap_start = heap_start;
        allocator.heap_size  = heap_page_size * FRAME_PAGE_SIZE;
        allocator.next_block = heap_start;

        // we are going to lazily allocate the required frames (for now we allocate just the first one)
        allocator.max_mapped_addr = heap_start + FRAME_PAGE_SIZE - 1;
        MEMORY_SUBSYSTEM.active_paging_context().map(heap_start, EntryFlags::PRESENT | EntryFlags::WRITABLE | EntryFlags::NO_EXECUTE)
    }

    // DEBUG fn
    pub fn print_freed_blocks(&self) {
        let allocator = &mut *self.0.lock();

        // serial_println!("+------------------------+");
        if allocator.freed_blocks.is_none() {
            serial_println!("| Empty |");
            return;
        }

        let first_block = unsafe { allocator.freed_blocks.unwrap().as_mut() };
        serial_print!("| {:#x} : {} |", addr_of!(*first_block) as VirtualAddress, first_block.size);

        // loop through the blocks
        let mut current_block = first_block;
        let mut option_next_block = current_block.next_freed_block;
        while let Some(mut addr_next_block) = option_next_block {
            let next_block = unsafe { addr_next_block.as_mut() };

            serial_print!(" -> | {:#x} : {} |", addr_of!(*next_block) as VirtualAddress, next_block.size);

            current_block = next_block;
            option_next_block = current_block.next_freed_block;
        }

        serial_println!("");
    }
}

// TODO: write tests for all of this
// TODO: do all these functions really need to be unsafe??
impl SimpleHeapAllocatorInner {
    /// # Safety
    /// 
    /// The caller must ensure that `addr` is valid and points to usable memory.
    unsafe fn add_to_list(&mut self, addr: VirtualAddress, size: usize) {
        assert!(size >= size_of::<FreedBlock>());

        // set up defaults for the new block
        let block = unsafe { &mut *(addr as *mut FreedBlock) };
        block.next_freed_block = None;
        block.size = size;

        // add at the beginning (list is empty)
        if self.freed_blocks.is_none() {
            self.freed_blocks = Some(unsafe { NonNull::new_unchecked(addr as _) });
            return;
        }

        // add at the beginning (list is not empty)
        let mut addr_first_block = self.freed_blocks.unwrap();
        if addr_first_block.as_ptr() > addr as _ {
            self.freed_blocks = Some(unsafe { NonNull::new_unchecked(addr as _) });
            block.next_freed_block = Some(unsafe { NonNull::new_unchecked(addr_first_block.as_ptr()) });
            return;
        }

        // loop through the remaning blocks
        let mut current_block = unsafe { addr_first_block.as_mut() };
        let mut option_next_block = current_block.next_freed_block;
        while let Some(mut addr_next_block) = option_next_block {
            let next_block = unsafe { addr_next_block.as_mut() };

            // add in the middle of the list
            if addr_next_block.as_ptr() > addr as _ {
                current_block.next_freed_block = Some(unsafe { NonNull::new_unchecked(addr as _) });
                block.next_freed_block = Some(unsafe { NonNull::new_unchecked(addr_next_block.as_ptr()) });
                return;
            }

            current_block = next_block;
            option_next_block = current_block.next_freed_block;
        }

        // add at the end
        current_block.next_freed_block = Some(unsafe { NonNull::new_unchecked(addr as _) });
    }

    /// # Safety
    /// 
    /// The caller must ensure that `addr` is valid and points to usable memory. `self.freed_blocks` must be Some(_)
    unsafe fn remove_from_list(&mut self, addr: VirtualAddress) {
        assert!(self.freed_blocks.is_some());

        // check if the first block matches the addr
        let mut addr_first_block = self.freed_blocks.unwrap();
        let first_block = unsafe { addr_first_block.as_mut() };

        if ptr_eq(addr_first_block.as_ptr(), addr as _) {
            self.freed_blocks = first_block.next_freed_block.take();
            return;
        }

        // loop through the remaning blocks
        let mut current_block = first_block;
        let mut option_next_block = current_block.next_freed_block;
        while let Some(mut addr_next_block) = option_next_block {
            let next_block = unsafe { addr_next_block.as_mut() };

            // check if the block matches and remove it if so
            if ptr_eq(addr_next_block.as_ptr(), addr as _) {
                current_block.next_freed_block = next_block.next_freed_block.take();
                break;
            }

            current_block = next_block;
            option_next_block = current_block.next_freed_block;
        }
    }

    // TODO: this only checks if the requested block fits in the start of the freed blocks but,
    //       while this might not be the case, the requested lbock might fit with an offset inside the
    //       freed block. could/should this be implemented??
    /// # Safety
    /// 
    /// The caller must ensure that `real_align` and `real_size` are valid.
    unsafe fn get_from_list(&mut self, real_align: usize, real_size: usize) -> Option<*mut u8> {
        assert!(real_align >= align_of::<FreedBlock>());
        assert!(real_size >= size_of::<FreedBlock>());

        // loop all the blocks to find the first that matches the requirements
        let mut option_current_block = self.freed_blocks;
        while let Some(mut addr_current_block) = option_current_block {
            let current_block = unsafe { addr_current_block.as_mut() };

            // the block must have matching alignment
            if !(addr_current_block.as_ptr() as VirtualAddress).is_multiple_of(real_align) {
                option_current_block = current_block.next_freed_block;
                continue;
            }

            // ideal case
            if current_block.size == real_size {
                unsafe { self.remove_from_list(addr_current_block.as_ptr() as VirtualAddress) }
                return Some(addr_current_block.as_ptr() as *mut u8);
            }

            // if the sizes do not match, it must have enough space to fit a new FreedBlock
            if current_block.size >= real_size + size_of::<FreedBlock>() {
                unsafe { self.remove_from_list(addr_current_block.as_ptr() as VirtualAddress) }

                let free_block_addr = unsafe { addr_current_block.byte_offset(real_size as _).as_ptr() as VirtualAddress };
                let free_block_size = current_block.size - real_size;
                unsafe { self.add_to_list(free_block_addr, free_block_size) }

                assert!(free_block_addr.is_multiple_of(size_of::<FreedBlock>()));
                assert!(free_block_size >= size_of::<FreedBlock>());

                return Some(addr_current_block.as_ptr() as *mut u8);
            }

            option_current_block = current_block.next_freed_block;
        }

        // no matching blocks
        None
    }

    unsafe fn unify_list(&mut self) {
        if self.freed_blocks.is_none() {
            return;
        }

        let first_block = unsafe { self.freed_blocks.unwrap().as_mut() };

        // loop through the blocks
        let mut current_block = first_block;
        let mut option_next_block = current_block.next_freed_block;
        while let Some(mut addr_next_block) = option_next_block {
            let mut next_block = unsafe { addr_next_block.as_mut() };

            let addr_current_block = current_block as *const FreedBlock as VirtualAddress;
            if addr_current_block + current_block.size == addr_next_block.as_ptr() as VirtualAddress {
                let new_size = current_block.size + next_block.size;
                let new_next = next_block.next_freed_block;

                unsafe { self.remove_from_list(addr_next_block.as_ptr() as _) }
                current_block.size = new_size;
                current_block.next_freed_block = new_next;

                // this will make it loop again over the block that just got expanded
                next_block = current_block;
            }

            current_block = next_block;
            option_next_block = current_block.next_freed_block;
        }
    }
}

unsafe impl GlobalAlloc for SimpleHeapAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let allocator = &mut *self.0.lock();
        assert!(allocator.next_block.is_multiple_of(size_of::<FreedBlock>()));

        let real_align = max(size_of::<FreedBlock>(), layout.align());
        let real_size  = max(layout.size().align_up(real_align), size_of::<FreedBlock>()); // buffer overflows??

        // try to find a free block first
        if let Some(addr) = unsafe { allocator.get_from_list(real_align, real_size) } {
            return addr;
        }

        let mut freed_block_needed = false;
        let freed_block_addr = allocator.next_block;
        let mut alloc_start = allocator.next_block.align_up(real_align);

        // TODO: this is only required when layout.align() > align_of::<FreedBlock>() and if it is big enough,
        //       we might not need to add size_of::<FreedBlock>();
        // check if we need padding
        if alloc_start != allocator.next_block {
            // we need to make space for the FreedBlock and realign the start (the space used for padding will be a new FreedBlock)
            freed_block_needed = true;
            alloc_start += size_of::<FreedBlock>();
            alloc_start = alloc_start.align_up(real_align);
        }

        if alloc_start + real_size > allocator.heap_start + allocator.heap_size {
            // TODO: this should probably return a null ptr to indicate failure on allocation
            panic!("Out of heap memory!");
        }

        allocator.next_block = alloc_start + real_size;

        // check if we need to allocate and map more frames to hold the new heap allocated data
        if allocator.next_block > allocator.max_mapped_addr {
            let start_addr = allocator.max_mapped_addr.align_up(FRAME_PAGE_SIZE);
            let end_addr = (allocator.next_block + 1).align_up(FRAME_PAGE_SIZE) - 1;

            for addr in (start_addr..=end_addr).step_by(FRAME_PAGE_SIZE) {
                MEMORY_SUBSYSTEM.active_paging_context().map(addr, EntryFlags::PRESENT | EntryFlags::WRITABLE | EntryFlags::NO_EXECUTE)
                    .expect("Could not allocate more frames for the heap memory.");
            }

            allocator.max_mapped_addr = end_addr;
        }

        assert!(allocator.max_mapped_addr >= allocator.next_block);
        assert!(allocator.next_block.is_multiple_of(size_of::<FreedBlock>()));
        assert!((allocator.max_mapped_addr + 1).is_multiple_of(FRAME_PAGE_SIZE));

        if freed_block_needed {
            // add the FreedBlock to the linked list
            unsafe { allocator.add_to_list(freed_block_addr, alloc_start - freed_block_addr) }
        }

        alloc_start as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        assert!((ptr as usize).is_multiple_of(size_of::<FreedBlock>()));
        assert!((ptr as usize).is_multiple_of(layout.align()));

        let allocator = &mut *self.0.lock();
        let real_align = max(size_of::<FreedBlock>(), layout.align());
        let real_size  = max(layout.size().align_up(real_align), size_of::<FreedBlock>()); // buffer overflows??

        unsafe {
            allocator.add_to_list(ptr as VirtualAddress, real_size);
            allocator.unify_list();
        }
    }
}
