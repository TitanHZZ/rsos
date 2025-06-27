use core::{fmt, ptr::slice_from_raw_parts_mut};

// TODO: write tests for this

/// A bitmap with a mut ref to the bitmap itself.
pub struct BitmapRefMut<'a> {
    data: &'a mut [u8],
}

impl<'a> BitmapRefMut<'a> {
    pub const fn new(data: &'a mut [u8]) -> Self {
        BitmapRefMut {
            data,
        }
    }

    /// Creates a **BitmapRefMut** that starts at `data` and has `len` * 8 bits.
    /// 
    /// # Safety
    /// 
    /// The caller must ensure that `data` is valid, points to mapped memory and is big enough to hold `len` elements.
    pub const unsafe fn from_raw_parts_mut(data: *mut u8, len: usize) -> Self {
        Self::new(unsafe { &mut *slice_from_raw_parts_mut(data, len) })
    }

    pub fn get(&self, bit: usize) -> Option<bool> {
        if bit >= self.data.len() * 8 {
            return None;
        }

        let (byte, offset) = self.bit_pos(bit);
        match (self.data[byte] & (1 << offset)) >> offset {
            0 => Some(false),
            1 => Some(true),
            _ => unreachable!(),
        }
    }

    pub fn set(&mut self, bit: usize, value: bool) {
        assert!(bit < self.data.len() * 8);
        let (byte, offset) = self.bit_pos(bit);
        self.data[byte] &= !(1 << offset);
        self.data[byte] |= (value as u8) << offset;
    }

    pub fn iter(&self) -> BitmapRefMutIter {
        BitmapRefMutIter {
            curr_bit_idx: 0,
            bitmap: self,
        }
    }

    const fn bit_pos(&self, bit: usize) -> (usize, usize) {
        let byte   = bit >> 3; // bit / 8
        let offset = bit & 7;  // bit % 8
        (byte, offset)
    }
}

pub struct BitmapRefMutIter<'a> {
    curr_bit_idx: usize,
    bitmap: &'a BitmapRefMut<'a>,
}

impl<'a> Iterator for BitmapRefMutIter<'a> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        let bit = self.bitmap.get(self.curr_bit_idx);
        self.curr_bit_idx += 1;
        bit
    }
}

impl<'a> fmt::Display for BitmapRefMut<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &*self.data {
            for offset in 0..8 {
                write!(f, "{}", (byte & (1 << offset)) >> offset)?;
            }
        }

        Ok(())
    }
}
