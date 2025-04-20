#![no_std]
#![no_main]
#![feature(lazy_get)]
#![feature(custom_test_frameworks)]
#![test_runner(rsos::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use rsos::{vga_buffer, logger};

use rsos::memory::{frames::simple_frame_allocator::FRAME_ALLOCATOR, pages::paging::{inactive_paging_context::InactivePagingContext, ACTIVE_PAGING_CTX}};
use rsos::multiboot2::{elf_symbols::{ElfSectionFlags, ElfSymbols, ElfSymbolsIter}, memory_map::MemoryMap, MbBootInfo};
use rsos::memory::{{FRAME_PAGE_SIZE, pages::{Page, simple_page_allocator::HEAP_ALLOCATOR}}, AddrOps};
use rsos::{log, memory, print, println};
use alloc::boxed::Box;
use core::cmp::max;

// fn print_mem_status(mb_info: &MbBootInfo) {
//     let mem_map = mb_info.get_tag::<MemoryMap>().expect("Mem map tag is not present.");
//     let mem_map_entries = mem_map.entries().expect("Only 64bit mem map entries are supported.");
//     println!("Memory areas:");
//     for entry in mem_map_entries {
//         println!(
//             "\tstart: 0x{:x}, length: {:.2} MB, type: {:?}",
//             entry.base_addr,
//             entry.length as f64 / 1024.0 / 1024.0,
//             entry.entry_type()
//         );
//     }
//     let total_memory: u64 = mem_map_entries.into_iter()
//         .filter(|entry| entry.entry_type() == MemoryMapEntryType::AvailableRAM)
//         .map(|entry| entry.length)
//         .sum();
//     println!(
//         "Total (available) memory: {} bytes ({:.2} GB)",
//         total_memory,
//         total_memory as f64 / 1024.0 / 1024.0 / 1024.0
//     );
// }

// TODO: build tests
// TODO: look into stack probes
// TODO: double check the section permissions on the linker script
#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(mb_boot_info_addr: *const u8) -> ! {
    // at this point, the cpu is running in 64 bit long mode
    // paging is enabled (including the NXE and WP bits) and we are using identity mapping
    log!(ok, "Rust kernel code started.");
    let mb_info = unsafe { MbBootInfo::new(mb_boot_info_addr) }.expect("Invalid mb2 data");

    // get the necessary mb2 tags and data
    let mem_map: &MemoryMap          = mb_info.get_tag::<MemoryMap>().expect("Memory map tag is not present");
    let elf_symbols: &ElfSymbols     = mb_info.get_tag::<ElfSymbols>().expect("Elf symbols tag is not present");
    let elf_sections: ElfSymbolsIter = elf_symbols.sections().expect("Elf sections are invalid");

    // get the kernel start and end addrs
    let k_start = elf_sections.filter(|s: _| s.flags().contains(ElfSectionFlags::ELF_SECTION_ALLOCATED))
        .map(|s: _| s.addr()).min().expect("Elf sections is empty").align_down(FRAME_PAGE_SIZE);

    let k_end   = elf_sections.filter(|s: _| s.flags().contains(ElfSectionFlags::ELF_SECTION_ALLOCATED))
        .map(|s: _| s.addr() + s.size() as usize).max().expect("Elf sections is empty").align_up(FRAME_PAGE_SIZE) - 1;

    // get the mb2 info start and end addrs
    let mb_start = mb_info.addr().align_down(FRAME_PAGE_SIZE);
    let mb_end   = mb_start + mb_info.size() as usize - 1;
    let mb_end   = mb_end.align_up(FRAME_PAGE_SIZE) - 1;

    // set up the frame allocator
    let mem_map_entries = mem_map.entries().expect("Memory map entries are invalid").0;
    unsafe {
        FRAME_ALLOCATOR.init(mem_map_entries, k_start, k_end, mb_start, mb_end).expect("Could not initialize the frame allocator");
        log!(ok, "Frame allocator initialized.");
    }

    // get the current paging context and create a new (empty) one
    log!(ok, "Remapping the kernel memory, vga buffer and mb2 info.");
    { // this scope makes sure that the inactive context does not get used again
        let inactive_paging = &mut InactivePagingContext::new(&ACTIVE_PAGING_CTX, &FRAME_ALLOCATOR).unwrap();

        // remap (identity map) the kernel, mb2 info and vga buffer with the correct flags and permissions into the new paging context
        memory::kernel_remap(&ACTIVE_PAGING_CTX, inactive_paging, elf_sections, &FRAME_ALLOCATOR, &mb_info)
            .expect("Could not remap the kernel");
        ACTIVE_PAGING_CTX.switch(inactive_paging);

        // TODO: is this really necessary?
        // the unwrap is fine as we know that the addr is valid
        ACTIVE_PAGING_CTX.unmap_page(Page::from_virt_addr(inactive_paging.p4_frame().addr()).unwrap(), &FRAME_ALLOCATOR);
    }

    // at this point, we are using a new paging context that just identity maps the kernel, mb2 info and vga buffer
    // the paging context created during the asm bootstrapping is now being used as stack for the kernel
    // except for the p4 table that is being used as a guard page
    // because of this, we now have just over 2MiB of stack

    log!(ok, "Kernel remapping completed.");
    log!(ok, "Stack guard page created.");

    // set up the heap allocator
    unsafe {
        // we know that the addr of the vga buffer and the start of the kernel will never change at runtime
        // and that the addr of the kernel is bigger so, we only need to avoid the mb2 info struct
        // and thus, we can start the kernel heap at the biggest of the 2
        HEAP_ALLOCATOR.init(max(k_end, mb_end).align_up(FRAME_PAGE_SIZE), 100 * 1024, &ACTIVE_PAGING_CTX)
            .expect("Could not initialize the heap allocator");
        log!(ok, "Heap allocator initialized.");
    }

    {
        let a = Box::new(Aligned16(10));
        // let b = String::from("Hello, World!");
        println!("{:?}", a);
        // println!("{}", b);
    }

    #[cfg(test)]
    test_main();

    loop {}
}

#[derive(Debug)]
#[repr(align(16))]
struct Aligned16(u64);

#[test_case]
fn basic_assert() {
    assert_eq!(1, 1);
}

/*
 * Current physical memory layout (NOT UP TO DATE):
 * 
 * +--------------------+ (higher addresses)
 * |      Unused        |
 * +--------------------+ 0x513000
 * |                    |
 * |      Kernel        |
 * |                    |
 * +--------------------+ 0x200000
 * |                    |
 * |   Multiboot Info   |
 * |                    |
 * +--------------------+ 0x1FF000
 * |      Unused        |
 * +--------------------+ 0x0B9000
 * |                    |
 * |    VGA Buffer      |
 * |                    |
 * +--------------------+ 0x0B8000
 * |      Unused        |
 * +--------------------+ 0x000000
 */
