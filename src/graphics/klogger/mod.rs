mod font_renderer;
mod painter;

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
        let mut buf = [0u8; 4]; // enough for any UTF-8 character

        for (i, c) in string.chars().enumerate() {
            let bytes = c.encode_utf8(&mut buf).as_bytes();

            // TODO: this should take in consideration the actual size of the font in pixels
            self.fr.draw_char(&self.fb, bytes, i as u32 * self.fr.pixel_width(), 0, color);
        }
    }
}
