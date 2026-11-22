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

// TODO: non exhaustive list
enum I8042ControllerCommand {
    DisableFirstPort,
    DisableSecondPort,
    ReadConfigByte,
    WriteConfigByte(u8),
    TestController,
    EnableSecondPort,
    TestFirstPort,
    TestSecondPort,
    EnableFirstPort,
    WriteNextByteToSecondPort,
}

impl I8042ControllerCommand {
    const fn cmd_byte(&self) -> u8 {
        match self {
            Self::DisableFirstPort          => 0xAD,
            Self::DisableSecondPort         => 0xA7,
            Self::ReadConfigByte            => 0x20,
            Self::WriteConfigByte(_)        => 0x60,
            Self::TestController            => 0xAA,
            Self::EnableSecondPort          => 0xA8,
            Self::TestFirstPort             => 0xAB,
            Self::TestSecondPort            => 0xA9,
            Self::EnableFirstPort           => 0xAE,
            Self::WriteNextByteToSecondPort => 0xD4,
        }
    }
}

#[derive(PartialEq, Eq)]
enum I8042Port {
    Port1,
    Port2,
}

// TODO: this should not be here
// TODO: how big is the delay really?
fn io_wait() {
    IoPort::write_u8(0x80, 0);
}

fn get_status_register_flags() -> StatusRegisterFlags {
    StatusRegisterFlags::from_bits_truncate(IoPort::read_u8(STATUS_COMMAND_REGISTER))
}

fn wait_input_buffer_clear() {
    while get_status_register_flags().contains(StatusRegisterFlags::INPUT_BUFFER_FULL) {}
}

fn get_data() -> Option<u8> {
    // TODO: how big should the loop be??
    for _ in 0..500 {
        // wait for data in the ouput buffer
        if get_status_register_flags().contains(StatusRegisterFlags::OUTPUT_BUFFER_FULL) {
            return Some(IoPort::read_u8(DATA_PORT));
        }

        // add some delay
        io_wait();
    }

    None
}

fn send_data(data: u8) {
    wait_input_buffer_clear();
    IoPort::write_u8(DATA_PORT, data);
}

fn send_controller_command(command: I8042ControllerCommand) -> Option<u8> {
    wait_input_buffer_clear();
    IoPort::write_u8(STATUS_COMMAND_REGISTER, command.cmd_byte());
    match command {
        I8042ControllerCommand::WriteConfigByte(data) => send_data(data),
        _ => ()
    }

    // TODO: better solution to not create a delay on every call??
    get_data()
}

// TODO: what about checking if a second port really is available??
fn send_device_command(port: I8042Port,command: u8) -> Option<u8> {
    if port == I8042Port::Port2 {
        // tell the ps/2 controller that we want to talk to the second port
        send_controller_command(I8042ControllerCommand::WriteNextByteToSecondPort);
    }

    send_data(command);

    // TODO: better solution to not create a delay on every call??
    get_data()
}

// TODO: what about PS/2 device detection instead of just assuming that they exist??
pub fn i8042_init() -> Result<(), I8042Error> {
    log!(ok, "Initializing PS/2 driver.");
    let (has_first_port, has_second_port) = {
        let mut is_dual: bool = false;

        // --- disable devices --- //
        send_controller_command(I8042ControllerCommand::DisableFirstPort);
        send_controller_command(I8042ControllerCommand::DisableSecondPort);

        // --- flush the output buffer --- //
        while get_status_register_flags().contains(StatusRegisterFlags::OUTPUT_BUFFER_FULL) {
            IoPort::read_u8(DATA_PORT);
        }

        // --- set the controller configuration byte --- // clear bits 0, 4 and 6
        let config_byte = send_controller_command(I8042ControllerCommand::ReadConfigByte).ok_or(I8042Error::NoData)?;
        send_controller_command(I8042ControllerCommand::WriteConfigByte(config_byte & 0b10101110));

        // --- controller self test --- //
        let config_byte = send_controller_command(I8042ControllerCommand::ReadConfigByte).ok_or(I8042Error::NoData)?;
        if send_controller_command(I8042ControllerCommand::TestController).ok_or(I8042Error::NoData)? != 0x55 {
            return Err(I8042Error::ControllerSelfTestFailed);
        }

        // restore config byte
        send_controller_command(I8042ControllerCommand::WriteConfigByte(config_byte));

        log!(ok, "PS/2 controller self test passed!");

        // --- check for dual channel controllers --- //
        send_controller_command(I8042ControllerCommand::EnableSecondPort);
        if send_controller_command(I8042ControllerCommand::ReadConfigByte).ok_or(I8042Error::NoData)? & 0b00100000 == 0 {
            is_dual = true;
            log!(ok, "PS/2 controller is dual channel");

            // disable second ps/2 port (channel 2)
            send_controller_command(I8042ControllerCommand::DisableSecondPort);

            // disable IRQs and enable clock for port 2 (clear bits 1 and 5)
            let config_byte = send_controller_command(I8042ControllerCommand::ReadConfigByte).ok_or(I8042Error::NoData)?;
            send_controller_command(I8042ControllerCommand::WriteConfigByte(config_byte & 0b11011101));
        }

        // interface tests
        let first_port_broken = send_controller_command(I8042ControllerCommand::TestFirstPort).ok_or(I8042Error::NoData)? != 0x00;
        let second_port_broken = match is_dual {
            true => send_controller_command(I8042ControllerCommand::TestSecondPort).ok_or(I8042Error::NoData)? != 0x00,
            false => true
        };

        (first_port_broken, is_dual && !second_port_broken)
    };

    // --- enable devices --- //
    let mut new_controller_config_options: u8 = 0b00000000;

    // enable first port
    if has_first_port {
        send_controller_command(I8042ControllerCommand::EnableFirstPort);
        new_controller_config_options |= 0b00000001; //  set bit 0 (for the first port)
    }

    // enable second port
    if has_second_port {
        send_controller_command(I8042ControllerCommand::EnableSecondPort);
        new_controller_config_options |= 0b00000010; //  set bit 1 (for the second port)
    }

    // enable interrupts for the available ports
    let config_byte = send_controller_command(I8042ControllerCommand::ReadConfigByte).ok_or(I8042Error::NoData)?;
    send_controller_command(I8042ControllerCommand::WriteConfigByte(config_byte | new_controller_config_options));

    // --- reset devices --- //
    // TODO: i need to keep track of the available devices
    if has_first_port {
        let data = send_device_command(I8042Port::Port1, 0xFF).ok_or(I8042Error::NoData)?;
        if data == 0xFA {
            if get_data().ok_or(I8042Error::NoData)? != 0xAA {
                log!(warn, "Cannot use device on first PS/2 port")
            }
        } else if data == 0xAA {
            if get_data().ok_or(I8042Error::NoData)? != 0xFA {
                log!(warn, "Cannot use device on first PS/2 port")
            }
        } else {
            log!(warn, "Cannot use device on first PS/2 port")
        }
    }

    log!(ok, "{:?}", get_data());

    if has_second_port {
        let data = send_device_command(I8042Port::Port2, 0xFF).ok_or(I8042Error::NoData)?;
        if data == 0xFA {
            if get_data().ok_or(I8042Error::NoData)? != 0xAA {
                log!(warn, "Cannot use device on second PS/2 port")
            }
        } else if data == 0xAA {
            if get_data().ok_or(I8042Error::NoData)? != 0xFA {
                log!(warn, "Cannot use device on second PS/2 port")
            }
        } else {
            log!(warn, "Cannot use device on second PS/2 port")
        }
    }

    log!(ok, "{:?}", get_data());

    Ok(())
}
