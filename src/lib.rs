#![no_std]
#![no_main]

extern crate multiboot2;
mod memory;
mod vga_buffer;

use core::panic::PanicInfo;
use memory::SimpleFrameAllocator;
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

    let memory_map_tag = mb_info.memory_map_tag().expect("Memory map tag required");
    let elf_sections_tag = mb_info.elf_sections().expect("Elf-sections tag required");

    let kernel_start = elf_sections_tag
        .clone()
        .map(|s| s.start_address())
        .min()
        .unwrap() as usize;
    let kernel_end = elf_sections_tag
        .clone()
        .map(|s| s.start_address() + s.size())
        .max()
        .unwrap() as usize;

    let multiboot_start = mb_boot_info_addr;
    let multiboot_end = multiboot_start + (mb_info.total_size() as usize);

    let simple_frame_allocator = SimpleFrameAllocator::new(
        memory_map_tag.memory_areas(),
        kernel_start,
        kernel_end,
        multiboot_start,
        multiboot_end,
    );

    println!("Didnt crash!");

    loop {}
}
