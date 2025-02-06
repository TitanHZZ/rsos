mod inactive_paging_context;

use crate::memory::{frames::{Frame, FrameAllocator}, MemoryError, PhysicalAddress, VirtualAddress, FRAME_PAGE_SIZE};
use super::{page_table::{page_table_entry::EntryFlags, Level4, Table, P4}, Page};
use core::{arch::asm, marker::PhantomData, ptr::NonNull};

/*
 * Safety: Raw pointers are not Send/Sync so `Paging` cannot be used between threads as it would cause data races.
 */
pub struct ActivePagingContext {
    p4: NonNull<Table<Level4>>,

    // makes this struct `own` a `Table<Level4>`
    _marker: PhantomData<Table<Level4>>,
}

impl ActivePagingContext {
    /*
     * Safety: This should be unsafe because the p4 addr will always be the same (at least for now),
     * and that means that creating multiple `Paging` objects could result in undefined behaviour
     * as all the `Paging` objects woulb be pointing to the same memory (and own it).
     */
    pub unsafe fn new() -> Self {
        ActivePagingContext {
            // this can be unchecked as we know that the ptr is non null
            p4: NonNull::new_unchecked(P4),
            _marker: PhantomData,
        }
    }

    fn p4(&self) -> &Table<Level4> {
        unsafe { self.p4.as_ref() }
    }

    fn p4_mut(&mut self) -> &mut Table<Level4> {
        unsafe { self.p4.as_mut() }
    }

    pub fn map_page_to_frame<A: FrameAllocator>(&mut self, page: Page, frame: Frame, frame_allocator: &mut A, flags: EntryFlags) -> Result<(), MemoryError> {
        let p4 = self.p4_mut();
        let p3 = p4.create_next_table(page.p4_index(), frame_allocator)?;
        let p2 = p3.create_next_table(page.p3_index(), frame_allocator)?;
        let p1 = p2.create_next_table(page.p2_index(), frame_allocator)?;

        // the entry must be unused
        debug_assert!(!p1.entries[page.p1_index()].is_used());

        p1.entries[page.p1_index()].set(frame, flags | EntryFlags::PRESENT);
        Ok(())
    }

    pub fn map_page<A: FrameAllocator>(&mut self, page: Page, frame_allocator: &mut A, flags: EntryFlags) -> Result<(), MemoryError> {
        // get a random (free) frame
        let frame = frame_allocator.allocate_frame()?;
        return self.map_page_to_frame(page, frame, frame_allocator, flags);
    }

    pub fn map<A: FrameAllocator>(&mut self, virtual_addr: VirtualAddress, frame_allocator: &mut A, flags: EntryFlags) -> Result<(), MemoryError> {
        let page = Page::from_virt_addr(virtual_addr)?;
        return self.map_page(page, frame_allocator, flags);
    }

    /*
     * This will unmap a page and the respective frame.
     * If an invalidd page is given, it will simply be ignored as there is nothing to unmap.
     */
    // TODO: - free P1, P2 and P3 if they get empty
    //       - deallocate the frame
    pub fn unmap_page<A: FrameAllocator>(&mut self, page: Page, frame_allocator: &mut A) {
        // set the entry in p1 as unused and free the respective frame
        self.p4_mut().next_table(page.p4_index())
            .and_then(|p3: _| p3.next_table_mut(page.p3_index()))
            .and_then(|p2: _| p2.next_table_mut(page.p2_index()))
            .and_then(|p1: _| {
                let entry = &mut p1.entries[page.p1_index()];
                let frame = entry.pointed_frame();
                entry.set_unused();

                frame
            }).and_then(|frame| {
                // deallocate the frame
                // frame_allocator.deallocate_frame(frame);

                Some(()) // `and_then()` requires an Option to be returned
            });

        // invalidate the TLB entry
        unsafe {
            asm!("invlpg [{}]", in(reg) page.addr() as u64, options(nostack, preserves_flags));
        }
    }

    /*
     * This takes a Page and returns the respective Frame if the address is mapped.
     */
    fn translate_page(&self, page: Page) -> Option<Frame> {
        self.p4().next_table(page.p4_index())
            .and_then(|p3| p3.next_table(page.p3_index()))
            .and_then(|p2| p2.next_table(page.p2_index()))
            .and_then(|p1| p1.entries[page.p1_index()].pointed_frame())
    }

    /*
     * Takes a virtual address and returns the respective physical address if it exists (if it is mapped).
     */
    pub fn translate(&self, virtual_addr: VirtualAddress) -> Result<Option<PhysicalAddress>, MemoryError> {
        let offset = virtual_addr % FRAME_PAGE_SIZE;
        let page = Page::from_virt_addr(virtual_addr)?;

        match self.translate_page(page) {
            Some(frame) => return Ok(Some(frame.addr() + offset)),
            None => return Ok(None),
        }
    }
}
