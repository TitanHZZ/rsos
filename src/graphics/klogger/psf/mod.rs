mod psf1;
mod psf2;

use crate::graphics::klogger::psf::{psf1::Psf1Font, psf2::Psf2Font};

// Useful Resources:
// - https://docs.rs/spleen-font/latest/spleen_font/index.html
// - https://en.wikipedia.org/wiki/PC_Screen_Font
// - http://justsolve.archiveteam.org/wiki/PC_Screen_Font

enum PsfType<'a> {
    Type1(Psf1Font<'a>),
    Type2(Psf2Font<'a>),
}

pub(super) struct PSF<'a>(PsfType<'a>);

impl<'a> PSF<'a> {
    pub(super) fn from_bytes(font_bytes: &'a [u8]) -> Self {
        if let Ok(font) = Psf2Font::from_bytes(font_bytes) {
            return Self(PsfType::Type2(font));
        }

        if let Ok(font) = Psf1Font::from_bytes(font_bytes) {
            return Self(PsfType::Type1(font));
        }

        todo!()
    }

    pub(super) fn get_glyph(&self, chr: &[u8]) -> Option<&[u8]> {
        match self.0 {
            PsfType::Type1(ref font) => todo!(),
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
