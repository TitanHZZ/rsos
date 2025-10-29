use crate::graphics::{framebuffer::{FrameBufferColor, Framebuffer}, klogger::{painter::KLoggerPainter, psf::{Psf, PsfError}}, FRAMEBUFFER};
use core::{fmt, ptr::copy};

const FONT: &[u8] = include_bytes!("fonts/spleen-8x16.psfu");
const TAB_SIZE: usize = 4;

pub(in crate::graphics::klogger) struct FontRenderer<'a> {
    font: Psf<'a>,
    color: FrameBufferColor,
    column: usize,
    row: usize,
}

#[derive(Debug)]
pub enum FontError {
    PsfErrs((PsfError, PsfError)),
}

impl<'a> FontRenderer<'a> {
    pub(in crate::graphics::klogger) fn new(color: FrameBufferColor) -> Result<Self, FontError> {
        Ok(Self {
            font: Psf::from_bytes(FONT).map_err(FontError::PsfErrs)?,
            color,
            column: 0,
            row: 0,
        })
    }

    fn draw_char(&mut self, fb: &mut Framebuffer, chr: char, x: u32, y: u32) {
        if let Some(glyph) = self.font.get_glyph(chr) {
            let bytes_per_row = self.font.pixel_width().div_ceil(8) as usize;
            let pixel_height  = self.font.pixel_height() as usize;
            let pixel_width   = self.font.pixel_width() as usize;

            for ypos in 0..pixel_height {
                for xpos in 0..pixel_width {
                    let byte = glyph[ypos * bytes_per_row + (xpos / 8)];
                    if (byte >> (7 - (xpos % 8))) & 1 != 0 {
                        KLoggerPainter::put_pixel(fb, x + xpos as u32, y + ypos as u32, self.color);
                    }
                }
            }
        }
    }

    // TODO: do these fns really need to exist??
    pub(in crate::graphics::klogger) fn set_color(&mut self, color: FrameBufferColor) {
        self.color = color;
    }

    pub(in crate::graphics::klogger) fn color(&self) -> FrameBufferColor {
        self.color
    }
}

impl<'a> fmt::Write for FontRenderer<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // TODO: is locking every time we write a string really the best approach?
        let framebuffer = &mut *FRAMEBUFFER.lock();

        for chr in s.chars() {
            if self.column >= (framebuffer.width / self.font.pixel_width()) as _ {
                // go to the next line
                self.column = 0;
                self.row += 1;
            }

            if self.row >= framebuffer.height as usize {
                // "scroll" down
                self.row = framebuffer.height as usize - 1;
                unsafe { copy(
                    framebuffer.as_ptr().offset(framebuffer.pitch as isize),
                    framebuffer.as_mut_ptr(),
                    framebuffer.pitch as usize * (framebuffer.height - 1) as usize
                )};
            }

            match chr {
                '\n' => {
                    self.column = 0;
                    self.row += 1;
                }
                '\t' => {
                    // calculate how many spaces it needs to print
                    let count = TAB_SIZE - (self.column % TAB_SIZE);

                    // recursively write the spaces
                    for _ in 0..count {
                        // self.write_chr(0x20);
                        self.draw_char(framebuffer, ' ', self.column as u32 * self.font.pixel_width(), self.row as u32);
                    }
                }
                '\r' => {
                    self.column = 0;
                }
                chr => {
                    self.draw_char(framebuffer, chr, self.column as u32 * self.font.pixel_width(), self.row as u32);
                    self.column += 1;
                }
            }
        }

        Ok(())
    }
}
