mod psf1;
mod psf2;

use crate::graphics::klogger::psf::{psf1::Psf1Font, psf2::Psf2Font};

// TODO: what about the multiple, sequential, entries in the unicode table for the PSF1/2 fonts that match to a single glyph??
//       do i have to worry about that??

// Useful Resources:
// - https://docs.rs/spleen-font/latest/spleen_font/index.html
// - https://en.wikipedia.org/wiki/PC_Screen_Font
// - http://justsolve.archiveteam.org/wiki/PC_Screen_Font

#[derive(Debug)]
pub(super) enum PsfError {
    MalformedHeader,
    MalformedUnicodeMappingTable,
    MalformedGlyphsTable,
    WrongMagicValue,
    UnsupportedVersion,
}

enum PsfType<'a> {
    Type1(Psf1Font<'a>),
    Type2(Psf2Font<'a>),
}

pub(super) struct Psf<'a>(PsfType<'a>);

impl<'a> Psf<'a> {
    /// Creates a Pc Screen Font (PSF) from `font_bytes`.
    /// 
    /// In case the provided `font_bytes` cannot be parsed as either PSF1 or PSF2,
    /// their parsing errors will be returned in order PSF1 and then PSF2.
    pub(super) fn from_bytes(font_bytes: &'a [u8]) -> Result<Self, (PsfError, PsfError)> {
        let psf1_err = match Psf1Font::from_bytes(font_bytes) {
            Ok(font) => return Ok(Self(PsfType::Type1(font))),
            Err(psf1_err) => psf1_err,
        };

        let psf2_err = match Psf2Font::from_bytes(font_bytes) {
            Ok(font) => return Ok(Self(PsfType::Type2(font))),
            Err(psf2_err) => psf2_err,
        };

        Err((psf1_err, psf2_err))
    }

    pub(super) fn get_glyph(&self, chr: char) -> Option<&[u8]> {
        match self.0 {
            PsfType::Type1(ref font) => font.get_glyph(chr),
            PsfType::Type2(ref font) => font.get_glyph(chr),
        }
    }

    pub(super) fn pixel_width(&self) -> u32 {
        match self.0 {
            PsfType::Type1(ref font) => font.pixel_width(),
            PsfType::Type2(ref font) => font.pixel_width()
        }
    }

    pub(super) fn pixel_height(&self) -> u32 {
        match self.0 {
            PsfType::Type1(ref font) => font.pixel_height(),
            PsfType::Type2(ref font) => font.pixel_height()
        }
    }
}
