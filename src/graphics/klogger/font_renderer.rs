use crate::graphics::{framebuffer::{FrameBufferColor, Framebuffer}, klogger::{painter::KLoggerPainter, psf::{Psf, PsfError}}};

const FONT: &[u8] = include_bytes!("fonts/spleen-8x16.psfu");

pub(in crate::graphics::klogger) struct FontRenderer<'a> {
    font: Psf<'a>,
}

#[derive(Debug)]
pub enum FontError {
    PsfErrs((PsfError, PsfError)),
}

impl<'a> FontRenderer<'a> {
    pub(in crate::graphics::klogger) fn new() -> Result<Self, FontError> {
        Ok(Self { font: Psf::from_bytes(FONT).map_err(FontError::PsfErrs)? })
    }

    pub(in crate::graphics::klogger) fn pixel_width(&self) -> u32 {
        self.font.pixel_width()
    }

    pub(in crate::graphics::klogger) fn draw_char(&self, fb: &Framebuffer, chr: char, x: u32, y: u32, color: FrameBufferColor) {
        if let Some(glyph) = self.font.get_glyph(chr) {
            let bytes_per_row = self.font.pixel_width().div_ceil(8) as usize;
            let pixel_height  = self.font.pixel_height() as usize;
            let pixel_width   = self.font.pixel_width() as usize;

            for ypos in 0..pixel_height {
                for xpos in 0..pixel_width {
                    let byte = glyph[ypos * bytes_per_row + (xpos / 8)];
                    if (byte >> (7 - (xpos % 8))) & 1 != 0 {
                        KLoggerPainter::put_pixel(fb, x + xpos as u32, y + ypos as u32, color);
                    }
                }
            }
        }
    }
}
