pub mod temporary_page_allocator;
pub mod simple_page_allocator;
pub mod page_table;
pub mod paging;

use crate::memory::{pages::{simple_page_allocator::BitmapPageAllocator, temporary_page_allocator::TemporaryPageAllocator}};
use crate::{assert_called_once, memory::FRAME_PAGE_SIZE};
use super::{MemoryError, VirtualAddress};
use core::cell::Cell;

// TODO: look into the whole covariant and invariants thing (and aliasing)

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

/// Represents the public interface of a page allocator.
/// 
/// # Safety
/// 
/// Implementors must adhere to the following rules:
/// - The client **should** only be initialized after the [Frame Allocator](super::frames::FrameAllocator) is available, and it should, preferably, be static.
/// - Frame allocations are allowed as well as the use of the [Paging Context](crate::globals::ACTIVE_PAGING_CTX) for metadata creation.
/// - No more than one page allocator can be [initialized](PageAllocator::init) and used at the same time but, a two stage system is expected
///   where a temporary page allocator is created that then gives place to a permanent one.
/// - The page allocator must ensure that the [kernel prohibited memory ranges](crate::kernel::Kernel::prohibited_memory_ranges) are **never** violated.
/// - If a two stage system is used:
///   - The temporary page allocator might or might not rely on [ORIGINALLY_IDENTITY_MAPPED](crate::kernel::ORIGINALLY_IDENTITY_MAPPED) for metadata but,
///     the permanent allocator **must** not.
///   - The temporary allocator **must** assume that it will be used just until the higher half remapping is performed at what point the switch
///     to the permanent allocator will happen.
///   - The permanent allocator **must** assume that it will be used after the higher half remapping is completed where,
///     only the higher half needs to be "allocatable". This is, 127.5TB as the paging system is recursive and so, we loose 512GB of virtual memory.
/// 
/// # Const Constructors
/// 
/// All implementors **must** implement the following constructors:
/// - `pub(in crate::memory::pages) const fn new() -> Self;` with `#[cfg(not(test))]`
/// - `pub const fn new() -> Self;` with `#[cfg(test)]`
/// 
/// These are requirements because Rust doesn't yet support const trait functions.
/// 
/// These shall be used for normal use and testing respectively. They might just be the same with different visibility.
pub unsafe trait PageAllocator: Send + Sync {
    /// Allocate a single page.
    /// 
    /// # Panics
    /// 
    /// If called before [initialization](PageAllocator::init()).
    fn allocate(&self) -> Result<Page, MemoryError>;

    /// Allocate multiple, contiguous, pages.
    /// 
    /// Returns the first page in the contiguous block.
    /// 
    /// # Panics
    /// 
    /// - If called before [initialization](PageAllocator::init()).
    /// - If `count` is 0.
    fn allocate_contiguous(&self, count: usize) -> Result<Page, MemoryError>;

    /// Deallocates `page`.
    /// 
    /// # Safety
    /// 
    /// The consumer deallocating a page **must** be the same as the one that previously allocated it.
    /// 
    /// Otherwise, the consumer might cause memory corruption or undefined behavior.
    /// 
    /// # Panics
    /// 
    /// - May panic when trying to deallocate a page not currently allocated by this page allocator (implementation dependent).
    /// - If called before [initialization](PageAllocator::init()).
    unsafe fn deallocate(&self, page: Page);

    /// Deallocates `count` pages starting at `page`.
    /// 
    /// # Safety
    /// 
    /// The consumer deallocating the pages **must** be the same as the one that previously allocated them.
    /// 
    /// Otherwise, the consumer might cause memory corruption or undefined behavior.
    /// 
    /// # Panics
    /// 
    /// - May panic when trying to deallocate pages not currently allocated by this page allocator (implementation dependent).
    /// - If called before [initialization](PageAllocator::init()).
    /// - If `count` is 0.
    unsafe fn deallocate_contiguous(&self, page: Page, count: usize);

