pub mod pages;
pub mod frames;
mod cr3;

use pages::{page_table::page_table_entry::EntryFlags, paging::{inactive_paging_context::InactivePagingContext, ActivePagingContext}, Page};
use crate::{multiboot2::elf_symbols::{ElfSectionFlags, ElfSymbolsIter}, print, println};
use frames::{Frame, FrameAllocator};

// the size of the pages and frames
const FRAME_PAGE_SIZE: usize = 4096;

pub type PhysicalAddress = usize;
pub type VirtualAddress = usize;

#[derive(Debug)]
pub enum MemoryError {
    PageInvalidVirtualAddress, // tried creating a page with an invalid x86_64 addr
    NotEnoughPhyMemory,        // a frame allocator ran out of memory
}

// TODO: use the correct entry flags for appropriate page permissions
pub fn kernel_remap<A>(ctx: &mut ActivePagingContext, new_ctx: &InactivePagingContext, elf_secs: ElfSymbolsIter, fr_alloc: &mut A) -> Result<(), MemoryError>
where
    A: FrameAllocator
{
    ctx.update_inactive_context(&new_ctx, fr_alloc, |active_ctx, frame_allocator| {
        for elf_section in elf_secs {
            // if the section is not in memory, we don't need to map it
            if !elf_section.flags().contains(ElfSectionFlags::ELF_SECTION_ALLOCATED) {
                continue;
            }

            // get section addr range
            let start_addr = elf_section.addr();
            let end_addr = start_addr + elf_section.size() - 1;

            // this is assert!() and not debug_assert!() because we need to make sure that no matter the compiler or the linker,
            // we always get FRAME_PAGE_SIZE aligned kernel sections
            assert!(start_addr % FRAME_PAGE_SIZE as u64 == 0, "The kernel sections are not {} aligned.", FRAME_PAGE_SIZE);

            // identity map every section
            for addr in (start_addr..=end_addr).step_by(FRAME_PAGE_SIZE) {
                let frame = Frame::from_phy_addr(addr as _);
                active_ctx.identity_map(frame, frame_allocator, EntryFlags::PRESENT | EntryFlags::WRITABLE)?;
            }
        }

        Ok(())
    })
}


// the unwraps() here are fine as we are just testing things
pub fn test_paging<A: FrameAllocator>(frame_allocator: &mut A) {
    let mut page_table = unsafe { ActivePagingContext::new() };

    let virt_addr = 42 * 512 * 512 * FRAME_PAGE_SIZE; // 42 th entry in p3
    let page = Page::from_virt_addr(virt_addr).unwrap();
    let frame = frame_allocator.allocate_frame().expect("out of memory");

    println!("None = {:?}, map to {:?}", page_table.translate(virt_addr), frame);
    page_table.map_page_to_frame(page, frame, frame_allocator, EntryFlags::empty()).unwrap();
    println!("Some = {:?}", page_table.translate(virt_addr));
    println!("next free frame: {:?}", frame_allocator.allocate_frame());

    println!("-------------------");

    println!("virt addr contents: {:#x}", unsafe { *(Page::from_virt_addr(virt_addr).unwrap().addr() as *const u64) });
    page_table.unmap_page(Page::from_virt_addr(virt_addr).unwrap(), frame_allocator);
    // println!("virt addr contents: {:#x}", unsafe { *(Page::from_virt_addr(virt_addr).addr() as *const u64) });
    println!("None = {:?}", page_table.translate(virt_addr));
}
