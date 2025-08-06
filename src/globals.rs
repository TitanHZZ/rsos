use crate::memory::pages::{simple_page_allocator::BitmapPageAllocator, GlobalPageAllocator, temporary_page_allocator::TemporaryPageAllocator};
use crate::{kernel::ORIGINALLY_IDENTITY_MAPPED, memory::{frames::{bitmap_frame_allocator::BitmapFrameAllocator, GlobalFrameAllocator}}};
use crate::memory::pages::paging::ActivePagingContext;

// the frame allocator
static FA: BitmapFrameAllocator = BitmapFrameAllocator::new();
pub static FRAME_ALLOCATOR: GlobalFrameAllocator = GlobalFrameAllocator::new(&FA);

// the page allocator
// TODO: needs a system to switch to the second stage page allocator
static FIRST_STAGE_PA: TemporaryPageAllocator = TemporaryPageAllocator::new(ORIGINALLY_IDENTITY_MAPPED);
static SECOND_STAGE_PA: BitmapPageAllocator = BitmapPageAllocator::new();
pub static PAGE_ALLOCATOR: GlobalPageAllocator = GlobalPageAllocator::new(&FIRST_STAGE_PA);

// the paging system
pub static ACTIVE_PAGING_CTX: ActivePagingContext = ActivePagingContext::new();
