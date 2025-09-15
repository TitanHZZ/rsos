use crate::memory::{pages::{page_table::page_table_entry::EntryFlags, Page, PageAllocator}, MemoryError, VirtualAddress, FRAME_PAGE_SIZE, MEMORY_SUBSYSTEM};
use crate::{data_structures::bitmap::Bitmap, serial_println};
use crate::{assert_called_once, kernel::Kernel};
use spin::Mutex;

struct TemporaryPageAllocatorInner {
    bitmap: Bitmap<1>,
    start_addr: VirtualAddress,

    initialized: bool,
}

/// A page allocator meant to be used until a permanent page allocator is initialized.
pub struct TemporaryPageAllocator(Mutex<TemporaryPageAllocatorInner>);

impl TemporaryPageAllocator {
    /// Create a new **TemporaryPageAllocator** that holds 8 pages for allocation starting at [ORIGINALLY_IDENTITY_MAPPED](crate::kernel::ORIGINALLY_IDENTITY_MAPPED).
    #[cfg(not(test))]
    pub(in crate::memory::pages) const fn new() -> Self {
        TemporaryPageAllocator(Mutex::new(TemporaryPageAllocatorInner {
            bitmap: Bitmap::new(None),
            start_addr: Kernel::originally_identity_mapped(),

            initialized: false,
        }))
    }

    /// Create a new **TemporaryPageAllocator** that holds 8 pages for allocation starting at [ORIGINALLY_IDENTITY_MAPPED](crate::kernel::ORIGINALLY_IDENTITY_MAPPED).
    #[cfg(test)]
    pub const fn new() -> Self {
        TemporaryPageAllocator(Mutex::new(TemporaryPageAllocatorInner {
            bitmap: Bitmap::new(None),
            start_addr: Kernel::originally_identity_mapped(),

            initialized: false,
        }))
    }
}

unsafe impl PageAllocator for TemporaryPageAllocator {
    unsafe fn init(&self) -> Result<(), MemoryError> {
        assert_called_once!("Cannot call TemporaryPageAllocator::init() more than once");
        let allocator = &mut *self.0.lock();

        // make sure that the pages are not being used
        for i in 0..allocator.bitmap.len() {
            let addr = allocator.start_addr + i * FRAME_PAGE_SIZE;
            MEMORY_SUBSYSTEM.active_paging_context().translate(addr).map_err(|_| MemoryError::BadTemporaryPageAllocator)?;
        }

        allocator.initialized = true;
        Ok(())
    }

    fn allocate(&self, map_page: bool) -> Result<Page, MemoryError> {
        let allocator = &mut *self.0.lock();
        assert!(allocator.initialized);

        // look for the first free page and return it
        let idx = allocator.bitmap.iter().enumerate().find(|(_, bit)| !bit).ok_or(MemoryError::NotEnoughVirMemory)?.0;
        allocator.bitmap.set(idx, true);

        let page = Page::from_virt_addr(allocator.start_addr + idx * FRAME_PAGE_SIZE)?;
        if map_page {
            MEMORY_SUBSYSTEM.active_paging_context().map_page(page, EntryFlags::PRESENT | EntryFlags::WRITABLE | EntryFlags::NO_EXECUTE)?;
        }

        serial_println!("Allocated page: {:#x}", page.0);
        Ok(page)
    }

    fn allocate_contiguous(&self, _count: usize, _map_pages: bool) -> Result<Page, MemoryError> {
        let allocator = &mut *self.0.lock();
        assert!(allocator.initialized);

        todo!()
    }

    unsafe fn deallocate(&self, page: Page, unmap_page: bool) {
        unsafe { self.deallocate_contiguous(page, 1, unmap_page) };
    }

    unsafe fn deallocate_contiguous(&self, page: Page, count: usize, unmap_pages: bool) {
        let allocator = &mut *self.0.lock();
        assert!(allocator.initialized && count > 0);

        for offset in 0..count {
            let page_at_offset = Page::from_virt_addr(page.addr() + offset * FRAME_PAGE_SIZE).unwrap();

            // make sure that the address is valid and within range
            assert!(page_at_offset.addr() >= allocator.start_addr && page_at_offset.addr() < (allocator.start_addr + allocator.bitmap.len() * FRAME_PAGE_SIZE));

            // make sure that the page was previously allocated
            let bit_idx = (page_at_offset.addr() - allocator.start_addr) / FRAME_PAGE_SIZE;
            assert!(allocator.bitmap.get(bit_idx).is_some());

            // deallocate
            allocator.bitmap.set(bit_idx, false);
            if unmap_pages {
                MEMORY_SUBSYSTEM.active_paging_context().unmap_page(page_at_offset, true).unwrap();
            }
        }

        if count == 1 {
            serial_println!("Deallocated page: {:#x}", page.0);
        } else {
            serial_println!("Deallocated {} contiguous pages: {:#x}", count, page.0);
        }
    }
}
