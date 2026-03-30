//! Movie Fragment Header Box (`mfhd`) implementation.
//!
//! The Movie Fragment Header Box contains a sequence number that identifies
//! the order of movie fragments. Sequence numbers start at 1 and increment
//! for each subsequent fragment.
//!
//! This box is required within every Movie Fragment Box (`moof`).

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

define_box_flags!(
    /// Flags for the Movie Fragment Header Box (`mfhd`).
    ///
    /// Reserved (should be 0).
    MfhdFlags {}
);

/// Movie Fragment Header Box (`mfhd`).
///
/// Identifies a movie fragment by its sequence number. Fragments must
/// be processed in sequence number order to correctly reconstruct the
/// movie timeline.
///
/// # Structure
///
/// - `version`: Box version (should be 0).
/// - `flags`: Reserved (should be 0).
/// - `sequence_number`: 1-based fragment order (first fragment is 1).
///
/// # Example
///
/// ```
/// use mp4_bmff::BoxDecode;
/// use mp4_bmff::boxes::bmff::MfhdBox;
///
/// let data: [u8; 8] = [
///     0x00,                   // version = 0
///     0x00, 0x00, 0x00,       // flags
///     0x00, 0x00, 0x00, 0x2A, // sequence_number = 42
/// ];
///
/// let mfhd = MfhdBox::decode(&data).unwrap();
/// assert_eq!(mfhd.sequence_number, 42);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct MfhdBox {
    /// Box version (should be 0).
    pub version: u8,
    /// Reserved flags (should be 0).
    pub flags: MfhdFlags,
    /// Fragment sequence number (1-based, increments per fragment).
    pub sequence_number: u32,
}

impl MfhdBox {
    /// Creates a new `MfhdBox` with the specified sequence number.
    pub fn new(sequence_number: u32) -> Self {
        Self {
            version: 0,
            flags: MfhdFlags::empty(),
            sequence_number,
        }
    }
}

impl BoxCodec for MfhdBox {
    fn boxtype(&self) -> BoxType {
        BoxType::MFHD
    }
}

impl BoxDecode<'_> for MfhdBox {
    fn decode(bytes: &'_ [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = MfhdFlags::from_be_bytes(cur.read_array::<3>()?);

        let sequence_number = cur.read_u32_be()?;

        Ok(MfhdBox {
            version,
            flags,
            sequence_number,
        })
    }
}

impl BoxEncode for MfhdBox {
    #[inline]
    fn encoded_len(&self) -> usize {
        1 // version
            + 3 // flags
            + 4 // sequence_number
    }

    fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

        cur.write_u8(self.version)?;
        cur.write_array(&self.flags.to_be_bytes())?;

        cur.write_u32_be(self.sequence_number)?;

        Ok(cur.position())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_data() -> [u8; 8] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x2A, // sequence_number = 42
        ]
    }

    #[test]
    fn test_mfhd_box_decode() {
        let data = raw_data();
        let mfhd = MfhdBox::decode(&data).unwrap();

        assert_eq!(mfhd.version, 0);
        assert_eq!(mfhd.flags.bits(), 0);
        assert_eq!(mfhd.sequence_number, 42);
    }

    #[test]
    fn test_mfhd_box_decode_truncated() {
        let data: [u8; 4] = [0x00; 4];
        let result = MfhdBox::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_mfhd_box_round_trip() {
        let original = raw_data();
        let mfhd = MfhdBox::decode(&original).unwrap();

        let mut encoded = [0u8; 8];
        let len = mfhd.encode_into(&mut encoded).unwrap();

        assert_eq!(len, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }
}
