pub mod temporary_page_allocator;
pub mod simple_page_allocator;
pub mod page_table;
pub mod paging;

use crate::{kernel::ORIGINALLY_IDENTITY_MAPPED, memory::{pages::{simple_page_allocator::BitmapPageAllocator, temporary_page_allocator::TemporaryPageAllocator}, FRAME_PAGE_SIZE}};
use super::{MemoryError, VirtualAddress};
use core::cell::LazyCell;
use spin::Mutex;

#[derive(Clone, Copy)]
pub struct Page(usize); // this usize is the page index in the virtual memory

/* ----------------- SOME NOTES ON PAGE TABLE INDEX CALCULATION -----------------
 * Let´s assume this address: 0xdeadbeef, with 4KiB pages (12 bits)
 * The calculated page index is: 912091 (0xdeadbeef / PAGE_SIZE)
 *
 * Page table indexes to translate the addr:
 * 0xdeadbeef:
 *  p4_idx   -> 0    (0xdeadbeef >> 39 & 0o777)
 *  p3_idx   -> 3    (0xdeadbeef >> 30 & 0o777)
 *  p2_idx   -> 245  (0xdeadbeef >> 21 & 0o777)
 *  p1_idx   -> 219  (0xdeadbeef >> 12 & 0o777)
 *  page_idx -> 239  (0xdeadbeef & 0o777)
 *
 * To calculate the table indexes but with the page index instead:
 * idx:
 *  p4_idx   -> 0    (912091 >> (39 - 12) & 0o777)
 *  p3_idx   -> 3    (912091 >> (30 - 12) & 0o777)
 *  p2_idx   -> 245  (912091 >> (21 - 12) & 0o777)
 *  p1_idx   -> 219  (912091 >> (12 - 12) & 0o777)
 *
 * We need to subtract 12 because the page index is 4096 (4KiB) times smaller than the original addr.
 */
impl Page {
    pub fn from_virt_addr(addr: VirtualAddress) -> Result<Page, MemoryError> {
        // in x86_64, the top 16 bits of a virtual addr must be sign extension bits. if they are not, its an invalid addr
        // !(addr < 0x0000_8000_0000_0000 || addr >= 0xffff_8000_0000_0000)
        if (0x0000_8000_0000_0000..0xffff_8000_0000_0000).contains(&addr) {
            return Err(MemoryError::PageInvalidVirtualAddress);
        }

        Ok(Page(addr / FRAME_PAGE_SIZE))
    }

    pub fn addr(&self) -> VirtualAddress {
        self.0 * FRAME_PAGE_SIZE
    }

    pub fn p4_index(&self) -> usize {
        (self.0 >> 27) & 0o777
    }

    pub fn p3_index(&self) -> usize {
        (self.0 >> 18) & 0o777
    }

    pub fn p2_index(&self) -> usize {
        (self.0 >> 9) & 0o777
    }

    #[allow(clippy::identity_op)]
    pub fn p1_index(&self) -> usize {
        (self.0 >> 0) & 0o777
    }
}

/// Represents a page allocator to be used OS wide.
/// 
/// # Safety
/// 
/// Implementors must adhere to the following rules:
/// - The client **should** only be created after the [Frame Allocator](super::frames::FrameAllocator) is available, and it should, preferably, be static.
/// - Frame allocations are allowed as well as the use of the [Paging Context](crate::globals::ACTIVE_PAGING_CTX) for metadata creation.
/// - No more than one page allocator can be [initialized](PageAllocator::init) at the same time but, it is expected to have a two stage system
///   where a temporary page allocator is created that then gives place to a permanent one.
/// - The page allocator must ensure that the [kernel prohibited memory ranges](crate::kernel::Kernel::prohibited_memory_ranges) are **never** violated.
/// - If a two stage system is used:
///   - The temporary page allocator might or might not rely on [ORIGINALLY_IDENTITY_MAPPED](crate::kernel::ORIGINALLY_IDENTITY_MAPPED) for metadata but,
///     the permanent allocator **must** not.
///   - The temporary allocator **must** assume that it will be used just until the higher half remapping is performed at what point the switch
///     to the permanent allocator will happen.
///   - The permanent allocator **must** assume that it will be used after the higher half remapping is completed where,
///     only the higher half needs to be "allocatable". This is, 127.5TB as the paging system is recursive and so, we loose 512GB of virtual memory.
pub unsafe trait PageAllocator: Send + Sync {
    // const fn new() -> Self;

    fn allocate(&mut self) -> Result<Page, MemoryError>;
    fn allocate_contiguous(&mut self) -> Result<Page, MemoryError>;
    fn deallocate(&mut self, page: Page);

    /// Resets the page allocator state.
    /// 
    /// Any possible metadata **must** be initialized here.
    /// 
    /// # Safety
    /// 
    /// This must be called (before any allocation) as the allocator expects it.
    /// In the case that it does not get called, memory corruption is the most likely outcome.
    unsafe fn init(&self) -> Result<(), MemoryError>;
}

trait PageAllocatorStage {}

enum PageAllocatorFirstStage {}
enum PageAllocatorSecondStage {}

impl PageAllocatorStage for PageAllocatorFirstStage {}
impl PageAllocatorStage for PageAllocatorSecondStage {}

// TODO: read this: https://arunanshub.hashnode.dev/self-referential-structs-in-rust
// https://arunanshub.hashnode.dev/self-referential-structs-in-rust-part-2

// https://stackoverflow.com/questions/72379106/what-are-the-differences-between-cell-refcell-and-unsafecell
// look into the while covariant and invariants thing

static FIRST_STAGE_PA: Mutex<TemporaryPageAllocator> = TemporaryPageAllocator::new(ORIGINALLY_IDENTITY_MAPPED);
static SECOND_STAGE_PA: Mutex<BitmapPageAllocator> = BitmapPageAllocator::new();

pub struct GlobalPageAllocator {
    first_stage: &'static Mutex<dyn PageAllocator>,
    second_stage: &'static Mutex<dyn PageAllocator>,

    switched: bool,
}

// pub struct GlobalPageAllocator(GlobalPageAllocatorInner);
// unsafe impl<'a> Sync for GlobalPageAllocator {}

impl GlobalPageAllocator {
    pub(in crate::memory) const fn new() -> Self {
        todo!()
    }

    fn switch(&self) {
        unimplemented!()
    }

    pub fn allocate(&self) -> Result<Page, MemoryError> {
        todo!()
    }

    pub fn allocate_contiguous(&self) -> Result<Page, MemoryError> {
        todo!()
    }

    pub fn deallocate(&self, _page: Page) {
        todo!()
    }

    pub unsafe fn init(&self) -> Result<(), MemoryError> {
        todo!()
    }

    pub fn bruh(&mut self, a: &'static Mutex<dyn PageAllocator>) {
        self.first_stage = a;
    }
}
