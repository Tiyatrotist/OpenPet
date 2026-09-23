use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitResult {
    /// Pixel is solid/opaque enough to receive mouse and gesture events.
    Hit,
    /// Pixel is transparent; Windows should pass mouse event through to underlying window.
    TransparentPassThrough,
}

/// Compact 1-bit per pixel bitset hit-testing mask.
///
/// Avoids costly GPU texture readbacks and provides instant sub-millisecond click tests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HitMask {
    pub width: u32,
    pub height: u32,
    pub bits: Vec<u8>,
}

impl HitMask {
    /// Creates a new HitMask initialized with transparent pixels.
    pub fn new(width: u32, height: u32) -> Self {
        let byte_len = (width as usize * height as usize).div_ceil(8);
        Self {
            width,
            height,
            bits: vec![0u8; byte_len],
        }
    }

    /// Mark a pixel as solid (hit).
    pub fn set_pixel(&mut self, x: u32, y: u32, solid: bool) {
        if x >= self.width || y >= self.height {
            return;
        }
        let bit_index = (y as usize * self.width as usize) + x as usize;
        let byte_index = bit_index / 8;
        let bit_offset = bit_index % 8;

        if solid {
            self.bits[byte_index] |= 1 << bit_offset;
        } else {
            self.bits[byte_index] &= !(1 << bit_offset);
        }
    }

    /// Evaluates if coordinate within sprite frame hits opaque pet area.
    pub fn test_point(&self, x: u32, y: u32) -> HitResult {
        if x >= self.width || y >= self.height {
            return HitResult::TransparentPassThrough;
        }

        let bit_index = (y as usize * self.width as usize) + x as usize;
        let byte_index = bit_index / 8;
        let bit_offset = bit_index % 8;

        if (self.bits[byte_index] & (1 << bit_offset)) != 0 {
            HitResult::Hit
        } else {
            HitResult::TransparentPassThrough
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hit_mask() {
        let mut mask = HitMask::new(64, 64);
        assert_eq!(mask.test_point(10, 10), HitResult::TransparentPassThrough);

        mask.set_pixel(10, 10, true);
        assert_eq!(mask.test_point(10, 10), HitResult::Hit);
        assert_eq!(mask.test_point(10, 11), HitResult::TransparentPassThrough);
        assert_eq!(mask.test_point(100, 100), HitResult::TransparentPassThrough);
    }
}
