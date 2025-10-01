use crate::graphics::{framebuffer::{FrameBufferColor, Framebuffer}, klogger::{painter::KLoggerPainter, psf::Psf}};

const FONT: &[u8] = include_bytes!("fonts/spleen-12x24.psfu");

pub(in crate::graphics::klogger) struct FontRenderer<'a> {
    font: Psf<'a>,
}

impl<'a> FontRenderer<'a> {
    // TODO: this should not "crash" with an invalid font
    pub(in crate::graphics::klogger) fn new() -> Self {
        Self { font: Psf::from_bytes(FONT).expect("Could not parse font") }
    }

    pub(in crate::graphics::klogger) fn pixel_width(&self) -> u32 {
        self.font.pixel_width()
    }

    pub(in crate::graphics::klogger) fn draw_char(&self, fb: &Framebuffer, chr: &[u8], xpos: u32, ypos: u32, color: FrameBufferColor) {
        if let Some(glyph) = self.font.get_glyph(chr) {
            let bytes_per_row = self.font.pixel_width().div_ceil(8) as usize;
            for y in 0..self.font.pixel_height() as usize {
                for x in 0..self.font.pixel_width() as usize {
                    let byte = glyph[y * bytes_per_row + (x / 8)];
                    if (byte >> (7 - (x % 8))) & 1 != 0 {
                        KLoggerPainter::put_pixel(fb, xpos + x as u32, ypos + y as u32, color);
                    }
                }
            }
        }
    }
}
