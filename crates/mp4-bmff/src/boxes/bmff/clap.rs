use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;
use crate::types::FractionU32;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

/// Clean Aperture Box (`clap`).
///
/// Defines the clean aperture of a video sample, which specifies the region of the
/// video frame that contains valid image data. This box is typically used within
/// video sample entries to indicate the intended display area of the video content.
///
/// # Structure
/// - `clean_aperture_width`: Clean aperture width in counted pixels (16.16 fixed-point).
/// - `clean_aperture_height`: Clean aperture height in counted pixels (16.16 fixed-point).
/// - `horiz_offset`: Horizontal offset of clean aperture center minus `(width-1)/2` (16.16 fixed-point). Typically 0.
/// - `vert_offset`: Vertical offset of clean aperture center minus `(height-1)/2` (16.16
#[derive(Debug, Clone, Copy)]
pub struct ClapBox {
    /// Clean aperture width, in counted pixels, of the video image.
    pub clean_aperture_width: FractionU32,
    /// Clean aperture height, in counted pixels, of the video image.
    pub clean_aperture_height: FractionU32,
    /// Horizontal offset of clean aperture centre minus `(width-1)/2`. Typically 0.
    pub horiz_offset: FractionU32,
    /// Vertical offset of clean aperture centre minus `(height-1)/2`. Typically 0.
    pub vert_offset: FractionU32,
}

impl Default for ClapBox {
    fn default() -> Self {
        ClapBox {
            clean_aperture_width: FractionU32::new(1, 1),
            clean_aperture_height: FractionU32::new(1, 1),
            horiz_offset: FractionU32::new(0, 1),
            vert_offset: FractionU32::new(0, 1),
        }
    }
}

impl BoxCodec for ClapBox {
    fn boxtype(&self) -> BoxType {
        BoxType::CLAP
    }
}

impl<'de> BoxDecode<'de> for ClapBox {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let clean_aperture_width_n = cur.read_u32_be()?;
        let clean_aperture_width_d = cur.read_u32_be()?;
        let clean_aperture_width = FractionU32::new(clean_aperture_width_n, clean_aperture_width_d);
        let clean_aperture_height_n = cur.read_u32_be()?;
        let clean_aperture_height_d = cur.read_u32_be()?;
        let clean_aperture_height =
            FractionU32::new(clean_aperture_height_n, clean_aperture_height_d);

        let horiz_offset_n = cur.read_u32_be()?;
        let horiz_offset_d = cur.read_u32_be()?;
        let horiz_offset = FractionU32::new(horiz_offset_n, horiz_offset_d);
        let vert_offset_n = cur.read_u32_be()?;
        let vert_offset_d = cur.read_u32_be()?;
        let vert_offset = FractionU32::new(vert_offset_n, vert_offset_d);

        Ok(ClapBox {
            clean_aperture_width,
            clean_aperture_height,
            horiz_offset,
            vert_offset,
        })
    }
}

impl BoxEncode for ClapBox {
    fn encoded_len(&self) -> usize {
        32 // 8 fields * 4 bytes each
    }

    fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

        let (clean_aperture_width_n, clean_aperture_width_d) = self.clean_aperture_width.into_raw();
        cur.write_u32_be(clean_aperture_width_n)?;
        cur.write_u32_be(clean_aperture_width_d)?;
        let (clean_aperture_height_n, clean_aperture_height_d) =
            self.clean_aperture_height.into_raw();
        cur.write_u32_be(clean_aperture_height_n)?;
        cur.write_u32_be(clean_aperture_height_d)?;

        let (horiz_offset_n, horiz_offset_d) = self.horiz_offset.into_raw();
        cur.write_u32_be(horiz_offset_n)?;
        cur.write_u32_be(horiz_offset_d)?;
        let (vert_offset_n, vert_offset_d) = self.vert_offset.into_raw();
        cur.write_u32_be(vert_offset_n)?;
        cur.write_u32_be(vert_offset_d)?;

        Ok(cur.position())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_data() -> [u8; 32] {
        [
            0x00, 0x00, 0x02, 0xD0, // clean_aperture_width numerator = 720
            0x00, 0x00, 0x00, 0x01, // clean_aperture_width denominator = 1
            0x00, 0x00, 0x01, 0xE0, // clean_aperture_height numerator = 480
            0x00, 0x00, 0x00, 0x01, // clean_aperture_height denominator = 1
            0x00, 0x00, 0x00, 0x00, // horiz_offset numerator = 0
            0x00, 0x00, 0x00, 0x01, // horiz_offset denominator = 1
            0x00, 0x00, 0x00, 0x00, // vert_offset numerator = 0
            0x00, 0x00, 0x00, 0x01, // vert_offset denominator = 1
        ]
    }

    #[test]
    fn test_clap_box_decode() {
        let data = raw_data();
        let clap = ClapBox::decode(&data).unwrap();

        assert_eq!(clap.clean_aperture_width.into_raw(), (720, 1));
        assert_eq!(clap.clean_aperture_height.into_raw(), (480, 1));
        assert_eq!(clap.horiz_offset.into_raw(), (0, 1));
        assert_eq!(clap.vert_offset.into_raw(), (0, 1));
    }

    #[test]
    fn test_clap_box_decode_truncated() {
        let data: [u8; 16] = [0x00; 16];
        let result = ClapBox::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_clap_box_round_trip() {
        let original = raw_data();
        let clap = ClapBox::decode(&original).unwrap();

        let mut encoded = [0u8; 32];
        let len = clap.encode_into(&mut encoded).unwrap();

        assert_eq!(len, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }
}
