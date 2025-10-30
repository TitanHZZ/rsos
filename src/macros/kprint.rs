
#[macro_export]
macro_rules! kprintln {
    ( $r:expr, $g:expr, $b:expr, $fmt:expr, $($arg:tt)* ) => {{ use $crate::kprint; kprint!($r, $g, $b, concat!($fmt, "\n"), $($arg)*) }};
    ( $r:expr, $g:expr, $b:expr, $fmt:expr ) => {{ use $crate::kprint; kprint!($r, $g, $b, concat!($fmt, "\n")) }};
    ( $fmt:expr, $($arg:tt)* ) => {{ use $crate::kprint; kprint!(concat!($fmt, "\n"), $($arg)*) }};
    ( $fmt:expr ) => {{ use $crate::kprint; kprint!(concat!($fmt, "\n")) }};
}

#[macro_export]
macro_rules! kprint {
    // colored print with args
    ( $r:expr, $g:expr, $b:expr, $fmt:expr, $($arg:tt)* ) => {{
        // TODO: i think it would make sense to check if the values are valid as u8s
        use $crate::graphics::KLOGGER;
        KLOGGER.write_fmt_colored($r as u8, $g as u8, $b as u8, format_args!($fmt, $($arg)*)).unwrap();
    }};

    // colored print without args
    ( $r:expr, $g:expr, $b:expr, $fmt:expr ) => {
        kprint!($r, $g, $b, concat!($fmt, "{}"), "");
    };

    // conventional print with args
    ( $fmt:expr, $($arg:tt)* ) => {{
        kprint!(255, 255, 255, $fmt, $($arg)*);
    }};

    // conventional print without args
    ( $fmt:expr ) => {{
        kprint!(255, 255, 255, concat!($fmt, "{}"), "");
    }};
}
