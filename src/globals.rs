use crate::{memory::{frames::{bitmap_frame_allocator::BitmapFrameAllocator, GlobalFrameAllocator}}};

// the frame allocator
static FA: BitmapFrameAllocator = BitmapFrameAllocator::new();
pub static FRAME_ALLOCATOR: GlobalFrameAllocator = GlobalFrameAllocator::new(&FA);
