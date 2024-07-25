#![no_std]
#![no_main]
#![feature(ptr_metadata)]

extern crate multiboot2;
mod vga_buffer;

use core::panic::PanicInfo;
use multiboot2::BootInformationHeader;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[no_mangle]
pub extern "C" fn main(mb_boot_info_addr: usize) -> ! {
    let mb_info = unsafe {
        multiboot2::BootInformation::load(mb_boot_info_addr as *const BootInformationHeader)
    }
    .expect("Invalid multiboot2 boot information.");

    let mem_map_tag = mb_info
        .memory_map_tag()
        .expect("Memory map tag is not present.");

    for area in mem_map_tag.memory_areas() {
        println!(
            "    start: 0x{:x}, length: 0x{:x}",
            area.start_address(),
            area.size()
        );
    }

    let elf_sections_tag = mb_info.elf_sections().expect("Elf-sections tag required");

    println!("kernel sections:");
    for section in elf_sections_tag {
        println!(
            "    addr: 0x{:x}, size: 0x{:x}, flags: 0x{:x}",
            section.start_address(),
            section.size(),
            section.flags()
        );
    }

    loop {}
}
