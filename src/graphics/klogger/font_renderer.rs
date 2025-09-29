use crate::{graphics::{framebuffer::{FrameBufferColor, Framebuffer}, klogger::painter::KLoggerPainter}, serial_println};

// https://docs.rs/spleen-font/latest/spleen_font/index.html

// const FONT: &[u8] = include_bytes!("fonts/Lat2-Terminus16.psfu");
// const FONT: &[u8] = include_bytes!("fonts/ter-212n.psf");
const FONT: &[u8] = include_bytes!("fonts/spleen-12x24.psfu");

#[repr(C)]
struct Psf2Header {
    magic: u32,
    version: u32,
    headersize: u32,
    flags: u32,
    numglyph: u32,
    bytesperglyph: u32,
    height: u32,
    width: u32,
}

#[repr(C)]
struct Psf2Font<'a> {
    header: &'a Psf2Header,
    glyphs: &'a [u8],
    unicode_mappings: &'a[u8],
}

enum UnicodeTableDecodeState {
    Invalid,
    SingleEntries,
    MultipleEntries,
}

// TODO: this should return proper errors
impl<'a> Psf2Font<'a> {
    pub fn from_bytes(data: &'a [u8]) -> Option<Self> {
        if data.len() < core::mem::size_of::<Psf2Header>() {
            return None;
        }

        // TODO: it should be possible to parse PSF1 fonts with this same PSF2 font parser (just need header adjustments i think)
        let header = unsafe { &*(data.as_ptr() as *const Psf2Header) };
        if header.magic != 0x864ab572 {
            serial_println!("invalid font header magic: {:#x}", header.magic);
            return None;
        }

        let glyphs_offset  = header.headersize as usize;
        let glyphs_size    = header.numglyph as usize * header.bytesperglyph as usize;
        let unicode_offset = glyphs_offset + glyphs_size;

        let glyphs = &data[glyphs_offset..unicode_offset];
        let unicode_mappings = if (header.flags & 0x1) == 1 {
            &data[unicode_offset..]
        } else {
            &[]
        };

        Some(Psf2Font {
            header,
            glyphs,
            unicode_mappings,
        })
    }

    fn get_glyph_by_idx(&self, idx: u32) -> Option<&'a [u8]> {
        if idx >= self.header.numglyph {
            return None;
        }

        let start = idx as usize * self.header.bytesperglyph as usize;
        let end   = start + self.header.bytesperglyph as usize;
        Some(&self.glyphs[start..end])
    }

    /// Decode exactly one valid UTF-8 scalar and return its length in bytes.
    /// 
    /// Returns None on malformed UTF-8.
    //
    // Every UTF-8 sequence starts with a leading byte that indicates the number of bytes in the sequence.
    // The leading byte is followed by continuation bytes that each start with the bits 10xxxxxx.
    // 
    // one byte:       0.......
    // two bytes:      110..... 10......
    // three bytes:    1110.... 10...... 10......
    // four bytes:     11110... 10...... 10...... 10......
    fn next_utf8_len(b: u8) -> Option<usize> {
        Some(match b {
            0x00..=0x7F => 1, // 0xxxxxxx
            0xC2..=0xDF => 2, // 110xxxxx
            0xE0..=0xEF => 3, // 1110xxxx
            0xF0..=0xF4 => 4, // 11110xxx
            _ => return None, // continuation or invalid
        })
    }

    fn scan_unicode_table(&self, chr: &[u8]) -> Option<u32> {
        let mut glyph_idx: u32 = 0;
        let mut p: usize = 0;

        const START_SEQ: u8 = 0xFE;
        const END_REC: u8 = 0xFF;

        // let mut state = UnicodeTableDecodeState::SingleEntries;
        // for (i, entry) in self.unicode_mappings.split(|e| *e == END_REC).enumerate() {
        //     while p < entry.len() {
        //         match entry[p] {
        //             START_SEQ => {
        //                 state = UnicodeTableDecodeState::MultipleEntries;
        //                 p += 1;
        //             },
        //             END_REC => {
        //                 state = UnicodeTableDecodeState::SingleEntries;
        //                 p = 0;
        //             },
        //             b => {
        //                 match state {
        //                     UnicodeTableDecodeState::SingleEntries => {
        //                         let mut n = Psf2Font::next_utf8_len(b)?;
        //                         while p + n < entry.len() && !matches!(entry[p + n], START_SEQ) && &entry[p..(p + n)] != chr {
        //                             p += n;
        //                             n = Psf2Font::next_utf8_len(b)?;
        //                         }
        //                         if &entry[p..(p + n)] == chr {
        //                             return Some(i as u32);
        //                         }
        //                     },
        //                     _ => return None,
        //                 }
        //             },
        //         }
        //     }
        // }




        // parse the UTF-8 mappings
        while p < self.unicode_mappings.len() {
            // check for double 0xFF (end of the unicode mappings)
            if self.unicode_mappings.get(p..p + 2) == Some(&[END_REC, END_REC]) {
                return None;
            }

            // parse each unicode mappings entry
            loop {
                match self.unicode_mappings[p] {
                    START_SEQ => p += 1,
                    END_REC => {
                        glyph_idx += 1;
                        p += 1;
                        break;
                    }
                    b => {
                        let start = p;
                        p += Psf2Font::next_utf8_len(b)?;
                        while p < self.unicode_mappings.len() && !matches!(self.unicode_mappings[p], START_SEQ | END_REC) {
                            p += Psf2Font::next_utf8_len(self.unicode_mappings[p])?;
                        }

                        if &self.unicode_mappings[start..p] == chr {
                            return Some(glyph_idx);
                        }
                    }
                }
            }
        }

        None
    }

    fn get_glyph(&self, chr: &[u8]) -> Option<&[u8]> {
        // check if the character is simple ASCII
        if chr.len() == 1 && chr[0] <= 0x7f {
            return self.get_glyph_by_idx(chr[0] as u32);
        }

        if let Some(idx) = self.scan_unicode_table(chr) {
            return self.get_glyph_by_idx(idx);
        }

        None
    }
}

pub(in crate::graphics::klogger) struct FontRenderer<'a> {
    font: Psf2Font<'a>,
}

impl<'a> FontRenderer<'a> {
    // TODO: this should not "crash" with an invalid font
    pub(in crate::graphics::klogger) fn new() -> Self {
        Self {
            font: Psf2Font::from_bytes(FONT).expect("Invalid PSF font"),
        }
    }

    pub(in crate::graphics::klogger) fn pixel_width(&self) -> u32 {
        self.font.header.width
    }

    pub(in crate::graphics::klogger) fn draw_char(&self, fb: &Framebuffer, chr: &[u8], xpos: u32, ypos: u32, color: FrameBufferColor) {
        if let Some(glyph) = self.font.get_glyph(chr) {
            let bytes_per_row = self.font.header.width.div_ceil(8) as usize;

            for y in 0..self.font.header.height as usize {
                for x in 0..self.font.header.width as usize {
                    let byte = glyph[y * bytes_per_row + (x / 8)];
                    if (byte >> (7 - (x % 8))) & 1 != 0 {
                        KLoggerPainter::put_pixel(fb, xpos + x as u32, ypos + y as u32, color);
                    }
                }
            }
        }
    }
}
