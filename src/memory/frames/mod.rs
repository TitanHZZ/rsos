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

// TODO: look into more advanced docs
// TODO: fix the prohibited_memory_range name and description
/// Represents a frame allocator to be used OS wide.
/// 
/// # Safety
/// 
/// A correct implementation must follow these rules:
/// 
/// - The client **should** be created very early on, and it should, preferably, be static.
/// - [init()](FrameAllocator::init()) **must** be called very early on, preferably before remapping the [`Kernel`] and multiboot2 to the higher half.
/// - The frame allocator must ensure that the [`Kernel`] prohibited memory ranges are **never** violated.
/// - Only valid RAM can be used for metadata.
/// - No more than one frame allocator is expected to ever exist at runtime.
/// - The allocator may rely on [`ORIGINALLY_IDENTITY_MAPPED`] for its metadata that **needs** to later be remapped with [remap()](FrameAllocator::remap()).
/// - The use of a [Page Allocator](crate::memory::pages::PageAllocator) is **prohibited** to ensure that no recursive state is reached.
pub unsafe trait FrameAllocator: Send + Sync {
    fn allocate_frame(&self) -> Result<Frame, MemoryError>;
    fn deallocate_frame(&self, frame: Frame);

    /// Initializes/Resets the frame allocator state.
    /// 
    /// # Safety
    /// 
    /// This must be called (before any allocation) as the allocator expects it.
    /// In the case that it does not get called, memory corruption is the most likely outcome.
    unsafe fn init(&self, kernel: &Kernel) -> Result<(), MemoryError>;

    /// Remaps the frame allocator to use higher half mapping with its underlying control structure.
    /// 
    /// # Safety
    /// 
    /// This **does not** recreate the frame allocator, it simply adjusts the internal metadata/control structure so,
    /// if the new mapping is wrong, this *will* result in **undefined behavior**.
    /// 
    /// **Must** be called right after changing the mapping and before any more frame allocations to avoid problems.
    unsafe fn remap(&self, kernel: &Kernel);

    /// Get the physical memory region that **MUST** be correctly mapped and cannot be used for allocations by other frame allocators.
    /// 
    /// The addresses are virtual and so, they change from before higher half remapping to after it.
    fn prohibited_memory_range(&self) -> Option<ProhibitedMemoryRange>;
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
