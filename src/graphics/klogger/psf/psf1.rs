use crate::graphics::klogger::psf::PsfError;
use core::slice::from_raw_parts;

#[repr(C)]
struct Psf1Header {
    magic: u16,
    mode: u8,
    bytesperglyph: u8,
}

#[repr(C)]
pub(super) struct Psf1Font<'a> {
    header: &'a Psf1Header,
    glyphs: &'a [u8],
    unicode_mappings: &'a[u16],
    numglyph: u32,
}

enum UnicodeTableDecodeState {
    SingleEntries,
    MultipleEntries,
}

impl<'a> Psf1Font<'a> {
    pub(super) fn from_bytes(font_bytes: &'a [u8]) -> Result<Self, PsfError> {
        if font_bytes.len() < size_of::<Psf1Header>() {
            return Err(PsfError::MalformedHeader)
        }

        let header = unsafe { &*(font_bytes.as_ptr() as *const Psf1Header) };
        if header.magic != 0x0436 {
            return Err(PsfError::WrongMagicValue);
        }

        // check if the font contains 512 glyphs or just 256
        let numglyph = if (header.mode & 0x1) == 1 {
            512
        } else {
            256
        };

        let glyphs_offset  = size_of::<Psf1Header>();
        let glyphs_size    = numglyph * header.bytesperglyph as usize;
        let unicode_offset = glyphs_offset + glyphs_size;

        let (glyphs, unicode_mappings) = if (header.mode & 0x2) != 0 {
            // the unicode mapping table must have positive size
            if unicode_offset >= font_bytes.len() - 1 {
                return Err(PsfError::MalformedUnicodeMappingTable);
            }

            let unicode_mappings = &font_bytes[unicode_offset..];
            if !unicode_mappings.len().is_multiple_of(2) || unicode_mappings.as_ptr().align_offset(2) != 0 {
                return Err(PsfError::MalformedUnicodeMappingTable);
            }

            let unicode_mappings = unsafe { from_raw_parts(unicode_mappings.as_ptr() as *const u16, unicode_mappings.len() / 2) };
            (&font_bytes[glyphs_offset..unicode_offset], unicode_mappings)
        } else {
            // sanity check the bitmap glyphs size
            if (glyphs_offset + glyphs_size) > font_bytes.len() {
                return Err(PsfError::MalformedGlyphsTable);
            }

            // TODO: in case this is not aligned to 2, we could just move the ptr forward
            let unicode_mappings = &font_bytes[0..0];
            if unicode_mappings.as_ptr().align_offset(2) != 0 {
                return Err(PsfError::MalformedUnicodeMappingTable);
            }

            let unicode_mappings = unsafe { from_raw_parts(unicode_mappings.as_ptr() as *const u16, 0) };
            (&font_bytes[glyphs_offset..glyphs_offset + glyphs_size], unicode_mappings)
        };

        // Note: bits 0x4 and 0x5 are also used, but i am not sure what their purpose is
        Ok(Psf1Font { header, glyphs, unicode_mappings, numglyph: numglyph as u32 })
    }

    fn get_glyph_by_idx(&self, idx: u32) -> Option<&'a [u8]> {
        if idx >= self.numglyph {
            return None;
        }

        let start = idx as usize * self.header.bytesperglyph as usize;
        let end   = start + self.header.bytesperglyph as usize;
        Some(&self.glyphs[start..end])
    }

    fn scan_unicode_table(&self, chr: &[u16]) -> Option<u32> {
        const START_SEQ: u16 = 0xFFFE;
        const END_REC: u16 = 0xFFFF;

        let mut p: usize = 0;
        let mut state = UnicodeTableDecodeState::SingleEntries;
        for (i, mapping_entry) in self.unicode_mappings.split(|e| *e == END_REC).enumerate() {
            while p < mapping_entry.len() {
                match mapping_entry[p] {
                    START_SEQ => {
                        state = UnicodeTableDecodeState::MultipleEntries;
                        p += 1;
                    },
                    END_REC => {
                        // this *should* be unreacheble
                        return None;
                    },
                    _ => {
                        match state {
                            UnicodeTableDecodeState::SingleEntries => {
                                if &mapping_entry[p..p + 1] == chr {
                                    return Some(i as u32);
                                }

                                p += 1;
                            },
                            UnicodeTableDecodeState::MultipleEntries => {
                                let start = p;
                                while p < mapping_entry.len() && mapping_entry[p] != START_SEQ {
                                    p += 1;
                                }

                                if &mapping_entry[start..p] == chr {
                                    return Some(i as u32);
                                }
                            },
                        }
                    }
                }
            }

            state = UnicodeTableDecodeState::SingleEntries;
            p = 0;
        }

        None
    }

    pub(super) fn get_glyph(&self, chr: char) -> Option<&[u8]> {
        if chr.len_utf16() != 1 {
            return  None;
        }

        let mut buf = [0u16; 1]; // all characters in PSF1 are encoded in 2 bytes
        let bytes = chr.encode_utf16(&mut buf);

        // check if the character is simple ASCII
        if bytes[0] <= 0x7f {
            return self.get_glyph_by_idx(bytes[0] as u32);
        }

        if let Some(idx) = self.scan_unicode_table(bytes) {
            return self.get_glyph_by_idx(idx);
        }

        None
    }

    pub(super) const fn pixel_width(&self) -> u32 {
        8
    }

    pub(super) fn pixel_height(&self) -> u32 {
        self.header.bytesperglyph as u32
    }
}
