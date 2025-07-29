pub mod simple_heap_allocator;
pub mod pages;
pub mod frames;
mod cr3;

use crate::{kernel::Kernel, memory::{frames::FRAME_ALLOCATOR, pages::{Page, PageAllocator}}, multiboot2::elf_symbols::{ElfSectionError, ElfSectionFlags, ElfSymbols}};
use pages::{page_table::page_table_entry::EntryFlags, paging::{inactive_paging_context::InactivePagingContext, ActivePagingContext}};
use crate::multiboot2::memory_map::MemoryMapError;
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
        assert!(align.is_power_of_two());
        *self & !(align - 1)
    }

    fn align_up(&self, align: usize) -> usize {
        assert!(align.is_power_of_two());
        (*self + align - 1) & !(align - 1)
    }
}

/// Represents a memory range that MUST not be touched by frame/page allocators.
/// 
/// This memory regions are expected to be identity mapped and as such, these addrs are virtual and physical.
/// 
/// The addrs are both inclusive. The start address is guaranteed to be **0** or a multiple of **FRAME_PAGE_SIZE**, while the end address is guaranteed
/// to be **0** or else, the **end address + 1** is a multiple of **FRAME_PAGE_SIZE**.
#[derive(Clone, Copy)]
pub struct ProhibitedMemoryRange {
    start_addr: PhysicalAddress,
    end_addr: PhysicalAddress,
}

impl ProhibitedMemoryRange {
    /// Creates a `ProhibitedMemoryRange`.
    pub const fn new(start_addr: PhysicalAddress, end_addr: PhysicalAddress) -> ProhibitedMemoryRange {
        assert!(start_addr.is_multiple_of(FRAME_PAGE_SIZE));
        assert!((end_addr == 0) || (end_addr + 1).is_multiple_of(FRAME_PAGE_SIZE));

        ProhibitedMemoryRange {
            start_addr,
            end_addr,
        }
    }

    /// Creates a `ProhibitedMemoryRange` with both addrs as 0.
    pub const fn empty() -> ProhibitedMemoryRange {
        ProhibitedMemoryRange::new(0, 0)
    }

    pub fn start_addr(&self) -> PhysicalAddress {
        self.start_addr
    }

    pub fn end_addr(&self) -> PhysicalAddress {
        self.end_addr
    }

    /// Get the prohibited range length in bytes.
    pub fn length(&self) -> usize {
        self.end_addr - self.start_addr + 1
    }

    /// Get the prohibited range length in frames.
    pub fn frame_length(&self) -> usize {
        self.length().align_up(FRAME_PAGE_SIZE) / FRAME_PAGE_SIZE
    }
}

#[derive(Debug, PartialEq)]
pub enum MemoryError {
    /// Tried creating a page with an invalid x86_64 addr (non canonical address).
    PageInvalidVirtualAddress,
    /// A frame allocator ran out of memory.
    NotEnoughPhyMemory,
    /// A page allocator ran out of memory.
    NotEnoughVirMemory,
    /// A kernel ELF section that is not FRAME_PAGE_SIZE aligned.
    MisalignedKernelSection,
    /// The user is trying to map to a used page table entry.
    MappingUsedTableEntry,
    /// The user is trying to unmap an unused page table entry.
    UnmappingUnusedTableEntry,
    /// The allocator gave an addr that is not FRAME_PAGE_SIZE aligned.
    FrameInvalidAllocatorAddr,
    /// The kernel was placed in a way that overlaps with memory rigions that are not `AvailableRAM`.
    BadMemoryPlacement,
    /// The start address given to the temporary page allocator conflicts with other mappings.
    BadTemporaryPageAllocator,

    // TODO: perhaps these should be considered multiboot2 errors??
    /// The `ElfSymbols` multiboot2 tag does not exist.
    ElfSymbolsMbTagDoesNotExist,
    /// The `MemoryMap` multiboot2 tag does not exist.
    MemoryMapMbTagDoesNotExist,
    /// Elf section specific errors.
    ElfSectionErr(ElfSectionError),
    /// Elf section specific errors.
    MemoryMapErr(MemoryMapError),
}

/// Remaps (to the higher half) the kernel, the multiboot2 info and the prohibited memory regions
/// from the frame allocator into an InactivePagingContext.
pub fn remap<F, P>(kernel: &Kernel, ctx: &ActivePagingContext, new_ctx: &InactivePagingContext, fa: &F, pa: &P) -> Result<(), MemoryError>
where
    F: FrameAllocator,
    P: PageAllocator,
{
    ctx.update_inactive_context(new_ctx, fa, pa, |active_ctx, frame_allocator| {
        // get the kernel elf sections
        let elf_symbols = kernel.mb_info().get_tag::<ElfSymbols>().ok_or(MemoryError::ElfSymbolsMbTagDoesNotExist)?;
        let elf_sections = elf_symbols.sections().map_err(MemoryError::ElfSectionErr)?;

        // remap the kernel (just the allocated sections)
        for elf_section in elf_sections.filter(|s| s.flags().contains(ElfSectionFlags::ELF_SECTION_ALLOCATED)) {
            // get section addr range (from first byte of first frame to last byte of last frame)
            let start_addr = elf_section.load_addr();
            let end_addr   = start_addr + elf_section.size() as usize - 1;
            let end_addr   = end_addr.align_up(FRAME_PAGE_SIZE) - 1;

            // make sure that kernel elf sections are FRAME_PAGE_SIZE aligned
            if !start_addr.is_multiple_of(FRAME_PAGE_SIZE) {
                return Err(MemoryError::MisalignedKernelSection);
            }

            // map every section to the kernel higher half (defined in the linker script)
            for addr in (start_addr..=end_addr).step_by(FRAME_PAGE_SIZE) {
                let frame = Frame::from_phy_addr(addr);
                let page = Page::from_virt_addr(addr + Kernel::k_lh_hh_offset())?;
                let flags = EntryFlags::from_elf_section_flags(elf_section.flags());
                active_ctx.map_page_to_frame(page, frame, frame_allocator, flags)?;
            }
        }

        // higher half map the multiboot2 info
        let mb2_lh_hh_offset = kernel.mb2_lh_hh_offset();
        for addr in (kernel.mb_start()..=kernel.mb_end()).step_by(FRAME_PAGE_SIZE) {
            let frame = Frame::from_phy_addr(addr);
            let page = Page::from_virt_addr(addr + mb2_lh_hh_offset)?;
            active_ctx.map_page_to_frame(page, frame, frame_allocator, EntryFlags::PRESENT | EntryFlags::NO_EXECUTE)?;
        }

        // higher half map the frame allocator prohibited physical memory region
        if FRAME_ALLOCATOR.prohibited_memory_range().is_none() {
            return Ok(());
        }

        let fa_lh_hh_offset = kernel.fa_lh_hh_offset();
        let prohibited_mem_range = FRAME_ALLOCATOR.prohibited_memory_range().unwrap();
        for addr in (prohibited_mem_range.start_addr()..=prohibited_mem_range.end_addr()).step_by(FRAME_PAGE_SIZE) {
            let frame = Frame::from_phy_addr(addr);
            let page = Page::from_virt_addr(addr + fa_lh_hh_offset)?;
            active_ctx.map_page_to_frame(page, frame, frame_allocator, EntryFlags::PRESENT | EntryFlags::WRITABLE | EntryFlags::NO_EXECUTE)?;
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
