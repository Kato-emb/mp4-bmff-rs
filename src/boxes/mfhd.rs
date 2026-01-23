use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;

use super::FullBoxFlags;

/// Movie Fragment Header Box (`mfhd`).
///
/// This box contains a sequence number, as a safety check. The sequence number
/// is incremented by one for each movie fragment in the file, in the order in
/// which they occur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MfhdBox {
    /// The version of the box (should be 0).
    pub version: u8,
    /// The flags of the box (should be 0).
    pub flags: MfhdFlags,
    /// The sequence number of this fragment.
    pub sequence_number: u32,
}

/// Specification for the Movie Fragment Header Box (`mfhd`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MfhdSpec;

/// Flags for the Movie Fragment Header Box (`mfhd`).
pub type MfhdFlags = FullBoxFlags<MfhdSpec>;

impl BoxCodec for MfhdBox {
    fn boxtype(&self) -> BoxType {
        BoxType::MFHD
    }
}

impl BoxDecode<'_> for MfhdBox {
    fn decode(bytes: &[u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = MfhdFlags::from_bytes(cur.read_array()?);

        let sequence_number = cur.read_u32_be()?;

        Ok(MfhdBox {
            version,
            flags,
            sequence_number,
        })
    }
}

impl TryFrom<&[u8]> for MfhdBox {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self> {
        MfhdBox::decode(value)
    }
}

impl BoxEncode for MfhdBox {
    #[inline]
    fn encoded_len(&self) -> usize {
        4 // version(1) + flags(3)
        + 4 // sequence_number(4)
    }

    fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

        cur.write_u8(self.version)?;
        cur.write_array(&self.flags.to_bytes())?;
        cur.write_u32_be(self.sequence_number)?;

        Ok(cur.position())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let original = MfhdBox {
            version: 0,
            flags: MfhdFlags::empty(),
            sequence_number: 0xDEADBEEF,
        };

        let mut buf = vec![0u8; 32];
        original.encode_into(&mut buf).unwrap();

        let parsed = MfhdBox::decode(&buf).unwrap();
        assert_eq!(parsed, original);
    }
}
