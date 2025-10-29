
#[macro_export]
macro_rules! serial_println {
    ( $fmt:expr, $($arg:tt)* ) => {{
        use $crate::serial_print;
        serial_print!(concat!($fmt, "\n"), $($arg)*);
    }};

    ( $fmt:expr ) => {{
        use $crate::serial_print;
        serial_print!(concat!($fmt, "\n"));
    }};
}

// TODO: if we print the result of a function that also calls print, we have a dead lock
#[macro_export]
macro_rules! serial_print {
    ( $fmt:expr, $($arg:tt)* ) => {{
        use core::{cell::LazyCell, fmt::Write};
        use $crate::serial::SERIAL_PORT;

        LazyCell::force_mut(&mut SERIAL_PORT.lock()).write_fmt(format_args!($fmt, $($arg)*)).unwrap();
    }};

    ( $fmt:expr ) => {{
        use core::{cell::LazyCell, fmt::Write};
        use $crate::serial::SERIAL_PORT;

        LazyCell::force_mut(&mut SERIAL_PORT.lock()).write_fmt(format_args!($fmt)).unwrap();
    }};
}
