mod font_renderer;
mod painter;
mod psf;

use crate::graphics::{framebuffer::{FrameBufferColor, FrameBufferError, Framebuffer}, klogger::font_renderer::{FontError, FontRenderer}};

pub struct KLogger<'a> {
    fb: Framebuffer,
    fr: FontRenderer<'a>,
}

#[derive(Debug)]
pub enum KLoggerError {
    FrameBufferErr(FrameBufferError),
    FontErr(FontError),
}

impl<'a> KLogger<'a> {
    pub fn new() -> Result<Self, KLoggerError> {
        Ok(Self {
            fb: Framebuffer::new().map_err(KLoggerError::FrameBufferErr)?,
            fr: FontRenderer::new().map_err(KLoggerError::FontErr)?,
        })
    }

    pub fn log(&self, string: &str) {
        let color = FrameBufferColor::new(255, 255, 255);
        for (i, chr) in string.chars().enumerate() {
            self.fr.draw_char(&self.fb, chr, i as u32 * self.fr.pixel_width(), 0, color);
        }
    }
}
