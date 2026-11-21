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

#[derive(Debug)]
pub enum I8042Error {
    NoData,
    ControllerSelfTestFailed,
}

// TODO: what about a controller command enum??

// TODO: this should not be here
/// 1 - 4 microseconds of delay
fn io_wait() {
    IoPort::write_u8(0x80, 0);
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

fn get_data() -> Result<u8, I8042Error> {
    for _ in 0..1_000_000 {
        // wait for data in the ouput buffer
        if get_status_register_flags().contains(StatusRegisterFlags::OUTPUT_BUFFER_FULL) {
            return Ok(IoPort::read_u8(DATA_PORT));
        }

        // add some delay
        io_wait();
    }

    Err(I8042Error::NoData)
}

fn send_controller_command_with_ret(command: u8) -> Result<u8, I8042Error> {
    send_controller_command(command);
    get_data()
}

fn send_data(data: u8) {
    wait_input_buffer_clear();
    IoPort::write_u8(DATA_PORT, data);
}

fn send_controller_command_with_data(command: u8, data: u8) {
    send_controller_command(command);
    send_data(data);
}

// TODO: what about PS/2 device detection instead o just assuming that they exist??
pub fn i8042_init() -> Result<(), I8042Error> {
    log!(ok, "Initializing PS/2 driver.");
    let (has_first_port, has_second_port) = {
        let mut is_dual: bool = false;

        // --- disable devices --- //
        send_controller_command(0xAD); // disable channel 1
        send_controller_command(0xA7); // disable channel 2

        // --- flush the output buffer --- //
        while get_status_register_flags().contains(StatusRegisterFlags::OUTPUT_BUFFER_FULL) {
            IoPort::read_u8(DATA_PORT);
        }

        // --- set the controller configuration byte --- //
        // clear bits 0, 4 and 6
        let new_controller_config = send_controller_command_with_ret(0x20)? & 0b10101110;
        send_controller_command_with_data(0x60, new_controller_config);

        // --- controller self test --- //
        if send_controller_command_with_ret(0xAA)? != 0x55 {
            return Err(I8042Error::ControllerSelfTestFailed);
        }

        log!(ok, "PS/2 controller self test passed!");
        // TODO: what about ps/2 controller resets?

        // --- check for dual channel controllers --- //
        send_controller_command(0xA8);
        if send_controller_command_with_ret(0x20)? & 0b00100000 == 0 {
            is_dual = true;
            log!(ok, "PS/2 controller is dual channel");

            // disable second ps/2 port (channel 2)
            send_controller_command(0xA7);

            // disable IRQs and enable clock for port 2 (clear bits 1 and 5)
            let new_controller_config = send_controller_command_with_ret(0x20)? & 0b11011101;
            send_controller_command_with_data(0x60, new_controller_config);
        }

        // interface tests
        let first_port_broken = send_controller_command_with_ret(0xAB)? != 0x00;
        let second_port_broken = match is_dual {
            true => send_controller_command_with_ret(0xA9)? != 0x00,
            false => true
        };

        (first_port_broken, is_dual && !second_port_broken)
    };

    // --- enable devices --- //
    let mut new_controller_config_options: u8 = 0b00000000;

    // enable first port
    if has_first_port {
        send_controller_command(0xAE);
        new_controller_config_options |= 0b00000001; //  set bit 0 (for the first port)
    }

    // enable second port
    if has_second_port {
        send_controller_command(0xA8);
        new_controller_config_options |= 0b00000010; //  set bit 1 (for the second port)
    }

    // enable interrupts for the available ports
    let new_controller_config = send_controller_command_with_ret(0x20)? | new_controller_config_options;
    send_controller_command_with_data(0x60, new_controller_config);

    // --- reset devices --- //
    if has_first_port {
        send_data(0xFF);
        let data = get_data()?;
        if data == 0xFA {
            if get_data()? != 0xAA {
                log!(warn, "Cannot use device on first PS/2 port")
            }
        } else if data == 0xAA {
            if get_data()? != 0xFA {
                log!(warn, "Cannot use device on first PS/2 port")
            }
        } else {
            log!(warn, "Cannot use device on first PS/2 port")
        }
    }

    // TODO: same for port 2 (sequence of commands is a bit more complicated)

    // get device type
    send_data(0xF5);
    if get_data()? != 0xFA {
        log!(warn, "Could not get device type")
    }

    send_data(0xF2);
    if get_data()? != 0xFA {
        log!(warn, "Could not get device type")
    }

    log!(ok, "{:#04X}", get_data()?);

    // send_data(0xF4);
    // if get_data() != 0xFA {
    //     log!(warn, "Could not get device type")
    // }

    Ok(())
}
