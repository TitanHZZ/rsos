mod font_renderer;
mod painter;
mod psf;

use crate::{assert_called_once, graphics::{framebuffer::{FrameBufferColor, FrameBufferError}, klogger::font_renderer::{FontError, FontRenderer}}};
use core::fmt::{self, Write};
use spin::Mutex;

pub struct KLogger<'a>(Mutex<Option<FontRenderer<'a>>>);

#[derive(Debug)]
pub enum KLoggerError {
    FrameBufferErr(FrameBufferError),
    FontErr(FontError),
}

impl<'a> KLogger<'a> {
    /// Creates a new **KLogger** that needs to be [initialized](KLogger::init()).
    pub(in crate::graphics) const fn new() -> Self {
        KLogger(Mutex::new(None))
    }

    /// Initializes this simple Kernel logger.
    /// 
    /// Default color is pure white.
    /// 
    /// # Safety
    /// 
    /// - **Must** be called *after* the higher half remapping is completed and *after* the [HEAP_ALLOCATOR](crate::memory::simple_heap_allocator::HEAP_ALLOCATOR) is initialized.
    /// 
    /// Failure to follow the rules may result in data corruption.
    /// 
    /// # Panics
    /// 
    /// If called more than once.
    pub unsafe fn init(&self) -> Result<(), KLoggerError> {
        assert_called_once!("Cannot call KLogger::init() more than once");
        let klogger = &mut *self.0.lock();
        assert!(klogger.is_none());

        *klogger = Some(FontRenderer::new(FrameBufferColor::new(255, 255, 255)).map_err(KLoggerError::FontErr)?);
        Ok(())
    }

    /// Print `str` to the screen.
    /// 
    /// # Panics
    /// 
    /// If called before [initialization](KLogger::init()).
    pub fn log(&self, s: &str) -> fmt::Result {
        self.0.lock().as_mut().unwrap().write_str(s)
    }

    pub fn log_colored(&self, r: u8, g: u8, b: u8, s: &str) -> fmt::Result {
        let fr = &mut *self.0.lock();
        let fr = fr.as_mut().unwrap();

        let original_color = fr.color();
        fr.set_color(FrameBufferColor::new(r, g, b));
        fr.write_str(s)?;
        fr.set_color(original_color);

        Ok(())
    }

    /// Set the text color.
    /// 
    /// # Panics
    /// 
    /// If called before [initialization](KLogger::init()).
    pub fn set_color(&self, r: u8, g: u8, b: u8) {
        self.0.lock().as_mut().unwrap().set_color(FrameBufferColor::new(r, g, b));
    }
}
