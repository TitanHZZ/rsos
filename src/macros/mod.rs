
// TODO: description

#[macro_export]
macro_rules! assert_called_once {
    // with args
    ( $fmt:expr, $($arg:tt)* ) => {{
        use core::sync::atomic::{AtomicBool, Ordering};
        static CALLED: AtomicBool = AtomicBool::new(false);

        if CALLED.swap(true, Ordering::SeqCst) {
            panic!($fmt, $($arg)*);
        }
    }};

    // without args
    ( $fmt:expr ) => {{
        assert_called_once!(concat!($fmt, "{}"), "");
    }};
}
