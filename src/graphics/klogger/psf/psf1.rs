
#[repr(C)]
struct Psf1Header {
    magic: u16,
    mode: u8,
    charsize: u8,
}

#[repr(C)]
pub(super) struct Psf1Font<'a> {
    header: &'a Psf1Header,
    glyphs: &'a [u8],
    unicode_mappings: &'a[u8],

    numglyph: u32,
}

#[derive(Debug)]
pub(super) enum Psf1FontError {
    MalformedHeader,
    MalformedUnicodeMappingTable,
    MalformedGlyphsTable,
    WrongMagicValue,
    UnsupportedVersion,
}

impl<'a> Psf1Font<'a> {
    pub(super) fn from_bytes(font_bytes: &'a [u8]) -> Result<Self, Psf1FontError> {
        if font_bytes.len() < size_of::<Psf1Header>() {
            return Err(Psf1FontError::MalformedHeader)
        }

        let header = unsafe { &*(font_bytes.as_ptr() as *const Psf1Header) };
        if header.magic != 0x0436 {
            return Err(Psf1FontError::WrongMagicValue);
        }

        // check if the font contains 512 glyphs or just 256
        let numglyph = if (header.mode & 0x1) == 1 {
            512
        } else {
            256
        };

        let glyphs_offset  = size_of::<Psf1Header>();
        let glyphs_size    = numglyph * header.charsize as usize;
        let unicode_offset = glyphs_offset + glyphs_size;

        let (glyphs, unicode_mappings) = if (header.mode & 0x2) == 1 {
            // the unicode mapping table must have positive size
            if unicode_offset >= font_bytes.len() - 1 {
                return Err(Psf1FontError::MalformedUnicodeMappingTable);
            }

            (&font_bytes[glyphs_offset..unicode_offset], &font_bytes[unicode_offset..])
        } else {
            // sanity check the bitmap glyphs size
            if (glyphs_offset + glyphs_size) > font_bytes.len() {
                return Err(Psf1FontError::MalformedGlyphsTable);
            }

            (&font_bytes[glyphs_offset..glyphs_offset + glyphs_size], &font_bytes[0..0])
        };

        // Note: bits 0x4 and 0x5 are also used, but i am not sure what their purpose is
        Ok(Psf1Font { header, glyphs, unicode_mappings, numglyph: numglyph as u32 })
    }

    pub(super) const fn pixel_width(&self) -> u32 {
        8
    }

    pub(super) fn pixel_height(&self) -> u32 {
        self.header.charsize as u32
    }
}
