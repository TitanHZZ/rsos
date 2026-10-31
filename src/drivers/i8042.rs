// https://wiki.osdev.org/I8042_PS/2_Controller

use crate::io_port::IoPort;
use bitflags::bitflags;

const DATA_PORT: u16 = 0x60;
const STATUS_COMMAND_REGISTER: u16 = 0x64;

bitflags! {
    struct StatusRegisterFlags: u8 {
        const OUTPUT_BUFFER_FULL = 1 << 0;
        const INPUT_BUFFER_FULL  = 1 << 1;
        const SYSTEM_FLAG        = 1 << 2;
        const COMMAND_DATA       = 1 << 3;
        // const UNKNOWN         = 1 << 4;
        // const UNKNOWN         = 1 << 5;
        const TIME_OUT_ERROR     = 1 << 6;
        const PARITY_ERROR       = 1 << 7;
    }
}

fn wait_input_buffer_clear() {
    while StatusRegisterFlags::from_bits_truncate(IoPort::read_u8(STATUS_COMMAND_REGISTER))
        .contains(StatusRegisterFlags::INPUT_BUFFER_FULL) {}
}

fn init() {
    // TODO: what about PS/2 device detection instead o just assuming that they exist??

    // disable devices
    IoPort::write_u8(STATUS_COMMAND_REGISTER, 0xAD);
    wait_input_buffer_clear();
    IoPort::write_u8(STATUS_COMMAND_REGISTER, 0xA7);
}
