pub mod inactive_paging_context;

use crate::memory::{cr3::CR3, frames::{Frame, FrameAllocator}, pages::PageAllocator, MemoryError, PhysicalAddress, VirtualAddress, FRAME_PAGE_SIZE, MEMORY_SUBSYSTEM};
use super::{page_table::{page_table_entry::EntryFlags, Level4, Table, ENTRY_COUNT, P4}, Page};
use crate::{globals::{FRAME_ALLOCATOR}, serial_println};
use inactive_paging_context::InactivePagingContext;
use core::{marker::PhantomData, ptr::NonNull};
use spin::Mutex;

// Safety:
// Raw pointers are not Send/Sync so `Paging` cannot be used between threads as it would cause data races.
/// Represents a paging context (active and currently being used).
pub(in crate::memory) struct ActivePagingContextInner {
    p4: NonNull<Table<Level4>>,

    // makes this struct `own` a `Table<Level4>`
    _marker: PhantomData<Table<Level4>>,
}

unsafe impl Send for ActivePagingContextInner {}

pub struct ActivePagingContext(Mutex<ActivePagingContextInner>);

impl ActivePagingContextInner {
    fn p4(&self) -> &Table<Level4> {
        unsafe { self.p4.as_ref() }
    }

    fn p4_mut(&mut self) -> &mut Table<Level4> {
        unsafe { self.p4.as_mut() }
    }

    /// Maps a specific Page to a specific Frame.
    pub(in crate::memory) fn map_page_to_frame(&mut self, page: Page, frame: Frame, flags: EntryFlags) -> Result<(), MemoryError> {
        let p4 = self.p4_mut();
        let p3 = p4.create_next_table(page.p4_index())?;
        let p2 = p3.0.create_next_table(page.p3_index())?;
        let p1 = p2.0.create_next_table(page.p2_index())?;

        // the entry must be unused
        if p1.0.entries[page.p1_index()].is_used() {
            return Err(MemoryError::MappingUsedTableEntry);
        }

        // add the new entry to the p1 table
        p1.0.entries[page.p1_index()].set(frame, flags | EntryFlags::PRESENT);
        p1.0.set_used_entries_count(p1.0.used_entries_count() + 1); // a new entry was added

        // if the p1 table was created, we need to increment the entry count on the p2 table
        if p1.1 {
            p2.0.set_used_entries_count(p2.0.used_entries_count() + 1);
        }

        // if the p2 table was created, we need to increment the entry count on the p3 table
        if p2.1 {
            p3.0.set_used_entries_count(p3.0.used_entries_count() + 1);
        }

        Ok(())
    }

    /// Maps a specific Page to a (random) Frame.
    pub(in crate::memory) fn map_page(&mut self, page: Page, flags: EntryFlags) -> Result<(), MemoryError> {
        // get a random (free) frame
        let frame = FRAME_ALLOCATOR.allocate()?;
        self.map_page_to_frame(page, frame, flags)
    }

    /// Maps the Page containing the `virtual_addr` to a (random) Frame.
    pub(in crate::memory) fn map(&mut self, virtual_addr: VirtualAddress, flags: EntryFlags) -> Result<(), MemoryError> {
        let page = Page::from_virt_addr(virtual_addr)?;
        self.map_page(page, flags)
    }

    /// Maps a Frame to a Page with same addr (identity mapping).
    pub(in crate::memory) fn identity_map(&mut self, frame: Frame, flags: EntryFlags) -> Result<(), MemoryError> {
        self.map_page_to_frame(Page::from_virt_addr(frame.addr())?, frame, flags)
    }

