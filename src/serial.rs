#![allow(dead_code)]

// https://wiki.osdev.org/I/O_Ports
// https://wiki.osdev.org/Serial_Ports
use core::{cell::LazyCell, fmt};
use crate::io_port::IoPort;
use spin::Mutex;

pub struct SerialPort(u16);

// 0x3F8 is the default addr for COM1
pub static SERIAL_PORT: Mutex<LazyCell<SerialPort>> = Mutex::new(LazyCell::new(|| SerialPort::init(0x3F8)));

impl SerialPort {
    #[allow(clippy::identity_op)]
    /// This `needs` to be called at least once before any data being sent but should be fine if it is called mutiple times.
    fn init(port: u16) -> SerialPort {
        IoPort::write_u8(port + 1, 0x00); // disable all interrupts
        IoPort::write_u8(port + 3, 0x80); // enable DLAB (set baud rate divisor)
        IoPort::write_u8(port + 0, 0x03); // set divisor to 3 (lo byte) 38400 baud rate
        IoPort::write_u8(port + 1, 0x00); //                  (hi byte)
        IoPort::write_u8(port + 3, 0x03); // 8 bits, no parity, one stop bit
        IoPort::write_u8(port + 2, 0xC7); // enable FIFO, clear them, with 14-byte threshold
        IoPort::write_u8(port + 4, 0x0B); // IRQs enabled, RTS/DSR set

        // set the port to normal operation mode (not-loopback with IRQs enabled and OUT#1 and OUT#2 bits enabled)
        IoPort::write_u8(port + 4, 0x0F);

        Self(port)
    }

    fn send(&self, value: u8) {
        // wait for the serial port to be ready for the transmission
        while IoPort::read_u8(self.0 + 5) & 0x20 == 0 {}

        IoPort::write_u8(self.0, value);
    }

    fn receive(&self) -> u8 {
        // wait for the serial port to be ready to receive
        while IoPort::read_u8(self.0 + 5) & 1 == 0 {}

        IoPort::read_u8(self.0)
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // send all the bytes 'as is'
        for byte in s.bytes() {
            self.send(byte);
        }

        Ok(())
    }
}
