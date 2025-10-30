mod graphics_renderer;
mod framebuffer;
pub mod klogger;

use crate::graphics::{framebuffer::Framebuffer, klogger::KLogger};
use core::cell::LazyCell;
use spin::Mutex;

// TODO: 'graphics' should probably be a subsystem (just like 'memory' is)
//       this would, probably, require a trait

// TODO: `GraphicsRenderer` was a test for what a more modular system would, maybe, look like for when this has a
//       more advanced rendering system then "just" writing text

// TODO: with a more advanced initialization system, it should be possible to render text to the screen before the higher half remap
// TODO: this should definitely be "owned" by something (maybe by the `GraphicsRenderer`?)
// TODO: should this use a RwLock?
// TODO: should this actually use an explicit call to an init fn? instead of implicitly initializing?
static FRAMEBUFFER: Mutex<LazyCell<Framebuffer>> = Mutex::new(LazyCell::new(||
    Framebuffer::new().expect("Could not initialize the framebuffer")
));

pub static KLOGGER: KLogger = KLogger::new();

// pub static GRAPHICS_RENDERER: GraphicsRenderer = GraphicsRenderer::new();
