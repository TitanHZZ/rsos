// https://wiki.osdev.org/Task_State_Segment
use crate::memory::{pages::{Page, page_table::page_table_entry::EntryFlags}, MemoryError, simple_heap_allocator::HEAP_ALLOCATOR};
use crate::memory::{VirtualAddress, FRAME_PAGE_SIZE, MEMORY_SUBSYSTEM};
use core::{alloc::{GlobalAlloc, Layout}, arch::asm};
use super::gdt::SegmentSelector;

// TODO: this should probably not use the heap to allocate stacks but instead, the page allocator

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

    // this is not part of the structure but it is needed as metadata fot managing heap allocated stacks
    // this holds the size (in bytes) for each of tuse paging::ActivePagingContext;he currently allocated stacks and if they used a page guard
    previous_stack: [(usize, bool); 7],
}

// the TSS always has a constant size
pub const TSS_SIZE: u32 = 0x68;

impl Default for TSS {
    fn default() -> Self {
        Self::new()
    }
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

#[derive(Debug)]
pub enum TssError {
    PageCountIsZero,
    Memory(MemoryError),
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

            previous_stack: [(0, false); 7],
        }
    }

    /// Allocates a new stack on the heap to be used for interrupts with `page_count` pages and an optional guard page with some notes:
    /// - `page_count` needs to be at least 1 or `Err(TssError::PageCountIsZero)` will be returned.
    /// - the guard page is not part of the `page_count` meaning that if a page guard is used, the real page count allocated will be page_count + 1
    pub fn new_stack(&mut self, stack_number: TssStackNumber, page_count: u8, use_guard_page: bool) -> Result<(), TssError> {
        let active_paging_context = MEMORY_SUBSYSTEM.active_paging_context();

        // minimum 1 page for the stack
        if page_count == 0 {
            return Err(TssError::PageCountIsZero);
        }

        // this is a u16 to avoid overflows
        let real_page_count = page_count as u16 + use_guard_page as u16;

        // if a stack is already present, remove it so that a new can can be placed there
        if self.ist[stack_number as usize] != 0 {
            let previous_stack_size: usize    = self.previous_stack[stack_number as usize].0;
            let previous_stack_layout: Layout = Layout::from_size_align(previous_stack_size, FRAME_PAGE_SIZE).unwrap();
            let previous_stack_ptr: *mut u8   = (self.ist[stack_number as usize] - previous_stack_size + 1)  as *mut u8;

            // if the previous stack used a guard page, we need to map it again
            if self.previous_stack[stack_number as usize].1 {
                let guard_page_addr = previous_stack_ptr as VirtualAddress;
                let flags = EntryFlags::PRESENT | EntryFlags::WRITABLE | EntryFlags::NO_EXECUTE;
                active_paging_context.map(guard_page_addr, flags).map_err(TssError::Memory)?;
            }

            unsafe { HEAP_ALLOCATOR.dealloc(previous_stack_ptr, previous_stack_layout) };
        }

        // alocate enough memory for the stack
        let size = real_page_count as usize * FRAME_PAGE_SIZE;
        let layout = Layout::from_size_align(size, FRAME_PAGE_SIZE).unwrap();
        let stack = unsafe { HEAP_ALLOCATOR.alloc(layout) } as VirtualAddress;

        // in x86_64, the stack grows downwards so, it must point to the last stack byte
        self.ist[stack_number as usize] = (stack + real_page_count as usize * FRAME_PAGE_SIZE) - 1;
        self.previous_stack[stack_number as usize] = (size, use_guard_page);

        if use_guard_page {
            // the unwrap() **should** be fine as the addr was returned from the allocator itself
            active_paging_context.unmap_page(Page::from_virt_addr(stack).unwrap(), true).map_err(TssError::Memory)?;
        }

        Ok(())
    }

    /// Loads `tss_sel` as the current TSS.
    /// 
    /// # Safety
    /// 
    /// The caller must ensure that `tss_sel` is valid and that the TSS and GDT are also valid.
    // https://wiki.osdev.org/Task_State_Segment#TSS_in_software_multitasking
    pub unsafe fn load(tss_sel: SegmentSelector) {
        unsafe {
            asm! (
                "mov rax, {sel}",
                "ltr ax",
                sel = in(reg) tss_sel.as_u16() as u64,
            )
        }
    }
}
