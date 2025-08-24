pub mod simple_frame_allocator;
pub mod bitmap_frame_allocator;

use crate::{kernel::Kernel, memory::{frames::bitmap_frame_allocator::BitmapFrameAllocator, ProhibitedMemoryRange}};
use super::{MemoryError, PhysicalAddress, FRAME_PAGE_SIZE};
use core::cell::Cell;

#[repr(transparent)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub struct Frame(usize); // this usize is the frame index in the physical memory

impl Frame {
    pub fn from_phy_addr(addr: PhysicalAddress) -> Frame {
        Frame(addr / FRAME_PAGE_SIZE)
    }

    pub fn addr(&self) -> PhysicalAddress {
        self.0 * FRAME_PAGE_SIZE
    }
}

/// Represents the public interface of a frame allocator.
/// 
/// # Safety
/// 
/// Implementors must ensure that they adhere to these contracts:
/// - The client **should** be created very early on, and it should, preferably, be static.
/// - [init()](FrameAllocator::init) **must** be called very early on, before the higher half remapping and before performing any allocations.
/// - The frame allocator must ensure that the [kernel prohibited memory ranges](Kernel::prohibited_memory_ranges) are **never** violated.
/// - Only [valid RAM](crate::multiboot2::memory_map::MemoryMapEntries::usable_areas) can be used for metadata, if necessary.
/// - If metadata is used, it will **need** to be remapped with [remap()](FrameAllocator::remap) as soon as the higher half remapping is completed.
/// - No more than one frame allocator is ever expected to be initialized at the same time.
/// - The allocator may rely on [ORIGINALLY_IDENTITY_MAPPED](crate::kernel::ORIGINALLY_IDENTITY_MAPPED) to safely create it's metadata.
/// - The use of the [Page Allocator](crate::memory::pages::PageAllocator) is **prohibited** to ensure that no recursive state is ever reached.
/// - The use of the [Paging Context](crate::memory::pages::paging::ActivePagingContext) is also **prohibited** to ensure that no recursive state is ever reached.
/// 
/// # Const Constructors
/// 
/// All implementors **must** implement the following constructors:
/// - `pub(in crate::memory::frames) const fn new() -> Self;` with `#[cfg(not(test))]`
/// - `pub const fn new() -> Self;` with `#[cfg(test)]`
/// 
/// These are requirements because Rust doesn't yet support const trait functions.
/// 
/// These shall be used for normal use and testing respectively. They might just be the same with different visibility.
pub unsafe trait FrameAllocator: Send + Sync {
    /// Allocate a single frame.
    /// 
    /// # Panics
    /// 
    /// If called before [initialization](FrameAllocator::init()).
    fn allocate(&self) -> Result<Frame, MemoryError>;

    /// Deallocates `frame`.
    /// 
    /// # Panics
    /// 
    /// - May panic when trying to deallocate a frame not currently allocated by this frame allocator (implementation dependent).
    /// - If called before [initialization](FrameAllocator::init()).
    fn deallocate(&self, frame: Frame);

    /// Resets the frame allocator state.
    /// 
    /// All metadata (if used) **must** initialized here
    /// 
    /// # Safety
    /// 
    /// **Must** be called before performing any allocations as the allocator expects it.
    /// 
    /// Otherwise, memory corruption or undefined behavior can happen.
    /// 
    /// # Panics
    /// 
    /// If called more than once.
    unsafe fn init(&self, kernel: &Kernel) -> Result<(), MemoryError>;

    /// Remaps the frame allocator to use higher half mapping with its metadata.
    /// 
    /// This **does not** recreate the frame allocator, it simply adjusts the internal metadata structure so,
    /// [init()](FrameAllocator::init) **shouldn't** be called after this.
    /// 
    /// # Safety
    /// 
    /// **Must** be called right after the higher half remapping is completed and before any more frame allocations.
    /// 
    /// # Panics
    /// 
    /// - If called before [initialization](FrameAllocator::init()).
    /// - If called more than once.
    unsafe fn remap(&self, kernel: &Kernel);

    /// Get the metadata memory range that **must** be correctly mapped and that **cannot** be used for allocations.
    /// 
    /// This region **must** be remmaped before calling [remap()](FrameAllocator::remap) and no allocations can be done in between.
    /// 
    /// The addresses are virtual and so, they change from before higher half remapping to after it.
    /// 
    /// # Panics
    /// 
    /// If called before [initialization](FrameAllocator::init()).
    fn metadata_memory_range(&self) -> Option<ProhibitedMemoryRange>;
}

static FA: BitmapFrameAllocator = BitmapFrameAllocator::new();

/// The global frame allocator.
pub struct GlobalFrameAllocator {
    fa: Cell<&'static dyn FrameAllocator>,
}

unsafe impl Sync for GlobalFrameAllocator {}

impl GlobalFrameAllocator {
    pub const fn new() -> Self {
        GlobalFrameAllocator {
            fa: Cell::new(&FA),
        }
    }

    /// Sets the frame allocator to `allocator`.
    /// 
    /// # Safety
    /// 
    /// - **Must** be called before [init()](FrameAllocator::init)ing the allocator but NEVER after.
    /// - This operation is **NOT** thread safe.
    /// 
    /// Failure to follow the rules will result in data races and data corruption.
    #[cfg(test)]
    pub unsafe fn set_first_stage_allocator(&self, allocator: &'static dyn FrameAllocator) {
        self.fa.set(allocator);
    }
}

unsafe impl FrameAllocator for GlobalFrameAllocator {
    fn allocate(&self) -> Result<Frame, MemoryError> {
        self.fa.get().allocate()
    }

    fn deallocate(&self, frame: Frame) {
        self.fa.get().deallocate(frame);
    }

    unsafe fn init(&self, kernel: &Kernel) -> Result<(), MemoryError> {
        unsafe { self.fa.get().init(kernel) }
    }

    unsafe fn remap(&self, kernel: &Kernel) {
        unsafe { self.fa.get().remap(kernel) };
    }

    fn metadata_memory_range(&self) -> Option<ProhibitedMemoryRange> {
        self.fa.get().metadata_memory_range()
    }
}
