mod font_renderer;
mod painter;
mod psf;

use crate::graphics::{framebuffer::{FrameBufferColor, FrameBufferError}, klogger::font_renderer::{FontError, FontRenderer}};
use core::fmt::{self, Write};

pub struct KLogger<'a> {
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
            fr: FontRenderer::new(
                FrameBufferColor::new(255, 255, 255),
            ).map_err(KLoggerError::FontErr)?,
        })
    }

    pub fn log(&mut self, s: &str) -> fmt::Result {
        self.fr.write_str(s)
    }
}
