pub mod pages;
pub mod frames;
mod cr3;

use pages::{page_table::page_table_entry::EntryFlags, paging::{inactive_paging_context::InactivePagingContext, ActivePagingContext}};
use crate::{kernel::Kernel, multiboot2::{elf_symbols::{ElfSectionFlags, ElfSymbols}}};
use frames::{Frame, FrameAllocator};

// the size of the pages and frames
pub const FRAME_PAGE_SIZE: usize = 4096;

pub type PhysicalAddress = usize;
pub type VirtualAddress = usize;

pub trait AddrOps {
    fn align_down(&self, align: usize) -> usize;

    fn align_up(&self, align: usize) -> usize;
}

// this implements AddrsOps for both VirtualAddress and PhysicalAddress
impl AddrOps for usize {
    fn align_down(&self, align: usize) -> usize {
        debug_assert!(align.is_power_of_two());
        *self & !(align - 1)
    }

    fn align_up(&self, align: usize) -> usize {
        debug_assert!(align.is_power_of_two());
        (*self + align - 1) & !(align - 1)
    }
}

#[derive(Debug)]
pub enum MemoryError {
    PageInvalidVirtualAddress, // tried creating a page with an invalid x86_64 addr
    NotEnoughPhyMemory,        // a frame allocator ran out of memory
    MisalignedKernelSection,   // a kernel ELF section that is not FRAME_PAGE_SIZE aligned
    MappingUsedTableEntry,     // the user is trying to map to a used page table entry
    FrameInvalidAllocatorAddr, // the allocator gave an addr that is not FRAME_PAGE_SIZE aligned
}

/*
 * Remaps (identity maps) the kernel, vga buffer and multiboot2 info into an InactivePagingContext.
 * If nothing goes wrong, it *should* be safe to switch to the InactivePagingContext afterwards.
 */
pub fn kernel_remap<A>(kernel: &Kernel, ctx: &ActivePagingContext, new_ctx: &InactivePagingContext, fr_alloc: &A) -> Result<(), MemoryError>
where
    A: FrameAllocator
{    
    ctx.update_inactive_context(new_ctx, fr_alloc, |active_ctx, frame_allocator| {
        let elf_symbols: &ElfSymbols = kernel.mb_info().get_tag::<ElfSymbols>().expect("elf_symbols broke");
        let elf_sections = elf_symbols.sections().expect("elf_sections broke");

        for elf_section in elf_sections {
            // if the section is not in memory, we don't need to map it
            if !elf_section.flags().contains(ElfSectionFlags::ELF_SECTION_ALLOCATED) {
                continue;
            }

            // get section addr range (from first byte of first frame to last byte of last frame)
            let start_addr = elf_section.addr();
            let end_addr = start_addr + elf_section.size() as usize - 1;
            let end_addr = end_addr.align_up(FRAME_PAGE_SIZE) - 1;

            // make sure that kernel elf sections are FRAME_PAGE_SIZE aligned
            if start_addr % FRAME_PAGE_SIZE != 0 {
                return Err(MemoryError::MisalignedKernelSection);
            }

            // identity map every section
            for addr in (start_addr..=end_addr).step_by(FRAME_PAGE_SIZE) {
                let frame = Frame::from_phy_addr(addr);
                let flags = EntryFlags::from_elf_section_flags(elf_section.flags());
                active_ctx.identity_map(frame, frame_allocator, flags)?;
            }
        }

        // identity map the vga buffer
        let vga_buff_frame = Frame::from_phy_addr(0xb8000);
        let flags = EntryFlags::PRESENT | EntryFlags::WRITABLE | EntryFlags::NO_EXECUTE;
        active_ctx.identity_map(vga_buff_frame, frame_allocator, flags)?;

        // identity map the multiboot2 info
        for addr in (kernel.mb_start()..=kernel.mb_end()).step_by(FRAME_PAGE_SIZE) {
            let frame = Frame::from_phy_addr(addr);
            active_ctx.identity_map(frame, frame_allocator, EntryFlags::PRESENT | EntryFlags::NO_EXECUTE)?;
        }

        Ok(())
    })
}

// // the unwraps() here are fine as we are just testing things
// pub fn test_paging<A: FrameAllocator>(frame_allocator: &mut A) {
//     let mut page_table = unsafe { ActivePagingContext::new() };
//     let virt_addr = 42 * 512 * 512 * FRAME_PAGE_SIZE; // 42 th entry in p3
//     let page = Page::from_virt_addr(virt_addr).unwrap();
//     let frame = frame_allocator.allocate_frame().expect("out of memory");
//     println!("None = {:?}, map to {:?}", page_table.translate(virt_addr), frame);
//     page_table.map_page_to_frame(page, frame, frame_allocator, EntryFlags::empty()).unwrap();
//     println!("Some = {:?}", page_table.translate(virt_addr));
//     println!("next free frame: {:?}", frame_allocator.allocate_frame());
//     println!("-------------------");
//     println!("virt addr contents: {:#x}", unsafe { *(Page::from_virt_addr(virt_addr).unwrap().addr() as *const u64) });
//     page_table.unmap_page(Page::from_virt_addr(virt_addr).unwrap(), frame_allocator);
//     // println!("virt addr contents: {:#x}", unsafe { *(Page::from_virt_addr(virt_addr).addr() as *const u64) });
//     println!("None = {:?}", page_table.translate(virt_addr));
// }
