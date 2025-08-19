pub mod temporary_page_allocator;
pub mod simple_page_allocator;
pub mod page_table;
pub mod paging;

use crate::{assert_called_once, kernel::ORIGINALLY_IDENTITY_MAPPED, memory::{pages::{simple_page_allocator::BitmapPageAllocator, temporary_page_allocator::TemporaryPageAllocator}, FRAME_PAGE_SIZE}};
use super::{MemoryError, VirtualAddress};
use core::cell::Cell;

// TODO: write a call_once() macro
// TODO: what does rust consider "safe"? is there a list?
// TODO: should the global page allocator implement the PageAllocator trait?

// TODO: read this: https://arunanshub.hashnode.dev/self-referential-structs-in-rust
// https://arunanshub.hashnode.dev/self-referential-structs-in-rust-part-2

// https://stackoverflow.com/questions/72379106/what-are-the-differences-between-cell-refcell-and-unsafecell
// look into the while covariant and invariants thing

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

// TODO: we require new()
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
    fn allocate(&self) -> Result<Page, MemoryError>;
    fn allocate_contiguous(&self) -> Result<Page, MemoryError>;
    fn deallocate(&self, page: Page);

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

static FIRST_STAGE_PA: TemporaryPageAllocator = TemporaryPageAllocator::new(ORIGINALLY_IDENTITY_MAPPED);
static SECOND_STAGE_PA: BitmapPageAllocator = BitmapPageAllocator::new();

pub struct GlobalPageAllocator {
    first_stage: Cell<&'static dyn PageAllocator>,
    second_stage: Cell<&'static dyn PageAllocator>,

    switched: Cell<bool>,
}

unsafe impl Sync for GlobalPageAllocator {}

impl GlobalPageAllocator {
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

    unsafe fn switch(&self) {
        assert_called_once!("Cannot call GlobalPageAllocator::switch() more than once");
        self.switched.set(true);
    }

    #[cfg(test)]
    pub unsafe fn set_first_stage_allocator(&self, allocator: &'static dyn PageAllocator) {
        self.first_stage.set(allocator);
    }

    #[cfg(test)]
    pub unsafe fn set_second_stage_allocator(&self, allocator: &'static dyn PageAllocator) {
        self.second_stage.set(allocator);
    }

    // ----- Page Allocator Forwarding ----- //

    pub fn allocate(&self) -> Result<Page, MemoryError> {
        self.current().allocate()
    }

    pub fn allocate_contiguous(&self) -> Result<Page, MemoryError> {
        self.current().allocate_contiguous()
    }

    pub fn deallocate(&self, page: Page) {
        self.current().deallocate(page);
    }

    pub unsafe fn init(&self) -> Result<(), MemoryError> {
        unsafe { self.current().init() }
    }
}

/*
hi, i am currently making an OS in rust and i have a problem where i have a struct that should hold 2 references
to 2 different static objects. these references will change the static object that they point to at runtime.
how should i approach this?

the holder will also be static and thus, i think that i will have to use interior mutabiliy
to change the references/pointers to the 2 static object

i also need a boolean flag that should tell me if i should use the first or the second reference.
i wll never use them both at the same time but i still need it to store two refs/ptrs to 2 different statics.

currently i have this struct:

pub struct GlobalPageAllocator {
    first_stage: &'static dyn PageAllocator,
    second_stage: &'static dyn PageAllocator,

    switched: bool,
}

this trait:

pub unsafe trait PageAllocator: Send + Sync {
    fn allocate(&self) -> Result<Page, MemoryError>;
    fn allocate_contiguous(&self) -> Result<Page, MemoryError>;
    fn deallocate(&self, page: Page);

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

and this allocators:

static FIRST_STAGE_PA: TemporaryPageAllocator = TemporaryPageAllocator::new(ORIGINALLY_IDENTITY_MAPPED);
static SECOND_STAGE_PA: BitmapPageAllocator = BitmapPageAllocator::new();

this code is not doing all that i need it to do and is incomplete. I need the first stage, second stage and switched struct fields to be "changeable".
i know that i could add a mutex to the global page allocatot but that would make it necessary to go through 2 mutexes to reach the
"real allocator" which just sonds wrong. in the future, there will also be some testings only fns that would allow me to "hook" into
the global page allocator to change the references to externally defined allocators for testing purposes only.

Some notes:
- i have thought of making the global page allocator own the first and second stage allocator but that would create self referecing
struct that are a pain in Rust and woulkd just create more problems because i would still need the references for the externally defined
allocators for testing.

so, i am unsure of what to do.
please do not be afraid to suggest a better approach

i am in no_std making an OS so Box and all of those types that require allocs are out of the question.
also, i do want it to be safe for multi threaded environments not just impl Sync to fool rust when it
is actually not thread safe. currently i am using the spin crate for mutexes.

the page allocators are serializing the allocations anyway (at least for now) so that is not really a consideration 
*/
