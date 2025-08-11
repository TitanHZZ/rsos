pub mod simple_frame_allocator;
pub mod bitmap_frame_allocator;

use core::ops::Deref;

use super::{MemoryError, PhysicalAddress, FRAME_PAGE_SIZE};
use crate::{memory::ProhibitedMemoryRange, kernel::Kernel};

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

/// Represents a frame allocator to be used OS wide.
/// 
/// # Safety
/// 
/// Implementors must ensure that they adhere to these contracts:
/// - The client **should** be created very early on, and it should, preferably, be static.
/// - [init()](FrameAllocator::init) **must** be called very early on, preferably before remapping the [kernel](Kernel) and [multiboot2](crate::multiboot2::MbBootInfo) to the higher half.
/// - The frame allocator must ensure that the [kernel prohibited memory ranges](Kernel::prohibited_memory_ranges) are **never** violated.
/// - Only [valid RAM](crate::multiboot2::memory_map::MemoryMapEntries::usable_areas) can be used for metadata, if necessary.
/// - If metadata is used, it will **need** to be remapped with [remap()](FrameAllocator::remap) as soon as the higher half remapping is completed.
/// - No more than one frame allocator is expected to ever exist at runtime.
/// - The allocator may rely on [ORIGINALLY_IDENTITY_MAPPED](crate::kernel::ORIGINALLY_IDENTITY_MAPPED) to safely create it's metadata.
/// - The use of the [Page Allocator](crate::memory::pages::PageAllocator) is **prohibited** to ensure that no recursive state is ever reached.
/// - The use of the [Paging Context](crate::globals::ACTIVE_PAGING_CTX) is also **prohibited** to ensure that no recursive state is ever reached.
pub unsafe trait FrameAllocator: Send + Sync {
    fn allocate(&self) -> Result<Frame, MemoryError>;
    fn deallocate(&self, frame: Frame);

    /// Initializes/Resets the frame allocator state.
    /// 
    /// Any possible metadata **must** be initialized here.
    /// 
    /// # Safety
    /// 
    /// This must be called, before any allocation is performed, as the allocator expects it.
    /// 
    /// In the case that it does not get called, memory corruption is the most likely outcome.
    /// 
    /// Must also never be called more than once.
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
    /// **Cannot** be called more than once per remapping.
    unsafe fn remap(&self, kernel: &Kernel);

    /// Get the metadata memory range that **must** be correctly mapped and that **cannot** be used for allocations.
    /// 
    /// This region **must** be remmaped before calling [remap()](FrameAllocator::remap) and no allocations can be done in between.
    /// 
    /// The addresses are virtual and so, they change from before higher half remapping to after it.
    fn metadata_memory_range(&self) -> Option<ProhibitedMemoryRange>;
}

// TODO: a good idea would be to create a simple mechanism that would allow an easy way to switch the frame allocator
// even different allocators for different tests and "runners"
// pub static FRAME_ALLOCATOR: BitmapFrameAllocator = BitmapFrameAllocator::new();

/// The global frame allocator.
pub struct GlobalFrameAllocator {
    fa: Option<&'static dyn FrameAllocator>,
}

impl GlobalFrameAllocator {
    pub const fn new(fa: &'static dyn FrameAllocator) -> Self {
        GlobalFrameAllocator {
            fa: Some(fa)
        }
    }
}

impl Deref for GlobalFrameAllocator {
    type Target = dyn FrameAllocator;

    fn deref(&self) -> &Self::Target {
        self.fa.unwrap()
    }
}
