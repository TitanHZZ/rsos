mod font_renderer;
mod painter;

use crate::graphics::{framebuffer::{FrameBufferError, Framebuffer}, klogger::font_renderer::FontRenderer};

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
        self.fr.draw_char(&self.fb, string.as_bytes(), 0, 0);

        // // let scale = 1;
        // for (idx, chr) in string.bytes().enumerate() {
        //     self.fr.draw_char(&self.fb, chr, idx as u32 * 8, 0);
        // }
    }
}
