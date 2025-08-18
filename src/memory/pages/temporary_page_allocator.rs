use crate::{globals::ACTIVE_PAGING_CTX, memory::{pages::{Page, PageAllocator}, MemoryError, VirtualAddress, FRAME_PAGE_SIZE}, serial_println};
use crate::data_structures::bitmap::Bitmap;
use spin::Mutex;

struct TemporaryPageAllocatorInner {
    bitmap: Bitmap<1>,
    start_addr: VirtualAddress,
}

/// A page allocator meant to be used until a permanent page allocator is initialized.
pub struct TemporaryPageAllocator(Mutex<TemporaryPageAllocatorInner>);

impl TemporaryPageAllocator {
    /// Create a new **TemporaryPageAllocator** that holds 8 pages for allocation starting at `start_addr`.
    /// 
    /// # Panics
    /// If `start_addr` is not a multiple of **FRAME_PAGE_SIZE**.
    pub const fn new(start_addr: VirtualAddress) -> Self {
        assert!(start_addr.is_multiple_of(FRAME_PAGE_SIZE));
        TemporaryPageAllocator(Mutex::new(TemporaryPageAllocatorInner {
            bitmap: Bitmap::new(None),
            start_addr,
        }))
    }
}

unsafe impl PageAllocator for TemporaryPageAllocator {
    // /// Create a new **TemporaryPageAllocator** that holds 8 pages for allocation starting at `start_addr`.
    // fn new() -> Self {
    //     TemporaryPageAllocator {
    //         bitmap: Bitmap::new(None),
    //         start_addr: ORIGINALLY_IDENTITY_MAPPED,
    //     }
    // }

    unsafe fn init(&self) -> Result<(), MemoryError> {
        // make sure that the pages are not being used
        for i in 0..self.bitmap.len() {
            let addr = self.start_addr + i * FRAME_PAGE_SIZE;
            ACTIVE_PAGING_CTX.translate(addr).map_err(|_| MemoryError::BadTemporaryPageAllocator)?;
        }

        Ok(())
    }

    fn allocate(&mut self) -> Result<Page, MemoryError> {
        // look for the first free page and return it
        let idx = self.bitmap.iter().enumerate().find(|(_, bit)| !bit).ok_or(MemoryError::NotEnoughVirMemory)?.0;
        self.bitmap.set(idx, true);

        let page = Page::from_virt_addr(self.start_addr + idx * FRAME_PAGE_SIZE)?;
        serial_println!("Allocated page: {:#x}", page.0);

        Ok(page)
    }

    fn allocate_contiguous(&mut self) -> Result<Page, MemoryError> {
        todo!()
    }

    fn deallocate(&mut self, page: Page) {
        // make sure that the address is valid and within range
        assert!(page.addr() >= self.start_addr && page.addr() < (self.start_addr + self.bitmap.len() * FRAME_PAGE_SIZE));

        // make sure that the page was previously allocated
        let bit_idx = (page.addr() - self.start_addr) / FRAME_PAGE_SIZE;
        assert!(self.bitmap.get(bit_idx).is_some());

        // deallocate
        self.bitmap.set(bit_idx, false);

        serial_println!("Deallocated page: {:#x}", page.0);
    }
}
