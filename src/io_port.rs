#![cfg(test)]

use core::arch::asm;

pub struct IO_PORT;

impl IO_PORT {
    pub fn read_u32(port: u16) -> u32 {
        let value: u32;
        unsafe {
            asm!("in eax, dx", in("dx") port, out("eax") value, options(nostack, nomem, preserves_flags));
        }
        value
    }

    pub fn read_u8(port: u16) -> u8 {
        let value: u8;
        unsafe {
            asm!("in al, dx", in("dx") port, out("al") value, options(nostack, nomem, preserves_flags));
        }
        value
    }

    pub fn write_u32(port: u16, value: u32) {
        unsafe {
            asm!("out dx, eax", in("dx") port, in("eax") value, options(nomem, nostack, preserves_flags));
        }
    }

    pub fn write_u8(port: u16, value: u8) {
        unsafe {
            asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
        }
    }
}
