pub mod pages;
pub mod frames;

use pages::{page_table::page_table_entry::EntryFlags, paging::ActivePagingContext, Page};
use crate::{print, println};
use frames::FrameAllocator;

// the size of the pages and frames
const FRAME_PAGE_SIZE: usize = 4096;

pub type PhysicalAddress = usize;
pub type VirtualAddress = usize;

pub enum MemoryError {
    PageInvalidVirtualAddress, // tried creating a page with an invalid x86_64 addr
    NotEnoughPhyMemory,        // a frame allocator ran out of memory
}

pub fn test_paging<A: FrameAllocator>(frame_allocator: &mut A) {
//     let mut page_table = unsafe { ActivePagingContext::new() };
// 
//     let virt_addr = 42 * 512 * 512 * FRAME_PAGE_SIZE; // 42 th entry in p3
//     let page = Page::from_virt_addr(virt_addr);
//     let frame = frame_allocator.allocate_frame().expect("out of memory");
// 
//     println!("None = {:?}, map to {:?}", page_table.translate(virt_addr), frame);
//     page_table.map_page_to_frame(page, frame, frame_allocator, EntryFlags::empty());
//     println!("Some = {:?}", page_table.translate(virt_addr));
//     println!("next free frame: {:?}", frame_allocator.allocate_frame());
// 
//     println!("-------------------");
// 
//     println!("virt addr contents: {:#x}", unsafe { *(Page::from_virt_addr(virt_addr).addr() as *const u64) });
//     page_table.unmap_page(Page::from_virt_addr(virt_addr), frame_allocator);
//     // println!("virt addr contents: {:#x}", unsafe { *(Page::from_virt_addr(virt_addr).addr() as *const u64) });
//     println!("None = {:?}", page_table.translate(virt_addr));
}
