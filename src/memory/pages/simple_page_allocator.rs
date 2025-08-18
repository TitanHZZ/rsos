use crate::{data_structures::bitmap_ref_mut::BitmapRefMut, memory::{pages::{Page, PageAllocator}, MemoryError}};
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

pub struct BitmapPageAllocator<'a> {
    l1: [Option<BitmapRefMut<'a>>; 261120], // every bitmap is 16kb
}

impl<'a> BitmapPageAllocator<'a> {
    pub const fn new() -> Self {
        BitmapPageAllocator {
            l1: [const { None }; 261120]
        }
    }
}

unsafe impl<'a> PageAllocator for BitmapPageAllocator<'a> {
    // fn new() -> Self {
    //     BitmapPageAllocator {
    //         l1: [const { None }; 261120]
    //     }
    // }

    unsafe fn init(&self) -> Result<(), MemoryError> {
        todo!()
    }

    fn allocate(&self) -> Result<Page, MemoryError> {
        todo!()
    }

    fn allocate_contiguous(&self) -> Result<Page, MemoryError> {
        todo!()
    }

    fn deallocate(&self, _page: Page) {
        todo!()
    }
}
