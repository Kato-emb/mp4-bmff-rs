//! Video Media Header Box (`vmhd`) implementation.
//!
//! The Video Media Header Box contains general presentation information
//! independent of the video's coding. It is used for video tracks to
//! specify compositing options like graphics mode and color.

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

define_box_flags!(
    /// Flags for the Video Media Header Box (`vmhd`).
    ///
    /// The default value is 1 for QuickTime compatibility.
    VmhdFlags {}
);

/// Video Media Header Box (`vmhd`).
///
/// Contains information about video presentation such as compositing mode
/// and background color. Present in video tracks within the Media Information Box.
///
/// # Structure
///
/// - `version`: Box version (should be 0).
/// - `flags`: Should be 1 for QuickTime compatibility.
/// - `graphics_mode`: Compositing mode for video (0 = copy).
/// - `opcolor`: RGB color values used with certain graphics modes.
#[derive(Debug, Clone, Copy)]
pub struct VmhdBox {
    /// Box version.
    pub version: u8,
    /// Box flags.
    pub flags: VmhdFlags,
    /// Graphics mode.
    pub graphics_mode: u16,
    /// 3 color values
    pub opcolor: [u16; 3],
}

impl Default for VmhdBox {
    fn default() -> Self {
        VmhdBox {
            version: 0,
            flags: VmhdFlags::new(1), // default to quicktime compatibility
            graphics_mode: 0,
            opcolor: [0, 0, 0],
        }
    }
}

impl BoxCodec for VmhdBox {
    fn boxtype(&self) -> BoxType {
        BoxType::VMHD
    }
}

impl BoxDecode<'_> for VmhdBox {
    fn decode(bytes: &[u8]) -> Result<Self> {
        let mut cursor = ReadCursor::new(bytes);

        let version = cursor.read_u8()?;
        let flags = VmhdFlags::from_be_bytes(cursor.read_array::<3>()?);

        let graphics_mode = cursor.read_u16_be()?;
        let mut opcolor = [0u16; 3];
        for color in &mut opcolor {
            *color = cursor.read_u16_be()?;
        }

        Ok(VmhdBox {
            version,
            flags,
            graphics_mode,
            opcolor,
        })
    }
}

impl BoxEncode for VmhdBox {
    #[inline]
    fn encoded_len(&self) -> usize {
        4 // version (1) + flags (3)
        + 2 // graphics_mode
        + 6 // opcolor (3 * 2)
    }

    fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

        cur.write_u8(self.version)?;
        cur.write_array(&self.flags.to_be_bytes())?;

        cur.write_u16_be(self.graphics_mode)?;

        for color in &self.opcolor {
            cur.write_u16_be(*color)?;
        }

        Ok(cur.position())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_data() -> [u8; 12] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x01, // flags = 1 (quicktime compatibility)
            0x00, 0x00, // graphics_mode = 0
            0x80, 0x00, // opcolor[0] = 32768
            0x80, 0x00, // opcolor[1] = 32768
            0x80, 0x00, // opcolor[2] = 32768
        ]
    }

    #[test]
    fn test_vmhd_box_decode() {
        let data = raw_data();
        let vmhd = VmhdBox::decode(&data).unwrap();

        assert_eq!(vmhd.version, 0);
        assert_eq!(vmhd.flags.bits(), 1);
        assert_eq!(vmhd.graphics_mode, 0);
        assert_eq!(vmhd.opcolor, [32768, 32768, 32768]);
    }

    #[test]
    fn test_vmhd_box_decode_truncated() {
        let data: [u8; 6] = [0x00; 6];
        let result = VmhdBox::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_vmhd_box_round_trip() {
        let original = raw_data();
        let vmhd = VmhdBox::decode(&original).unwrap();

        let mut encoded = [0u8; 12];
        let len = vmhd.encode_into(&mut encoded).unwrap();

        assert_eq!(len, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }
}
