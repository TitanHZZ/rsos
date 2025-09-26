use crate::graphics::framebuffer::{FrameBufferColor, Framebuffer};

pub struct KLoggerPainter;

impl KLoggerPainter {
    pub fn put_pixel(fb: &Framebuffer, x: u32, y: u32, color: FrameBufferColor) {
        let pixel = (fb.vir_addr + (x * fb.pixel_width + y * fb.pitch) as usize) as *mut u8;
        unsafe {
            pixel.byte_offset((fb.color_info.red_field_position   / 8).into()).write_volatile(color.r); // red
            pixel.byte_offset((fb.color_info.green_field_position / 8).into()).write_volatile(color.g); // green
            pixel.byte_offset((fb.color_info.blue_field_position  / 8).into()).write_volatile(color.b); // blue
        }
    }

    pub fn fill_rect(fb: &Framebuffer, x: u32, y: u32, w: u32, h: u32, color: FrameBufferColor) {
        let mut pixel = fb.vir_addr as *mut u8;

        for _ in 0..w {
            for j in 0..h {
                unsafe {
                    pixel.byte_offset((j * fb.pixel_width + (fb.color_info.red_field_position   as u32 / 8)) as _).write_volatile(color.r); // red
                    pixel.byte_offset((j * fb.pixel_width + (fb.color_info.green_field_position as u32 / 8)) as _).write_volatile(color.g); // green
                    pixel.byte_offset((j * fb.pixel_width + (fb.color_info.blue_field_position  as u32 / 8)) as _).write_volatile(color.b); // blue
                }
            }

            pixel = unsafe { pixel.byte_offset(fb.pitch as _) };
        }
    }
}
