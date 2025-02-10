#![no_std]
#![no_main]
#![feature(lazy_get)]

mod multiboot2;
mod vga_buffer;
mod memory;

use memory::{frames::simple_frame_allocator::SimpleFrameAllocator, pages::paging::{inactive_paging_context::InactivePagingContext, ActivePagingContext}};
use multiboot2::{elf_symbols::ElfSymbols, memory_map::{MemoryMap, MemoryMapEntryType}, MbBootInfo};
use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

fn print_mem_status(mb_info: &MbBootInfo) {
    let mem_map = mb_info.get_tag::<MemoryMap>().expect("Mem map tag is not present.");
    let mem_map_entries = mem_map.entries().expect("Only 64bit mem map entries are supported.");

    println!("Memory areas:");
    for entry in mem_map_entries {
        println!(
            "\tstart: 0x{:x}, length: {:.2} MB, type: {:?}",
            entry.base_addr,
            entry.length as f64 / 1024.0 / 1024.0,
            entry.entry_type()
        );
    }

    let total_memory: u64 = mem_map_entries.into_iter()
        .filter(|entry| entry.entry_type() == MemoryMapEntryType::AvailableRAM)
        .map(|entry| entry.length)
        .sum();
    println!(
        "Total (available) memory: {} bytes ({:.2} GB)",
        total_memory,
        total_memory as f64 / 1024.0 / 1024.0 / 1024.0
    );
}

#[no_mangle]
pub extern "C" fn main(mb_boot_info_addr: *const u8) -> ! {
    let mb_info = unsafe { MbBootInfo::new(mb_boot_info_addr) }.expect("Invalid mb2 data.");
    print_mem_status(&mb_info);

    let mem_map = mb_info.get_tag::<MemoryMap>().expect("Memory map tag is not present.");
    let elf_symbols = mb_info.get_tag::<ElfSymbols>().expect("Elf symbols tag is not present.");
    let elf_sections = elf_symbols.sections().expect("Elf sections are invalid.");

    let k_start = elf_sections
        .map(|s| s.addr())
        .min()
        .expect("Elf sections is empty.") as usize;

    let k_end = elf_sections
        .map(|s| s.addr())
        .min()
        .expect("Elf sections is empty.") as usize;

    let mb_start = mb_boot_info_addr as usize;
    let mb_end = mb_start + mb_info.size() as usize;

    // --------------- PAGING TESTS ---------------

    let mem_map_entries = mem_map.entries().expect("Memory map entries are invalid.").0;
    let frame_allocator: _ = &mut SimpleFrameAllocator::new(mem_map_entries, k_start, k_end, mb_start, mb_end).expect("");

    // memory::test_paging(frame_allocator);

    // --------------- KERNEL REMAP TESTS ---------------

    let active_paging = unsafe { &mut ActivePagingContext::new() };
    let inactive_paging = &InactivePagingContext::new(active_paging, frame_allocator).unwrap();

    memory::kernel_remap(active_paging, inactive_paging, elf_sections, frame_allocator).unwrap();

    loop {}
}
