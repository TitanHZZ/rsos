// use crate::{print, println, vga_buffer::Color};
use core::fmt;

// TODO: fix this (use the framebuffer)

pub struct LOGGER;

impl LOGGER {
    pub fn failed(_fmt: fmt::Arguments) {
        // // [FAILED]
        // print!(Color::White,    Color::Black, "[");
        // print!(Color::LightRed, Color::Black, "FAILED");
        // println!(Color::White,  Color::Black, "] {}", fmt);
    }

    pub fn warn(_fmt: fmt::Arguments) {
        // // [ WARN ]
        // print!(Color::White,   Color::Black, "[");
        // print!(Color::Yellow,  Color::Black, " WARN ");
        // println!(Color::White, Color::Black, "] {}", fmt);
    }

    pub fn ok(_fmt: fmt::Arguments) {
        // // [  OK  ]
        // print!(Color::White,      Color::Black, "[");
        // print!(Color::LightGreen, Color::Black, "  OK  ");
        // println!(Color::White,    Color::Black, "] {}", fmt);
    }
}

#[macro_export]
macro_rules! log {
    ( $method:ident, $($arg:tt)* ) => {{
        use $crate::logger::LOGGER;
        LOGGER::$method(format_args!($($arg)*));
    }};
}
