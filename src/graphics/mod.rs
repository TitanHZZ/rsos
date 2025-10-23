pub mod klogger;
mod framebuffer;

use crate::graphics::framebuffer::Framebuffer;
use core::cell::LazyCell;
use spin::Mutex;

// TODO: 'graphics' should probably be a subsystem (just like 'memory' is)
//       this would, probably, require a trait

// TODO: should this use a RwLock?
// TODO: should this actually use an explicit call to an init fn? instead of implicitly initializing?
static FRAMEBUFFER: Mutex<LazyCell<Framebuffer>> = Mutex::new(LazyCell::new(||
    Framebuffer::new().expect("Could not initialize the framebuffer")
));