    /// Resets the page allocator state.
    /// 
    /// All metadata (if used) **must** initialized here.
    /// 
    /// # Safety
    /// 
    /// **Must** be called before performing any allocations as the allocator expects it.
    /// 
    /// Otherwise, memory corruption or undefined behavior can happen.
    /// 
    /// # Panics
    /// 
    /// If called more than once per allocator stage.
    unsafe fn init(&self) -> Result<(), MemoryError>;
}

static FIRST_STAGE_PA: TemporaryPageAllocator = TemporaryPageAllocator::new();
static SECOND_STAGE_PA: BitmapPageAllocator = BitmapPageAllocator::new();

pub struct GlobalPageAllocator {
    first_stage: Cell<&'static dyn PageAllocator>,
    second_stage: Cell<&'static dyn PageAllocator>,

    switched: Cell<bool>,
}

unsafe impl Sync for GlobalPageAllocator {}

impl GlobalPageAllocator {
    // The global page allocator does not follow the requirement of 2 new() fns simply because we do not need it.
    // Only the "real" allocators must be tested, and since this is wrapper, this does not.
    pub(in crate::memory) const fn new() -> Self {
        GlobalPageAllocator {
            first_stage: Cell::new(&FIRST_STAGE_PA),
            second_stage: Cell::new(&SECOND_STAGE_PA),
            switched: Cell::new(false),
        }
    }

    fn current(&self) -> &'static dyn PageAllocator {
        match self.switched.get() {
            true => self.second_stage.get(),
            false => self.first_stage.get(),
        }
    }

    /// Switches from the first stage page allocator to the second stage.
    /// 
    /// # Safety
    /// 
    /// - **Must** be called *after* the remap to the higher half, *after* [fixing the Kernel structure](crate::kernel::Kernel::rebuild()) and
    ///   *after* [remapping the frame allocator](crate::memory::frames::FrameAllocator::remap()) but before any higher half page allocations.
    /// - No allocations/deallocations can ever cross the switch so, users cannot allocate in the first stage, switch and then deallocate for example.
    /// - This operation is **NOT** thread safe.
    /// 
    /// Failure to follow these rules will result in data races and data corruption.
    /// 
    /// # Panics
    /// 
    /// If called more than once.
    pub unsafe fn switch(&self) {
        assert_called_once!("Cannot call GlobalPageAllocator::switch() more than once");
        self.switched.set(true);
    }

    /// Sets the first stage page allocator to `allocator`.
    /// 
    /// # Safety
    /// 
    /// - **Must** be called before [init()](GlobalPageAllocator::init)ing the first stage allocator but NEVER after.
    /// - This operation is **NOT** thread safe.
    /// 
    /// Failure to follow the rules will result in data races and data corruption.
    #[cfg(test)]
    pub unsafe fn set_first_stage_allocator(&self, allocator: &'static dyn PageAllocator) {
        self.first_stage.set(allocator);
    }

    /// Sets the second stage page allocator to `allocator`.
    /// 
    /// # Safety
    /// 
    /// - **Must** be called before [init()](GlobalPageAllocator::init())ing the second stage allocator but NEVER after.
    /// - This operation is **NOT** thread safe.
    /// 
    /// Failure to follow the rules will result in data races and data corruption.
    #[cfg(test)]
    pub unsafe fn set_second_stage_allocator(&self, allocator: &'static dyn PageAllocator) {
        self.second_stage.set(allocator);
    }
}

unsafe impl PageAllocator for GlobalPageAllocator {
    fn allocate(&self) -> Result<Page, MemoryError> {
        self.current().allocate()
    }

    fn allocate_contiguous(&self, count: usize) -> Result<Page, MemoryError> {
        self.current().allocate_contiguous(count)
    }

    unsafe fn deallocate(&self, page: Page) {
        unsafe { self.current().deallocate(page) };
    }

    unsafe fn deallocate_contiguous(&self, page: Page, count: usize) {
        unsafe { self.current().deallocate_contiguous(page, count) };
    }

    unsafe fn init(&self) -> Result<(), MemoryError> {
        unsafe { self.current().init() }
    }
}
