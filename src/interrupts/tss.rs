// https://wiki.osdev.org/Task_State_Segment
use crate::memory::{frames::simple_frame_allocator::FRAME_ALLOCATOR, pages::{paging::ACTIVE_PAGING_CTX, simple_page_allocator::HEAP_ALLOCATOR, Page}, VirtualAddress, FRAME_PAGE_SIZE};
use core::alloc::{GlobalAlloc, Layout};

// https://wiki.osdev.org/Task_State_Segment#Long_Mode
#[repr(C, packed)]
pub struct TSS {
    reserved_0: u32,
    rsp0: VirtualAddress,
    rsp1: VirtualAddress,
    rsp2: VirtualAddress,
    reserved_1: u32,
    reserved_2: u32,
    ist: [VirtualAddress; 7],
    reserved_3: u32,
    reserved_4: u32,
    reserved_5: u16,
    iopb: u16,
}

#[derive(Clone, Copy)]
pub enum TssStackNumber {
    TssStack1 = 0,
    TssStack2 = 1,
    TssStack3 = 2,
    TssStack4 = 3,
    TssStack5 = 4,
    TssStack6 = 5,
    TssStack7 = 6,
}

impl TSS {
    /// Creates a new, completly zeroed out, TSS struct.
    pub const fn new() -> Self {
        TSS {
            reserved_0: 0,
            rsp0: 0,
            rsp1: 0,
            rsp2: 0,
            reserved_1: 0,
            reserved_2: 0,
            ist: [0; 7],
            reserved_3: 0,
            reserved_4: 0,
            reserved_5: 0,
            iopb: 0,
        }
    }

    // TODO: fix the deallocation (might need rework of this fn as we do not know the size of the previous stack)
    pub fn new_stack(&mut self, stack_number: TssStackNumber, mut page_count: u8, use_guard_page: bool) {
        // minimum 1 page for the stack
        if page_count == 0 {
            page_count += 1;
        }

        // this is a u16 to avoid overflows
        let real_page_count = page_count as u16 + use_guard_page as u16;

        // if a stack is already present, remove it so that a new can can be placed there
        if self.ist[stack_number as usize] != 0 {
            unimplemented!();
        }

        // alocate enough memory for the stack
        // TODO: maybe a constant for the page align??
        // TODO: remove this unwrap() call
        let layout = Layout::from_size_align(real_page_count as usize * FRAME_PAGE_SIZE, 4096).unwrap();
        let stack = unsafe { HEAP_ALLOCATOR.alloc(layout) };

        // in x86_64, the stack grows downwards so, it must point to the last stack byte
        self.ist[stack_number as usize] = (stack as usize + real_page_count as usize * FRAME_PAGE_SIZE) - 1;

        if use_guard_page {
            // the unwrap() **should** be fine as the addr was returned from the allocator itself
            ACTIVE_PAGING_CTX.unmap_page(Page::from_virt_addr(stack as VirtualAddress).unwrap(), &FRAME_ALLOCATOR);
        }
    }
}
