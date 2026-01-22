use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;

use super::FullBoxFlags;

/// Track Fragment Decode Time Box (`tfdt`).
///
/// This box provides the absolute decode time of the first sample in
/// the track fragment, expressed in the media timescale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TfdtBox {
    /// The version of the box (0 or 1).
    pub version: u8,
    /// The flags of the box (should be 0).
    pub flags: TfdtFlags,
    /// The absolute decode time of the first sample in the track fragment,
    /// expressed in the media timescale.
    pub base_media_decode_time: u64,
}

/// Specification for the Track Fragment Decode Time Box (`tfdt`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TfdtSpec;

/// Flags for the Track Fragment Decode Time Box (`tfdt`).
pub type TfdtFlags = FullBoxFlags<TfdtSpec>;

impl TryFrom<&[u8]> for TfdtBox {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self> {
        TfdtBox::decode(value)
    }
}

impl BoxCodec for TfdtBox {
    fn boxtype(&self) -> BoxType {
        BoxType::TFDT
    }
}

impl BoxDecode<'_> for TfdtBox {
    fn decode(bytes: &[u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = TfdtFlags::from_bytes(cur.read_array()?);

        let base_media_decode_time = match version {
            0 => cur.read_u32_be()? as u64,
            1 => cur.read_u64_be()?,
            v => {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxVersion {
                        reason: "tfdt version must be 0 or 1",
                        got: v,
                    },
                    BoxType::TFDT,
                ));
            }
        };

        if !cur.is_empty() {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Extra data after tfdt fields",
                    got: cur.remaining() as u64,
                },
                BoxType::TFDT,
            ));
        }

        Ok(TfdtBox {
            version,
            flags,
            base_media_decode_time,
        })
    }
}

impl BoxEncode for TfdtBox {
    fn encode(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

        cur.write_u8(self.version)?;
        cur.write_array(&self.flags.to_bytes())?;

        match self.version {
            0 => cur.write_u32_be(self.base_media_decode_time as u32)?,
            _ => cur.write_u64_be(self.base_media_decode_time)?,
        }

        Ok(cur.position())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_v0() {
        let original = TfdtBox {
            version: 0,
            flags: TfdtFlags::empty(),
            base_media_decode_time: 12345,
        };

        let mut buf = vec![0u8; 32];
        let written = original.encode(&mut buf).unwrap();

        let parsed = TfdtBox::decode(&buf[..written]).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn round_trip_v1() {
        let original = TfdtBox {
            version: 1,
            flags: TfdtFlags::empty(),
            base_media_decode_time: 0x123456789ABCDEF0,
        };

        let mut buf = vec![0u8; 32];
        let written = original.encode(&mut buf).unwrap();

        let parsed = TfdtBox::decode(&buf[..written]).unwrap();
        assert_eq!(parsed, original);
    }
}
