use crate::memory::frames::{bitmap_frame_allocator::BitmapFrameAllocator, GlobalFrameAllocator};

// pub static FRAME_ALLOCATOR: BitmapFrameAllocator = BitmapFrameAllocator::new();
static BITMAP_FA: BitmapFrameAllocator = BitmapFrameAllocator::new();
pub static FRAME_ALLOCATOR: GlobalFrameAllocator = GlobalFrameAllocator::new(&BITMAP_FA);
