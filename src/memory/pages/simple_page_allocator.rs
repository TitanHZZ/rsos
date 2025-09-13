use crate::memory::{frames::FrameAllocator, pages::{page_table::page_table_entry::EntryFlags, Page, PageAllocator}, AddrOps, MemoryError, VirtualAddress};
use crate::{assert_called_once, data_structures::bitmap_ref_mut::BitmapRefMut, kernel::{Kernel, KERNEL}};
use crate::memory::{serial_println, FRAME_PAGE_SIZE, MEMORY_SUBSYSTEM};
use spin::Mutex;

// This page allocator manages the entire higher half of the 48 bit address space, 2 ** 48 // 2 bytes.
// 
// But, we don't actually need to manage all this memory, because the page tables are recursive meaning that
// the last P4 entry is mapped to the P4 page table itself. This removes 512GB of memory from the total,
// at the top of the address space so, now we "only" need to map 2 ** 48 // 2 - 2 ** 30 * 512.
// This is 140187732541440 bytes or 34225520640 4KB pages.
// So, we need 34225520640 bits and when using 16KB bitmaps, we get 34225520640 // (4096 * 8 * 4) bitmaps (261120)
// with 4096 * 8 bits per 4KB, and we want 4 4KBs.
// 
// All of this means that we need 261120, statically allocated, Option<BitmapRefMut<'a>>s where each bitmap will be of size 16KB.
// 
// Notes:
// 1MB --> 2 ** 20
// 1GB --> 2 ** 30
// 1TB --> 2 ** 40
// 
// All calculations are in Python syntax.

struct BitmapPageAllocatorInner<'a> {
    // every level 1 bitmap is 16kb
    l1: [Option<BitmapRefMut<'a>>; 261120],
    used_idxs_end: (usize, usize), // the last idxs used by the initialization (must NOT be used for allocations)
    initialized: bool,
}

impl<'a> BitmapPageAllocatorInner<'a> {
    /// Convert from a `page_idx` in the higher half to the l1 and l2 bitmap indexes that map the respective page.
    const fn page_idx_to_bit_idxs(&self, page_idx: usize) -> (usize, usize) {
        assert!(page_idx < (Kernel::hh_end() / FRAME_PAGE_SIZE));
        (page_idx / BitmapPageAllocator::level2_bitmap_bit_lenght(), page_idx % BitmapPageAllocator::level2_bitmap_bit_lenght())
    }

    /// Get the l1 and l2 bitmap indexes that map the respective `addr`.
    fn addr_to_bit_idxs(&self, addr: VirtualAddress) -> (usize, usize) {
        assert!(addr >= Kernel::k_hh_start() && addr <= Kernel::hh_end());
        let page_idx = (addr.align_down(FRAME_PAGE_SIZE) - Kernel::k_hh_start()) / FRAME_PAGE_SIZE;
        self.page_idx_to_bit_idxs(page_idx)
    }

    /// Get the address that is mapped by the l1 and l2 bitmap `idxs`.
    const fn bit_idxs_to_addr(&self, idxs: (usize, usize)) -> VirtualAddress {
        let page_idx = idxs.0 * BitmapPageAllocator::level2_bitmap_bit_lenght() + idxs.1;
        Kernel::k_hh_start() + page_idx * FRAME_PAGE_SIZE
    }

    /// Get the l2 bitmap start address from the respective l1 `bitmap_idx`.
    fn level2_bitmap_addr(&self, bitmap_idx: usize) -> VirtualAddress {
        assert!(bitmap_idx < 261120);
        BitmapPageAllocator::level2_bitmaps_start_addr() + (BitmapPageAllocator::level2_bitmap_lenght() * bitmap_idx)
    }

    /// Allocate the l2 bitmap with the respective l1 `bitmap_idx`, as well as, the necessary bitmaps to map the requested l2 bitmap.
    fn allocate_level2_bitmap(&mut self, bitmap_idx: usize) -> Result<(), MemoryError> {
        // allocate and map all the required pages for the second level bitmap
        let bitmap_start_addr = self.level2_bitmap_addr(bitmap_idx);
        for addr in (bitmap_start_addr..bitmap_start_addr + BitmapPageAllocator::level2_bitmap_lenght()).step_by(FRAME_PAGE_SIZE) {
            MEMORY_SUBSYSTEM.active_paging_context().map(addr, EntryFlags::PRESENT | EntryFlags::WRITABLE | EntryFlags::NO_EXECUTE)?;
        }

        // build the level 2 bitmap
        self.l1[bitmap_idx] = Some(unsafe {
            BitmapRefMut::from_raw_parts_mut(bitmap_start_addr as _, BitmapPageAllocator::level2_bitmap_lenght(), None, true)
        });

        // recursively allocate the second level bitmap that marks the current one as allocated
        let (l1_idx, l2_idx) = self.addr_to_bit_idxs(bitmap_start_addr);
        if self.l1[l1_idx].is_none() {
            self.allocate_level2_bitmap(l1_idx)?;
        }

        // mark the current second level bitmap pages as allocated in the new, recursively allocated, second level bitmap
        for offset in 0..BitmapPageAllocator::level2_bitmap_page_lenght() {
            self.l1[l1_idx].as_mut().unwrap().set(l2_idx + offset, true);
        }

        Ok(())
    }

