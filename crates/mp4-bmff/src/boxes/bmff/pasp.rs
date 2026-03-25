use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

/// Pixel Aspect Ratio Box (`pasp`).
///
/// Defines the horizontal and vertical spacing of pixels in a visual sample, allowing
/// for non-square pixels. This box is typically used within video sample entries to
/// specify the aspect ratio of the video content.
///
/// # Structure
/// - `h_spacing`: Horizontal spacing of pixels (unsigned 32-bit integer).
/// - `v_spacing`: Vertical spacing of pixels (unsigned 32-bit integer).
#[derive(Debug, Clone, Copy)]
pub struct PaspBox {
    /// Horizontal spacing of pixels in the visual sample.
    pub h_spacing: u32,
    /// Vertical spacing of pixels in the visual sample.
    pub v_spacing: u32,
}

impl Default for PaspBox {
    fn default() -> Self {
        PaspBox {
            h_spacing: 1,
            v_spacing: 1,
        }
    }
}

impl BoxCodec for PaspBox {
    fn boxtype(&self) -> BoxType {
        BoxType::PASP
    }
}

impl<'de> BoxDecode<'de> for PaspBox {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let h_spacing = cur.read_u32_be()?;
        let v_spacing = cur.read_u32_be()?;

        Ok(PaspBox {
            h_spacing,
            v_spacing,
        })
    }
}

impl BoxEncode for PaspBox {
    fn encoded_len(&self) -> usize {
        8 // h_spacing (4 bytes) + v_spacing (4 bytes)
    }

    fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

        cur.write_u32_be(self.h_spacing)?;
        cur.write_u32_be(self.v_spacing)?;

        Ok(cur.position())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_data() -> [u8; 8] {
        [
            0x00, 0x00, 0x00, 0x0A, // h_spacing = 10 (4 bytes)
            0x00, 0x00, 0x00, 0x0B, // v_spacing = 11 (4 bytes)
        ]
    }

    #[test]
    fn test_pasp_box_decode() {
        let data = raw_data();
        let pasp = PaspBox::decode(&data).unwrap();

        assert_eq!(pasp.h_spacing, 10);
        assert_eq!(pasp.v_spacing, 11);
    }

    #[test]
    fn test_pasp_box_decode_truncated() {
        let data: [u8; 4] = [0x00; 4];
        let result = PaspBox::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_pasp_box_round_trip() {
        let original = raw_data();
        let pasp = PaspBox::decode(&original).unwrap();

        let mut encoded = [0u8; 8];
        let len = pasp.encode_into(&mut encoded).unwrap();

        assert_eq!(len, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }
}
