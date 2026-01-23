use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;

use super::FullBoxFlags;

/// Movie Extends Header Box (`mehd`).
///
/// This box provides the overall duration of the fragmented movie,
/// including the duration of all fragments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MehdBox {
    /// The version of the box (0 or 1).
    pub version: u8,
    /// The flags of the box.
    pub flags: MehdFlags,
    /// The overall duration of the movie including all fragments.
    /// In the timescale of the movie header.
    pub fragment_duration: u64,
}

/// Specification for the Movie Extends Header Box (`mehd`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MehdSpec;

/// Flags for the Movie Extends Header Box (`mehd`).
pub type MehdFlags = FullBoxFlags<MehdSpec>;

impl BoxCodec for MehdBox {
    fn boxtype(&self) -> BoxType {
        BoxType::MEHD
    }
}

impl BoxDecode<'_> for MehdBox {
    fn decode(bytes: &[u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = MehdFlags::from_bytes(cur.read_array()?);

        let fragment_duration = match version {
            0 => cur.read_u32_be()? as u64,
            1 => cur.read_u64_be()?,
            v => {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxVersion {
                        reason: "mehd version must be 0 or 1",
                        got: v,
                    },
                    BoxType::MEHD,
                ));
            }
        };

        Ok(MehdBox {
            version,
            flags,
            fragment_duration,
        })
    }
}

impl TryFrom<&[u8]> for MehdBox {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self> {
        MehdBox::decode(value)
    }
}

impl BoxEncode for MehdBox {
    #[inline]
    fn encoded_len(&self) -> usize {
        let base_len = 4; // version(1) + flags(3)
        let duration_len = match self.version {
            0 => 4, // fragment_duration(4)
            _ => 8, // fragment_duration(8)
        };

        base_len + duration_len
    }

    fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

        cur.write_u8(self.version)?;
        cur.write_array(&self.flags.to_bytes())?;

        match self.version {
            0 => cur.write_u32_be(self.fragment_duration as u32)?,
            _ => cur.write_u64_be(self.fragment_duration)?,
        }

        Ok(cur.position())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_v0() {
        let original = MehdBox {
            version: 0,
            flags: MehdFlags::empty(),
            fragment_duration: 12345,
        };

        let mut buf = vec![0u8; 32];
        original.encode_into(&mut buf).unwrap();

        let parsed = MehdBox::decode(&buf).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn round_trip_v1() {
        let original = MehdBox {
            version: 1,
            flags: MehdFlags::empty(),
            fragment_duration: 0x1_0000_0000,
        };

        let mut buf = vec![0u8; 32];
        original.encode_into(&mut buf).unwrap();

        let parsed = MehdBox::decode(&buf).unwrap();
        assert_eq!(parsed, original);
    }
}