    /// This will unmap a page and deallocate the respective frame, if requested. In the event that a page table gets emptied, it will also be deallocated.
    /// 
    /// If an invalid page is given, `MemoryError::UnmappingUnusedTableEntry` will be returned.
    pub(in crate::memory) fn unmap_page(&mut self, page: Page, deallocate_frame: bool) -> Result<(), MemoryError> {
        let p4 = self.p4_mut();
        let p3 = p4.next_table_mut(page.p4_index()).ok_or(MemoryError::UnmappingUnusedTableEntry)?;
        let p2 = p3.next_table_mut(page.p3_index()).ok_or(MemoryError::UnmappingUnusedTableEntry)?;
        let p1 = p2.next_table_mut(page.p2_index()).ok_or(MemoryError::UnmappingUnusedTableEntry)?;

        let entry = &mut p1.entries[page.p1_index()];
        let frame = entry.pointed_frame().ok_or(MemoryError::UnmappingUnusedTableEntry)?;

        entry.set_unused();
        p1.set_used_entries_count(p1.used_entries_count() - 1);

        if deallocate_frame {
            FRAME_ALLOCATOR.deallocate(frame);
        }

        if p1.used_entries_count() == 0 {
            // invalidate the TLB entry for the P1 page table
            CR3::invalidate_entry(p1 as *const _ as VirtualAddress);

            let entry = &mut p2.entries[page.p2_index()];
            let frame = entry.pointed_frame().unwrap();

            entry.set_unused();
            p2.set_used_entries_count(p2.used_entries_count() - 1);

            FRAME_ALLOCATOR.deallocate(frame);
            serial_println!("Deallocated a P1 table.");
        }

        if p2.used_entries_count() == 0 {
            // invalidate the TLB entry for the P2 page table
            CR3::invalidate_entry(p2 as *const _ as VirtualAddress);

            let entry = &mut p3.entries[page.p3_index()];
            let frame = entry.pointed_frame().unwrap();

            entry.set_unused();
            p3.set_used_entries_count(p3.used_entries_count() - 1);

            FRAME_ALLOCATOR.deallocate(frame);
            serial_println!("Deallocated a P2 table.");
        }

        if p3.used_entries_count() == 0 {
            // invalidate the TLB entry for the P3 page table
            CR3::invalidate_entry(p3 as *const _ as VirtualAddress);

            let entry = &mut p4.entries[page.p4_index()];
            let frame = entry.pointed_frame().unwrap();

            entry.set_unused();
            FRAME_ALLOCATOR.deallocate(frame);
            serial_println!("Deallocated a P3 table.");
        }

        // invalidate the TLB entry for the original page
        CR3::invalidate_entry(page.addr());

        Ok(())
    }

    /// This takes a Page and returns the respective Frame if the address is mapped.
    pub(in crate::memory) fn translate_page(&self, page: Page) -> Option<Frame> {
        self.p4().next_table(page.p4_index())
            .and_then(|p3| p3.next_table(page.p3_index()))
            .and_then(|p2| p2.next_table(page.p2_index()))
            .and_then(|p1| p1.entries[page.p1_index()].pointed_frame())
    }

    // Safety: does not need locking as we are calling translate_page() that will lock()
    /// Takes the `virtual address` and returns the respective physical address if it exists (if it is mapped).
    pub(in crate::memory) fn translate(&self, virtual_addr: VirtualAddress) -> Result<Option<PhysicalAddress>, MemoryError> {
        let offset = virtual_addr % FRAME_PAGE_SIZE;
        let page = Page::from_virt_addr(virtual_addr)?;

        match self.translate_page(page) {
            Some(frame) => Ok(Some(frame.addr() + offset)),
            None => Ok(None),
        }
    }

    /// The current active paging context will become inactive and the inactive one, will become active.
    pub(in crate::memory) fn switch(&self, inactive_context: &mut InactivePagingContext) {
        // the ActivePagingContext does not need to be modified as it only uses a recursive addr,
        // so it will work with whatever addr is in CR3

        // swap the values in CR3 and InactivePagingContext (also clears the TLB)
        inactive_context.switch_with_cr3();
    }
}

impl Default for ActivePagingContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ActivePagingContext {
    // TODO: this should not be public
    pub const fn new() -> Self {
        ActivePagingContext(Mutex::new(ActivePagingContextInner {
            // this can be unchecked as we know that the ptr is non null
            p4: unsafe { NonNull::new_unchecked(P4) },
            _marker: PhantomData,
        }))
    }

    /// Maps a specific Page to a specific Frame.
    pub fn map_page_to_frame(&self, page: Page, frame: Frame, flags: EntryFlags) -> Result<(), MemoryError> {
        let apc = &mut *self.0.lock();
        apc.map_page_to_frame(page, frame, flags)
    }

