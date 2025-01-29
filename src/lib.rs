#![no_std]
#![no_main]

// mod memory;
mod multiboot2;
mod vga_buffer;

use core::panic::PanicInfo;
use multiboot2::MbBootInfo;
// use memory::{FrameAllocator, SimpleFrameAllocator};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

// fn print_mem_status(mb_info: &BootInformation) {
//     let memory_map_tag = mb_info.memory_map_tag().expect("Memory map tag required");
//     println!("All memory areas:");
//     for area in memory_map_tag.memory_areas() {
//         println!(
//             "    start: 0x{:x}, length: {:.2} MB, type: {:?}",
//             area.start_address(),
//             area.size() as f64 / 1024.0 / 1024.0,
//             area.typ()
//         );
//     }
//     let total_memory: u64 = memory_map_tag
//         .memory_areas()
//         .into_iter()
//         .filter(|area| area.typ() == MemoryAreaType::Available)
//         .map(|area| area.size())
//         .sum();
//     println!(
//         "Total (available) memory: {} bytes ({:.2} GB)",
//         total_memory,
//         total_memory as f64 / 1024.0 / 1024.0 / 1024.0
//     );
// }

#[no_mangle]
pub extern "C" fn main(mb_boot_info_addr: *const u8) -> ! {
    let mb_info = unsafe { MbBootInfo::new(mb_boot_info_addr) }.unwrap();

    // cmd line
    let cmd_line = mb_info.cmd_line().unwrap();
    println!("cmd line: '{}'", cmd_line.string().unwrap());

    // boot loader name
    let bln = mb_info.boot_loader_name().unwrap();
    println!("boot loader name: '{}'", bln.string().unwrap());

    // modules
    let modules = mb_info.modules();
    // println!("modules: {}", modules.unwrap().string().unwrap());

    // basic mem info
    let bmi = mb_info.basic_memory_info().unwrap();
    println!("basic mem info: {} {}", bmi.mem_lower, bmi.mem_upper);

    // bios boot device
    let bbd = mb_info.bios_boot_device().unwrap();
    println!("bios boot device: {}", bbd.partition);

    // memory map
    let mm = mb_info.memory_map().unwrap();
    let mm_entries = mm.entries().unwrap();
    println!("memory map: entry_size: {}, entry_version: {}", mm.entry_size, mm.entry_version);
    for e in mm_entries {
        println!("    base_addr: {}, len: {}, type: {:?}, reserved: {}", e.base_addr, e.length, e.entry_type(), e.reserved);
    }

    // let mb_info = unsafe {
    //     multiboot2::BootInformation::load(mb_boot_info_addr as *const BootInformationHeader)
    // }
    // .expect("Invalid multiboot2 boot information.");
    // let elf_sections_tag = mb_info.elf_sections().expect("Elf-sections tag required");
    // for a in elf_sections_tag {
    //     println!("is unused: {}", a.section_type() == ElfSectionType::Unused);
    // }
    // let cmd_line = mb_info.command_line_tag().expect("cmd line tag is required!");
    // println!("cmdline: {}", cmd_line.cmdline().unwrap());
    // print_mem_status(&mb_info);
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
