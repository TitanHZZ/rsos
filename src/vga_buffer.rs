use core::{cell::LazyCell, fmt, ptr::copy};
use spin::Mutex;

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;
const TAB_SIZE: usize = 4;

#[repr(u8)]
#[allow(dead_code)]
pub enum Color {
    Black        = 0x0,
    Blue         = 0x1,
    Green        = 0x2,
    Cyan         = 0x3,
    Red          = 0x4,
    Magenta      = 0x5,
    Brown        = 0x6,
    LightGray    = 0x7,
    DarkGray     = 0x8,
    LightBlue    = 0x9,
    LightGreen   = 0xa,
    LightCyan    = 0xb,
    LightRed     = 0xc,
    LightMagenta = 0xd,
    Yellow       = 0xe,
    White        = 0xf,
}

#[repr(transparent)]
#[derive(Clone, Copy)]
struct ColorCode(u8);

impl ColorCode {
    const fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 0x4 | (foreground as u8))
    }
}

#[repr(C)]
struct ScreenChar {
    ascii_char: u8,
    color_code: ColorCode,
}

#[repr(transparent)]
struct ScreenBuff {
    chars: [[ScreenChar; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    column: usize,
    row: usize,
    color_code: ColorCode,
    buffer: &'static mut ScreenBuff,
}

impl Writer {
    fn write_chr(&mut self, chr: u8) {
        if self.column >= BUFFER_WIDTH {
            // go to the next line
            self.column = 0;
            self.row += 1;
        }

        if self.row >= BUFFER_HEIGHT {
            // "scroll" down
            self.row = BUFFER_HEIGHT - 1;
            unsafe {
                // perhaps this could be optimized with a circular buffer??
                copy(self.buffer.chars.as_ptr().offset(1), self.buffer.chars.as_mut_ptr(), BUFFER_HEIGHT - 1);
            }
        }

        match chr {
            // match printable ascci characters
            0x20..=0x7e => {
                self.buffer.chars[self.row][self.column] = ScreenChar {
                    ascii_char: chr,
                    color_code: self.color_code,
                };

                self.column += 1;
            }
            b'\n' => {
                self.column = 0;
                self.row += 1;
            }
            b'\t' => {
                // calculate how many spaces it needs to print
                let count = TAB_SIZE - (self.column % TAB_SIZE);

                // recursively write the spaces
                for _ in 0..count {
                    self.write_chr(0x20);
                }
            }
            b'\r' => {
                self.column = 0;
            }
            _ => {}
        }
    }

    fn write_str(&mut self, str: &str) {
        for chr in str.bytes() {
            self.write_chr(chr);
        }
    }

    pub fn set_colors(&mut self, foreground: Color, background: Color) {
        self.color_code = ColorCode::new(foreground, background);
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str(s);
        Ok(())
    }
}

// spin locks are not the best but they work and we have no concept of blocking or even threads
// in this OS to use a better alternative (maybe use once_cell crate??)
pub static WRITER: Mutex<LazyCell<Writer>> = Mutex::new(LazyCell::new(|| Writer {
    column: 0,
    row: 0,
    color_code: ColorCode::new(Color::White, Color::Black),
    buffer: unsafe { &mut *(0xb8000 as *mut ScreenBuff) },
}));

#[macro_export]
macro_rules! println {
    ( $fg:path, $bg:path, $fmt:expr, $($arg:tt)* ) => (print!($fg, $bg, concat!($fmt, "\n"), $($arg)*));
    ( $fg:path, $bg:path, $fmt:expr ) => (print!($fg, $bg, concat!($fmt, "\n")));
    ( $fmt:expr, $($arg:tt)* ) => (print!(concat!($fmt, "\n"), $($arg)*));
    ( $fmt:expr ) => (print!(concat!($fmt, "\n")));
}

#[macro_export]
macro_rules! print {
    // colored print with args
    ( $fg:path, $bg:path, $fmt:expr, $($arg:tt)* ) => {{
        use crate::vga_buffer::{WRITER, Writer};
        use core::{cell::LazyCell, fmt::Write};

        // lock drops when the scope ends
        let lazy_cell: &mut LazyCell<Writer> = &mut *WRITER.lock();
        let writer: &mut Writer = LazyCell::force_mut(lazy_cell);

        writer.set_colors($fg, $bg);
        writer.write_fmt(format_args!($fmt, $($arg)*)).unwrap();

        // restore the default colors
        writer.set_colors(Color::White, Color::Black);
    }};

    // colored print without args
    ( $fg:path, $bg:path, $fmt:expr ) => {
        print!($fg, $bg, concat!($fmt, "{}"), "");
    };

    // conventional print with args
    ( $fmt:expr, $($arg:tt)* ) => {{
        use crate::Color;
        print!(Color::White, Color::Black, $fmt, $($arg)*);
    }};

    // conventional print without args
    ( $fmt:expr ) => {{
        use crate::Color;
        print!(Color::White, Color::Black, concat!($fmt, "{}"), "");
    }};
}
