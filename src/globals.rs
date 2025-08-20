use crate::{memory::{frames::{bitmap_frame_allocator::BitmapFrameAllocator, GlobalFrameAllocator}}};
use crate::memory::pages::paging::ActivePagingContext;

// the frame allocator
static FA: BitmapFrameAllocator = BitmapFrameAllocator::new();
pub static FRAME_ALLOCATOR: GlobalFrameAllocator = GlobalFrameAllocator::new(&FA);

// the paging system
pub static ACTIVE_PAGING_CTX: ActivePagingContext = ActivePagingContext::new();
