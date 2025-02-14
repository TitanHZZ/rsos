use crate::{print, println, vga_buffer::Color};
use core::fmt;

pub struct LOGGER;
impl LOGGER {
    pub fn failed(fmt: fmt::Arguments) {
        // [FAILED]
        print!(Color::White,    Color::Black, "[");
        print!(Color::LightRed, Color::Black, "FAILED");
        println!(Color::White,  Color::Black, "] {}", fmt);
    }

    pub fn warn(fmt: fmt::Arguments) {
        // [ WARN ]
        print!(Color::White,   Color::Black, "[");
        print!(Color::Yellow,  Color::Black, " WARN ");
        println!(Color::White, Color::Black, "] {}", fmt);
    }

    pub fn ok(fmt: fmt::Arguments) {
        // [  OK  ]
        print!(Color::White,      Color::Black, "[");
        print!(Color::LightGreen, Color::Black, "  OK  ");
        println!(Color::White,    Color::Black, "] {}", fmt);
    }
}

#[macro_export]
macro_rules! log {
    ( $method:ident, $($arg:tt)* ) => {{
        use crate::logger::LOGGER;
        LOGGER::$method(format_args!($($arg)*));
    }};
}
