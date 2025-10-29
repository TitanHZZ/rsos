use crate::{assert_called_once, graphics::{Framebuffer, framebuffer::FrameBufferError, klogger::KLogger}};
use spin::Mutex;

// TODO: this would allow me to have a video system as well
pub enum GraphicsRendererType {
    // Text(KLogger<'static>),
    Text,
}

pub struct GraphicsRenderer(Mutex<Option<GraphicsRendererInner>>);

struct GraphicsRendererInner {
    fb: Framebuffer,
    typ: GraphicsRendererType,
}

impl GraphicsRenderer {
    pub(in crate::graphics) const fn new() -> Self {
        GraphicsRenderer(Mutex::new(None))
    }

    pub unsafe fn init(&self, typ: GraphicsRendererType) -> Result<(), FrameBufferError> {
        assert_called_once!("Cannot call GraphicsRenderer::init() more than once");
        let gr = &mut *self.0.lock();
        assert!(gr.is_none());

        *gr = Some(GraphicsRendererInner {
            fb: Framebuffer::new()?,
            typ,
        });

        Ok(())
    }
}
