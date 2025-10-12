use crate::graphics::framebuffer::{FrameBufferColor, Framebuffer};

pub struct KLoggerPainter;

impl KLoggerPainter {
    pub fn put_pixel(fb: &mut Framebuffer, x: u32, y: u32, color: FrameBufferColor) {
        let pixel = unsafe { fb.as_mut_ptr().offset((x * fb.pixel_width + y * fb.pitch) as isize) };
        unsafe {
            pixel.byte_offset((fb.color_info.red_field_position   / 8).into()).write_volatile(color.r); // red
            pixel.byte_offset((fb.color_info.green_field_position / 8).into()).write_volatile(color.g); // green
            pixel.byte_offset((fb.color_info.blue_field_position  / 8).into()).write_volatile(color.b); // blue
        }
    }
}
