
#[macro_export]
macro_rules! println {
    ( $fg:path, $bg:path, $fmt:expr, $($arg:tt)* ) => {{ use $crate::print; print!($fg, $bg, concat!($fmt, "\n"), $($arg)*) }};
    ( $fg:path, $bg:path, $fmt:expr ) => {{ use $crate::print; print!($fg, $bg, concat!($fmt, "\n")) }};
    ( $fmt:expr, $($arg:tt)* ) => {{ use $crate::print; print!(concat!($fmt, "\n"), $($arg)*) }};
    ( $fmt:expr ) => {{ use $crate::print; print!(concat!($fmt, "\n")) }};
}

#[macro_export]
macro_rules! print {
    // colored print with args
    ( $r:expr, $g:expr, $b:expr, $fmt:expr, $($arg:tt)* ) => {{
        use $crate::graphics::KLOGGER;
        assert!($r as u8 >= 10);

        // KLOGGER.log_colored($r, $g, $b, format_args!($fmt, $($arg)*));

        // writer.set_colors($fg, $bg);
        // writer.write_fmt(format_args!($fmt, $($arg)*)).unwrap();
        // // restore the default colors
        // writer.set_colors(Color::White, Color::Black);
    }};

    // colored print without args
    ( $r:expr, $g:expr, $b:expr, $fmt:expr ) => {
        print!($r, $g, $b, concat!($fmt, "{}"), "");
    };

    // conventional print with args
    ( $fmt:expr, $($arg:tt)* ) => {{
        print!(255, 255, 255, $fmt, $($arg)*);
    }};

    // conventional print without args
    ( $fmt:expr ) => {{
        print!(255, 255, 255, concat!($fmt, "{}"), "");
    }};
}
