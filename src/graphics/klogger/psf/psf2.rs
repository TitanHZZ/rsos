use crate::graphics::klogger::psf::PsfError;

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
pub(super) struct Psf2Font<'a> {
    header: &'a Psf2Header,
    glyphs: &'a [u8],
    unicode_mappings: &'a[u8],
}

enum UnicodeTableDecodeState {
    SingleEntries,
    MultipleEntries,
}

impl<'a> Psf2Font<'a> {
    pub(super) fn from_bytes(font_bytes: &'a [u8]) -> Result<Self, PsfError> {
        if font_bytes.len() < size_of::<Psf2Header>() {
            return Err(PsfError::MalformedHeader);
        }

        let header = unsafe { &*(font_bytes.as_ptr() as *const Psf2Header) };
        if header.magic != 0x864ab572 {
            return Err(PsfError::WrongMagicValue);
        }

        // only version 0 is unsupported (as it is the only one that exists, for now)
        if header.version != 0 {
            return Err(PsfError::UnsupportedVersion);
        }

        let glyphs_offset  = header.headersize as usize;
        let glyphs_size    = header.numglyph as usize * header.bytesperglyph as usize;
        let unicode_offset = glyphs_offset + glyphs_size;

        let (glyphs, unicode_mappings) = if (header.flags & 0x1) != 0 {
            // the unicode mapping table must have positive size
            if unicode_offset >= font_bytes.len() {
                return Err(PsfError::MalformedUnicodeMappingTable);
            }

            (&font_bytes[glyphs_offset..unicode_offset], &font_bytes[unicode_offset..])
        } else {
            // sanity check the bitmap glyphs size
            if (glyphs_offset + glyphs_size) > font_bytes.len() {
                return Err(PsfError::MalformedGlyphsTable);
            }

            (&font_bytes[glyphs_offset..glyphs_offset + glyphs_size], &font_bytes[0..0])
        };

        Ok(Psf2Font { header, glyphs, unicode_mappings })
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
        const START_SEQ: u8 = 0xFE;
        const END_REC: u8 = 0xFF;

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
                    b => {
                        match state {
                            UnicodeTableDecodeState::SingleEntries => {
                                let n = Psf2Font::next_utf8_len(b)?;
                                if p + n > mapping_entry.len() {
                                    return None;
                                }

                                if &mapping_entry[p..p + n] == chr {
                                    return Some(i as u32);
                                }

                                p += n;
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
        let mut buf = [0u8; 4]; // enough for any UTF-8 character
        let bytes = chr.encode_utf8(&mut buf).as_bytes();

        // check if the character is simple ASCII
        if bytes.len() == 1 && bytes[0] <= 0x7f {
            return self.get_glyph_by_idx(bytes[0] as u32);
        }

        if let Some(idx) = self.scan_unicode_table(bytes) {
            return self.get_glyph_by_idx(idx);
        }

        None
    }

    pub(super) fn pixel_width(&self) -> u32 {
        self.header.width
    }

    pub(super) fn pixel_height(&self) -> u32 {
        self.header.height
    }
}
