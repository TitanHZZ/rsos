use crate::{memory::{frames::{bitmap_frame_allocator::BitmapFrameAllocator, GlobalFrameAllocator}}};
use crate::memory::pages::paging::ActivePagingContext;

// the frame allocator
static FA: BitmapFrameAllocator = BitmapFrameAllocator::new();
pub static FRAME_ALLOCATOR: GlobalFrameAllocator = GlobalFrameAllocator::new(&FA);

// // the page allocator
// // TODO: needs a system to switch to the second stage page allocator
// static FIRST_STAGE_PA: TemporaryPageAllocator = TemporaryPageAllocator::new(ORIGINALLY_IDENTITY_MAPPED);
// static SECOND_STAGE_PA: BitmapPageAllocator = BitmapPageAllocator::new();
// pub static PAGE_ALLOCATOR: GlobalPageAllocator = GlobalPageAllocator::new(&FIRST_STAGE_PA);

// the paging system
pub static ACTIVE_PAGING_CTX: ActivePagingContext = ActivePagingContext::new();

// Problems:
// - i need a way to switch from the temporary page allocator to the permanent one
// - what if i have multiple types of permanent/temporary page allocators?? should i have a system to support them all and allow switching
//   in at least one direction?? (temporary to permanent)
// - for frame allocators, i do not (currently) have the concept of a temporary and permanent frame allocator so,
//   should i still have a system to change them at runtime?? or is changing them at compile time enough?
// - I need to be able to use different frame allocators. i do not need to change them at runtime like the page allocators (at least for now) but,
//   i need to change them at compile time to test each individual frame allocator (changing at runtime and then testing them is also fine)
// - i would like to have the global "stuff" in a "single place" in a way that, at least the declarations for the global statics are close together.
//   this would avoid having global statics scattered all around the codebase (but, is this even a problem/thing i should worry about??)
// - assuming that i allow for runtime changes (which i kind of need for page allocators), how would that be handled? i do not like the idea of
//   allocators (page or frame allocators) being created and destroyed at any point and anywhere in the kernel code. that sounds like recipe for
//   disaster as the metadata for allocators might/will conflict with currently being used allocators so, i would like that to be controlled
//   but, how can i do that?? should i have an instance of every type of allocator created and then only allow for use of a single one
//   (blocking the creation of new ones) and making sure that i could only go from temporary allocators to permanent in the case of page allocators??
