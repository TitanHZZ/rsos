#![no_std]
#![no_main]
#![feature(lazy_get)]
#![feature(custom_test_frameworks)]
#![test_runner(rsos::test_runner)]
#![reexport_test_harness_main = "test_main"]

#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(mb_boot_info_addr: *const u8) -> ! {
    test_main();
    loop {}
}

#[test_case]
fn basic_assert() {
    assert_eq!(1, 1);
}