    /// Deallocate the l2 bitmap with the respective l1 `bitmap_idx`, as well as, the necessary bitmaps that mapped the requested l2 bitmap for deallocation.
    fn deallocate_level2_bitmap(&mut self, bitmap_idx: usize) -> Result<(), MemoryError> {
        // allocate and map all the required pages for the second level bitmap
        let bitmap_start_addr = self.level2_bitmap_addr(bitmap_idx);
        for addr in (bitmap_start_addr..bitmap_start_addr + BitmapPageAllocator::level2_bitmap_lenght()).step_by(FRAME_PAGE_SIZE) {
            MEMORY_SUBSYSTEM.active_paging_context().unmap_page(Page::from_virt_addr(addr)?, true)?;
        }

        self.l1[bitmap_idx] = None;

        // recursively deallocate the second level bitmap
        let (l1_idx, l2_idx) = self.addr_to_bit_idxs(bitmap_start_addr);
        assert!(self.l1[l1_idx].is_some());

        // mark the current second level bitmap pages as deallocated
        for offset in 0..BitmapPageAllocator::level2_bitmap_page_lenght() {
            self.l1[l1_idx].as_mut().unwrap().set(l2_idx + offset, false);
        }

        // recursively deallocate the second level bitmap that marked the current one, but is now empty
        if self.l1[l1_idx].as_ref().unwrap().zeroed() {
            self.deallocate_level2_bitmap(l1_idx)?;
        }

        Ok(())
    }
}

pub struct BitmapPageAllocator<'a>(Mutex<BitmapPageAllocatorInner<'a>>);

impl<'a> BitmapPageAllocator<'a> {
    #[cfg(not(test))]
    pub(in crate::memory::pages) const fn new() -> Self {
        BitmapPageAllocator(Mutex::new(BitmapPageAllocatorInner {
            l1: [const { None }; 261120],
            used_idxs_end: (0, 0),
            initialized: false,
        }))
    }

    #[cfg(test)]
    pub const fn new() -> Self {
        BitmapPageAllocator(Mutex::new(BitmapPageAllocatorInner {
            l1: [const { None }; 261120],
            used_idxs_end: (0, 0),
            initialized: false,
        }))
    }

    /// Get the size of a level 2 bitmap in bytes.
    const fn level2_bitmap_lenght() -> usize {
        FRAME_PAGE_SIZE * 4
    }

    /// Get the size of a level 2 bitmap in bits.
    const fn level2_bitmap_bit_lenght() -> usize {
        BitmapPageAllocator::level2_bitmap_lenght() * 8
    }

    /// Get the size of a level 2 bitmap in pages.
    const fn level2_bitmap_page_lenght() -> usize {
        BitmapPageAllocator::level2_bitmap_lenght() / FRAME_PAGE_SIZE
    }

    /// Get the address where the first level 2 bitmap will start.
    fn level2_bitmaps_start_addr() -> VirtualAddress {
        (KERNEL.fa_hh_start() + match MEMORY_SUBSYSTEM.frame_allocator().metadata_memory_range() {
            Some(metadata) => metadata.length(),
            None => 0,
        }).align_up(Self::level2_bitmap_lenght())
    }
}

unsafe impl<'a> PageAllocator for BitmapPageAllocator<'a> {
    unsafe fn init(&self) -> Result<(), MemoryError> {
        assert_called_once!("Cannot call BitmapPageAllocator::init() more than once");
        let allocator = &mut *self.0.lock();

        // the amount of bytes currently "allocated" in the higher half
        let allocated_size = BitmapPageAllocator::level2_bitmaps_start_addr() - Kernel::k_hh_start();

        let allocated_size_in_pages = allocated_size / FRAME_PAGE_SIZE;
        let level2_bitmap_count = allocated_size_in_pages.align_up(BitmapPageAllocator::level2_bitmap_bit_lenght())
            / BitmapPageAllocator::level2_bitmap_bit_lenght();

        // allocate all the necessary level2 bitmaps
        for bitmap_idx in 0..level2_bitmap_count {
            assert!(allocator.l1[bitmap_idx].is_none());
            allocator.allocate_level2_bitmap(bitmap_idx)?;
        }

        // mark the kernel, the multiboot2 and the frame allocator memory regions as allocated
        for page_idx in 0..allocated_size_in_pages {
            let (l1_idx, l2_idx) = allocator.page_idx_to_bit_idxs(page_idx);
            allocator.l1[l1_idx].as_mut().unwrap().set(l2_idx, true);
        }

        let idxs = allocator.addr_to_bit_idxs(allocator.level2_bitmap_addr(261120 - 1));
        allocator.used_idxs_end = (idxs.0, idxs.1 + BitmapPageAllocator::level2_bitmap_page_lenght() - 1);
        allocator.initialized = true;

        serial_println!("Page allocator last used idxs: {:?}", allocator.used_idxs_end);
        Ok(())
    }

