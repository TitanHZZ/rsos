// https://wiki.osdev.org/I8042_PS/2_Controller

use crate::{io_port::IoPort, log};
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

fn get_status_register_flags() -> StatusRegisterFlags {
    StatusRegisterFlags::from_bits_truncate(IoPort::read_u8(STATUS_COMMAND_REGISTER))
}

fn wait_input_buffer_clear() {
    while get_status_register_flags().contains(StatusRegisterFlags::INPUT_BUFFER_FULL) {}
}

fn send_controller_command(command: u8) {
    wait_input_buffer_clear();
    IoPort::write_u8(STATUS_COMMAND_REGISTER, command);
}

fn get_data() -> u8 {
    // wait for data in the ouput buffer
    while !get_status_register_flags().contains(StatusRegisterFlags::OUTPUT_BUFFER_FULL) {}
    IoPort::read_u8(DATA_PORT)
}

fn send_data(data: u8) {
    wait_input_buffer_clear();
    IoPort::write_u8(DATA_PORT, data);
}

// TODO: what about PS/2 device detection instead o just assuming that they exist??
// TODO: fix bit shifts
pub fn i8042_init() {
    log!(ok, "Initializing PS/2 driver.");
    let mut is_dual: bool = false;

    // disable devices
    send_controller_command(0xAD); // disable channel 1
    send_controller_command(0xA7); // disable channel 2

    // flush the output buffer
    while get_status_register_flags().contains(StatusRegisterFlags::OUTPUT_BUFFER_FULL) {
        IoPort::read_u8(DATA_PORT);
    }

    // set the controller configuration byte
    send_controller_command(0x20);
    let new_controller_configuration = get_data() & 0b01110101; // clear bits 0, 4 and 6
    send_controller_command(0x60);
    send_data(new_controller_configuration);

    // controller self test
    send_controller_command(0xAA);
    if get_data() != 0x55 {
        log!(failed, "PS/2 controller self test failed!");
        return;
    }

    log!(ok, "PS/2 controller self test passed!");
    // TODO: what about ps/2 controller resets?

    // check for dual channel controllers
    send_controller_command(0xA8);
    send_controller_command(0x20);
    if get_data() & 0b000001000 == 0 {
        is_dual = true;
        log!(ok, "PS/2 controller is dual channel");

        // disable second ps/2 port (channel 2)
        send_controller_command(0xA7);

        // disable IRQs and enable clock for port 2
        send_controller_command(0x20);
        let new_controller_configuration = get_data() & 0b10111011; // clear bits 1 and 5
        send_controller_command(0x60);
        send_data(new_controller_configuration);
    }

    // TODO: make sure that i keep track of working ports
    // interface tests
    send_controller_command(0xAB);
    if get_data() != 0x00 {
        log!(ok, "First PS/2 port is broken");
    }

    if is_dual {
        send_controller_command(0xA9);
        if get_data() != 0x00 {
            log!(ok, "Second PS/2 port is broken");
        }
    }

    // enable devices
    send_controller_command(0xAE); // enable first port
    if is_dual {
        send_controller_command(0xA8); // enable second port
    }

    send_controller_command(0x20);
    let new_controller_configuration = get_data() & 0b01111111; // clear bit 0
}
