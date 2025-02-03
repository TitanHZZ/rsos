#![no_std]
#![no_main]

mod multiboot2;
mod vga_buffer;
mod memory;

use core::panic::PanicInfo;
use multiboot2::{elf_symbols::ElfSymbols, memory_map::{MemoryMap, MemoryMapEntryType}, MbBootInfo};
// use memory::{FrameAllocator, SimpleFrameAllocator};

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
            "    start: 0x{:x}, length: {:.2} MB, type: {:?}",
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

    let mem_map = mb_info.get_tag::<MemoryMap>().expect("Memory map tag is not present");
    let elf_symbols = mb_info.get_tag::<ElfSymbols>().expect("Elf symbols tag is not present");
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

    // let memory_map_tag = mb_info.memory_map_tag().expect("Memory map tag required");
    // let elf_sections_tag = mb_info.elf_sections().expect("Elf-sections tag required");
    // let kernel_start = elf_sections_tag
    //     .clone()
    //     .map(|s| s.start_address())
    //     .min()
    //     .unwrap() as usize;
    // let kernel_end = elf_sections_tag
    //     .clone()
    //     .map(|s| s.start_address() + s.size())
    //     .max()
    //     .unwrap() as usize;
    // let multiboot_start = mb_boot_info_addr;
    // let multiboot_end = multiboot_start + (mb_info.total_size() as usize);

    // let mut simple_frame_allocator = SimpleFrameAllocator::new(
    //     memory_map_tag.memory_areas(),
    //     kernel_start,
    //     kernel_end,
    //     multiboot_start,
    //     multiboot_end,
    // )
    // .expect("Could not create a simple frame allocator!");
    // for i in 0.. {
    //     if let None = simple_frame_allocator.allocate_frame() {
    //         println!("Allocated {} frames with simple frame allocator", i);
    //         break;
    //     }
    // }

    // // --------------- PAGING TESTS ---------------

    // let mut frame_allocator = SimpleFrameAllocator::new(
    //     memory_map_tag.memory_areas(),
    //     kernel_start,
    //     kernel_end,
    //     multiboot_start,
    //     multiboot_end,
    // )
    // .expect("Could not create a simple frame allocator!");
    // memory::test_paging(&mut frame_allocator);

    loop {}
}
