mod font_renderer;
mod painter;
mod psf;

use crate::graphics::{framebuffer::{FrameBufferColor, FrameBufferError, Framebuffer}, klogger::font_renderer::FontRenderer};

pub struct KLogger<'a> {
    fb: Framebuffer,
    fr: FontRenderer<'a>,
}

impl<'a> KLogger<'a> {
    pub fn new() -> Result<Self, FrameBufferError> {
        Ok(Self {
            fb: Framebuffer::new()?,
            fr: FontRenderer::new(),
        })
    }

    pub fn log(&self, string: &str) {
        let color = FrameBufferColor::new(255, 255, 255);
        for (i, chr) in string.chars().enumerate() {
            self.fr.draw_char(&self.fb, chr, i as u32 * self.fr.pixel_width(), 0, color);
        }
    }
}