    fn allocate(&self) -> Result<Page, MemoryError> {
        self.allocate_contiguous(1)
    }

    fn allocate_contiguous(&self, count: usize) -> Result<Page, MemoryError> {
        let allocator = &mut *self.0.lock();
        assert!(allocator.initialized && count > 0);

        let mut consecutive_free_count = 0;
        let mut start_of_block_idxs = None;

        // 'search block to find a contiguous region of `count` free pages
        'search: for l1_idx in allocator.used_idxs_end.0..allocator.l1.len() {
            let level2_bitmap_offset = if allocator.used_idxs_end.0 == l1_idx {
                allocator.used_idxs_end.1 + 1
            } else {
                0
            };

            match &allocator.l1[l1_idx] {
                // this l2 bitmap hasn't been allocated yet
                None => {
                    if start_of_block_idxs.is_none() {
                        start_of_block_idxs = Some((l1_idx, level2_bitmap_offset));
                    }

                    consecutive_free_count += BitmapPageAllocator::level2_bitmap_bit_lenght() - level2_bitmap_offset;
                    if consecutive_free_count >= count {
                        break 'search;
                    }
                }

                // this l2 bitmap is mapped, so we need to inspect the bits
                Some(l2_bitmap) => {
                    for l2_idx in level2_bitmap_offset..BitmapPageAllocator::level2_bitmap_bit_lenght() {
                        // check if the page is free
                        if !l2_bitmap.get(l2_idx).unwrap() {
                            if start_of_block_idxs.is_none() {
                                start_of_block_idxs = Some((l1_idx, l2_idx));
                            }

                            consecutive_free_count += 1;
                            if consecutive_free_count >= count {
                                break 'search;
                            }
                        } else {
                            // the page is used so, the contiguous block is broken
                            consecutive_free_count = 0;
                            start_of_block_idxs = None;
                        }
                    }
                }
            }
        }

        // a block large enough was not found
        if consecutive_free_count < count {
            return Err(MemoryError::NotEnoughVirMemory);
        }

        let start_of_block_idxs = start_of_block_idxs.unwrap();
        let (mut current_l1_idx, mut current_l2_idx) = start_of_block_idxs;

        // mark the `count` pages as used
        for _ in 0..count {
            if allocator.l1[current_l1_idx].is_none() {
                allocator.allocate_level2_bitmap(current_l1_idx)?;
            }

            // set the page as used
            allocator.l1[current_l1_idx].as_mut().unwrap().set(current_l2_idx, true);

            // go to the next page index
            current_l2_idx += 1;
            if current_l2_idx == Self::level2_bitmap_bit_lenght() {
                current_l2_idx = 0;
                current_l1_idx += 1;
            }
        }

        let start_addr = allocator.bit_idxs_to_addr(start_of_block_idxs);
        if count == 1 {
            serial_println!("Allocated page: {:#x} {:?}", start_addr, start_of_block_idxs);
        } else {
            serial_println!("Allocated {} contiguous pages: {:#x} {:?}", count, start_addr, start_of_block_idxs);
        }

        Page::from_virt_addr(start_addr)
    }

    unsafe fn deallocate(&self, page: Page) {
        // let allocator = &mut *self.0.lock();
        // assert!(allocator.initialized);
        // let (l1_idx, l2_idx) = allocator.addr_to_bit_idxs(page.addr());
        // assert!(allocator.l1[l1_idx].as_ref().unwrap().get(l2_idx).unwrap() == true);
        // allocator.l1[l1_idx].as_mut().unwrap().set(l2_idx, false);
        // if allocator.l1[l1_idx].as_ref().unwrap().zeroed() {
        //     allocator.deallocate_level2_bitmap(l1_idx).unwrap();
        // }
        // serial_println!("Deallocated page: {:#x} {:?}", page.addr(), (l1_idx, l2_idx));

        unsafe { self.deallocate_contiguous(page, 1) };
    }

    unsafe fn deallocate_contiguous(&self, page: Page, count: usize) {
        let allocator = &mut *self.0.lock();
        assert!(allocator.initialized && count > 0);

        for offset in 0..count {
            let page_at_offset = Page::from_virt_addr(page.addr() + offset * FRAME_PAGE_SIZE).unwrap();

            let (l1_idx, l2_idx) = allocator.addr_to_bit_idxs(page_at_offset.addr());
            assert!(allocator.l1[l1_idx].as_ref().unwrap().get(l2_idx).unwrap());

            allocator.l1[l1_idx].as_mut().unwrap().set(l2_idx, false);
            if allocator.l1[l1_idx].as_ref().unwrap().zeroed() {
                allocator.deallocate_level2_bitmap(l1_idx).unwrap();
            }
        }

        if count == 1 {
            serial_println!("Deallocated page: {:#x} {:?}", page.0, allocator.addr_to_bit_idxs(page.addr()));
        } else {
            serial_println!("Deallocated {} contiguous pages: {:#x} {:?}", count, page.0, allocator.addr_to_bit_idxs(page.addr()));
        }
    }
}
