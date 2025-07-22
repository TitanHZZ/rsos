use core::{fmt, ptr::slice_from_raw_parts_mut};

// TODO: write tests for this

/// A bitmap with a mut ref to the bitmap itself.
pub struct BitmapRefMut<'a> {
    data: &'a mut [u8],
    bit_len: usize,
}

impl<'a> BitmapRefMut<'a> {
    /// Creates a new **BitmapRefMut** that points to `data`.
    /// The data pointed to by `data` will be zeroed out.
    /// 
    /// `bit_len` is an optional parameter that specifies how many of the bits in `data` will actually be used.
    /// 
    /// If `bit_len` is bigger than the total number of bits in `data`, this will panic.
    /// 
    /// In case this parameter is **None**, all the bits available in `data` will be used.
    pub fn new(data: &'a mut [u8], bit_len: Option<usize>) -> Self {
        // get the real length
        let bit_len = match bit_len {
            Some(len) => len,
            None => data.len() * 8,
        };

        // make sure that the real length is a valid value
        assert!(bit_len <= data.len() * 8);

        data.fill(0);
        BitmapRefMut {
            data,
            bit_len,
        }
    }

    /// Creates a **BitmapRefMut** that starts at `data` and has `len` bytes and `len` * 8 bits or `bit_len` bits.
    /// 
    /// If `bit_len` is bigger than `len` * 8, this will panic.
    /// 
    /// In case `bit_len` is **None**, all the bits available in `data` will be used.
    /// 
    /// The data pointed to by `data` with `len` elements will be zeroed out.
    /// 
    /// # Safety
    /// 
    /// The caller must ensure that `data` is valid, points to mapped memory and is big enough to hold `len` elements.
    pub unsafe fn from_raw_parts_mut(data: *mut u8, len: usize, bit_len: Option<usize>) -> Self {
        Self::new(unsafe { &mut *slice_from_raw_parts_mut(data, len) }, bit_len)
    }

    /// Get the value (true/false) in the position `bit` that works as an index in the array of bits.
    pub fn get(&self, bit: usize) -> Option<bool> {
        if bit >= self.bit_len {
            return None;
        }

        let (byte, offset) = self.bit_pos(bit);
        match (self.data[byte] & (1 << offset)) >> offset {
            0 => Some(false),
            1 => Some(true),
            _ => unreachable!(),
        }
    }

    /// Set bit at position `bit` to `value`.
    /// Panic if `bit` bigger than `len` * 8 or `bit_len` if provided during the creation of the bitmap.
    pub fn set(&mut self, bit: usize, value: bool) {
        assert!(bit < self.bit_len);
        let (byte, offset) = self.bit_pos(bit);
        self.data[byte] &= !(1 << offset);
        self.data[byte] |= (value as u8) << offset;
    }

    pub fn iter(&self) -> BitmapRefMutIter<'_> {
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
        for i in 0..self.bit_len {
            for offset in 0..8 {
                write!(f, "{}", (self.data[i] & (1 << offset)) >> offset)?;
            }
        }

        Ok(())
    }
}
