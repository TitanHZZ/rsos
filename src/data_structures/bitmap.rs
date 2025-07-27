use core::fmt;

// TODO: write tests for this

/// A bitmap with *BLOCKS* blocks of 8 bits (1 block --> 8 bits).
/// 
/// This owns the bitmap itself.
pub struct Bitmap<const BLOCKS: usize> {
    data: [u8; BLOCKS],
    bit_len: usize,
}

impl<const BLOCKS: usize> Bitmap<BLOCKS> {
    /// Creates a new **Bitmap** that holds a maximum of `BLOCKS` * 8 bits.
    /// This bitmap will be zeroed out.
    /// 
    /// `bit_len` is an optional parameter that specifies how many of the bits from `BLOCKS` * 8 will actually be used.
    /// 
    /// If `bit_len` is bigger than the maximum number of bits, this will panic.
    /// 
    /// In case this parameter is **None**, all the bits available will be used.
    pub const fn new(bit_len: Option<usize>) -> Self {
        // get the real length
        let bit_len = match bit_len {
            Some(len) => len,
            None => BLOCKS * 8,
        };

        // make sure that the real length is a valid value
        assert!(bit_len <= BLOCKS * 8);

        Bitmap {
            data: [0; BLOCKS],
            bit_len,
        }
    }

    /// Get the value (true/false) in the position `bit` that works as an index in the array of bits.
    pub fn get(&self, bit: usize) -> Option<bool> {
        if bit >= self.bit_len {
            return  None;
        }

        let (byte, offset) = self.bit_pos(bit);
        match (self.data[byte] & (1 << offset)) >> offset {
            0 => Some(false),
            1 => Some(true),
            _ => unreachable!(),
        }
    }

    /// Set bit at position `bit` to `value`.
    /// Panic if `bit` bigger than `BLOCKS` * 8 or `bit_len` if provided during the creation of the bitmap.
    pub fn set(&mut self, bit: usize, value: bool) {
        assert!(bit < self.bit_len);
        let (byte, offset) = self.bit_pos(bit);
        self.data[byte] &= !(1 << offset);
        self.data[byte] |= (value as u8) << offset;
    }

    /// Get the real bitmap len in bytes.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Get the bitmap len in bits.
    pub fn bit_len(&self) -> usize {
        self.bit_len
    }

    /// Get a const ptr to the data.
    pub fn data_ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }

    /// Get a mut ptr to the data.
    pub fn data_ptr_mut(&mut self) -> *mut u8 {
        self.data.as_mut_ptr()
    }

    pub fn iter(&self) -> BitmapIter<'_, BLOCKS> {
        BitmapIter {
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

pub struct BitmapIter<'a, const BLOCKS: usize> {
    curr_bit_idx: usize,
    bitmap: &'a Bitmap<BLOCKS>,
}

impl<'a, const BLOCKS: usize> Iterator for BitmapIter<'a, BLOCKS> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        let bit = self.bitmap.get(self.curr_bit_idx);
        self.curr_bit_idx += 1;
        bit
    }
}

impl<const BLOCKS: usize> fmt::Display for Bitmap<BLOCKS> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.data {
            for offset in 0..8 {
                write!(f, "{}", (byte & (1 << offset)) >> offset)?;
            }
        }

        Ok(())
    }
}
