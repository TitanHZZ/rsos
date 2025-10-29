mod serial;
mod print;

/// Asserts that a function has only been called once at runtime.
/// 
/// This will invoke the [`panic!`] macro if the function where
/// this is placed gets called more than once.
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
