use crate::{kprint, kprintln};
use core::fmt;

pub struct LOGGER;

impl LOGGER {
    pub fn failed(fmt: fmt::Arguments) {
        // [FAILED]
        kprint!("[");
        kprint!(255, 0, 0, "FAILED");
        kprintln!("] {}", fmt);
    }

    pub fn warn(fmt: fmt::Arguments) {
        // [ WARN ]
        kprint!("[");
        kprint!(255, 255, 0, " WARN ");
        kprintln!("] {}", fmt);
    }

    pub fn ok(fmt: fmt::Arguments) {
        // [  OK  ]
        kprint!("[");
        kprint!(0, 255, 0, "  OK  ");
        kprintln!("] {}", fmt);
    }
}

#[macro_export]
macro_rules! log {
    ( $method:ident, $($arg:tt)* ) => {{
        use $crate::logger::LOGGER;
        LOGGER::$method(format_args!($($arg)*));
    }};
}
