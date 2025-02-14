use core::alloc::GlobalAlloc;

pub struct SimplePageAllocator;

#[global_allocator]
static SIMPLE_PAGE_ALLOCATOR: SimplePageAllocator = SimplePageAllocator;

unsafe impl GlobalAlloc for SimplePageAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        todo!()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        todo!()
    }
}