    /// Maps a specific Page to a (random) Frame.
    pub fn map_page<A: FrameAllocator>(&self, page: Page, flags: EntryFlags) -> Result<(), MemoryError> {
        let apc = &mut *self.0.lock();
        apc.map_page(page, flags)
    }

    /// Maps the Page containing the `virtual_addr` to a (random) Frame.
    pub fn map(&self, virtual_addr: VirtualAddress, flags: EntryFlags) -> Result<(), MemoryError> {
        let apc = &mut *self.0.lock();
        apc.map(virtual_addr, flags)
    }

    /// Maps a Frame to a Page with same addr (identity mapping).
    pub fn identity_map(&self, frame: Frame, flags: EntryFlags) -> Result<(), MemoryError> {
        let apc = &mut *self.0.lock();
        apc.identity_map(frame, flags)
    }

    /// This will unmap a `page` and the respective frame.
    /// 
    /// If an invalid `page` is given, it will simply be ignored as there is nothing to unmap.
    pub fn unmap_page(&self, page: Page, deallocate_frame: bool) -> Result<(), MemoryError> {
        let apc = &mut *self.0.lock();
        apc.unmap_page(page, deallocate_frame)
    }

    /// This takes a Page and returns the respective Frame if the address is mapped.
    pub fn translate_page(&self, page: Page) -> Option<Frame> {
        let apc = &*self.0.lock();
        apc.translate_page(page)
    }

    /// Takes the `virtual address` and returns the respective physical address if it exists (if it is mapped).
    pub fn translate(&self, virtual_addr: VirtualAddress) -> Result<Option<PhysicalAddress>, MemoryError> {
        let apc = &mut *self.0.lock();
        apc.translate(virtual_addr)
    }

    /// The current active paging context will become inactive and the inactive one, will become active.
    pub fn switch(&self, inactive_context: &mut InactivePagingContext) {
        let apc = &*self.0.lock();
        apc.switch(inactive_context);
    }

    /// # Safety
    /// 
    /// If `&mut ActivePagingContextInner` is used incorrectly, it will lead to UB so, please be careful and
    /// do not share or send the reference to anywhere else. This is why this function cannot be used outside of crate::memory.
    /// 
    /// This is a special method that should only be used inside `memory` as it should not
    /// really be part of the public interface.
    /// 
    /// This allows to call ActivePagingContext funcs on an InactivePagingContext object to
    /// map, unmap and translate addrs manually as if it were the active paging context.
    /// 
    /// This does not affect hardware translations and thus, is totally safe to use as long as the
    /// caller makes sure that the inactive paging context is in a valid state before being switched to.
    pub(in crate::memory) fn update_inactive_context<O>(&self, inactive_context: &InactivePagingContext, f: O) -> Result<(), MemoryError>
    where
        O: FnOnce(&mut ActivePagingContextInner) -> Result<(), MemoryError>,
    {
        let apc = &mut *self.0.lock();
        let page_allocator = MEMORY_SUBSYSTEM.page_allocator();

        // backup the current active paging p4 frame addr and map the current p4 table so we can change it later
        let p4_frame = Frame::from_phy_addr(CR3::get());
        let p4_page = page_allocator.allocate()?;
        apc.map_page_to_frame(p4_page, p4_frame, EntryFlags::PRESENT | EntryFlags::WRITABLE)?;

        // set the recusive entry on the current paging context to the inactive p4 frame
        apc.p4_mut().entries[ENTRY_COUNT - 1].set_phy_addr(inactive_context.p4_frame());

        // flush all the tlb entries
        // needed because the recursive addrs may be mapped to the active paging context and
        // we need them pointing to the inactive context (hardware translations would still work)
        CR3::invalidate_all();

        f(apc)?;

        // restore the active paging context recusive mapping
        let table = unsafe { &mut *(p4_page.addr() as *mut Table<Level4>) };
        table.entries[ENTRY_COUNT - 1].set_phy_addr(p4_frame);

        // invalidate the entries so that the recursive mapping works again (so that we don't use cached addrs)
        CR3::invalidate_all();

        // deallocate the page
        page_allocator.deallocate(p4_page);

        // do not deallocate the frame as it needs to remain valid (after all, it is the current p4 frame)
        apc.unmap_page(p4_page, false)
    }
}
