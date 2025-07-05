use core::fmt;

// TODO: write tests for this

/// A bitmap with *BLOCKS* blocks of 8 bits (1 block --> 8 bits).
/// 
/// This owns the bitmap itself.
pub struct Bitmap<const BLOCKS: usize> {
    data: [u8; BLOCKS],
}

impl<const BLOCKS: usize> Default for Bitmap<BLOCKS> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const BLOCKS: usize> Bitmap<BLOCKS> {
    pub const fn new() -> Self {
        Bitmap {
            data: [0; BLOCKS],
        }
    }

    pub fn get(&self, bit: usize) -> Option<bool> {
        if bit >= size_of::<[u8; BLOCKS]>() * 8 {
            return  None;
        }

        let (byte, offset) = self.bit_pos(bit);
        match (self.data[byte] & (1 << offset)) >> offset {
            0 => Some(false),
            1 => Some(true),
            _ => unreachable!(),
        }
    }

    pub fn set(&mut self, bit: usize, value: bool) {
        assert!(bit < size_of::<[u8; BLOCKS]>() * 8);
        let (byte, offset) = self.bit_pos(bit);
        self.data[byte] &= !(1 << offset);
        self.data[byte] |= (value as u8) << offset;
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
