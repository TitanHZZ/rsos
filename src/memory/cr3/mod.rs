use super::{PhysicalAddress, VirtualAddress};
use core::arch::asm;

pub struct CR3;

/*
 * Safety: This unsafe blocks are safe as we know that the asm is valid
 * and the code will always run in kernel mode so it will always have access to the cr3 register.
 */
impl CR3 {
    pub fn invalidate_entry(virt_addr: VirtualAddress) {
        // invalidate the TLB entry
        unsafe {
            asm!("invlpg [{}]", in(reg) virt_addr as u64, options(nostack, preserves_flags));
        }
    }

    pub fn invalidate_all() {
        let cr3: u64;
        unsafe {
            asm!("mov {}, cr3", out(reg) cr3, options(nostack, preserves_flags));
            asm!("mov cr3, {}", in(reg) cr3, options(nostack, preserves_flags));
        }
    }

    pub fn get() -> PhysicalAddress {
        let cr3: u64;
        unsafe {
            asm!("mov {}, cr3", out(reg) cr3, options(nostack, preserves_flags));
        }

        cr3 as PhysicalAddress
    }

    pub fn set(addr: PhysicalAddress) {
        unsafe {
            asm!("mov cr3, {}", in(reg) addr as u64, options(nostack, preserves_flags));
        }
    }
}
