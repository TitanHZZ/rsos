#![no_std]
#![no_main]
#![feature(ptr_metadata)]

mod multiboot2;
mod vga_buffer;

use core::panic::PanicInfo;
use multiboot2::MbInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[no_mangle]
pub extern "C" fn main(mb_boot_info_addr: usize) -> ! {
    let mb_info = MbInfo::new(mb_boot_info_addr).expect("Invalid mb boot info ptr.");

    println!("total size: {}", mb_info.header.total_size);
    println!("reserved:   {}", mb_info.header.reserved);

    loop {}
}
