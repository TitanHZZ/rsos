#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn main() -> ! {
    let vga_buff = 0xb8000 as *mut u8;
    let str = b"Hello, World!";

    for (i, &ch) in str.into_iter().enumerate() {
        unsafe {
            *vga_buff.offset(i as isize * 2) = ch;
            *vga_buff.offset(i as isize * 2 + 1) = 0xf;
        }
    }

    loop {}
}
