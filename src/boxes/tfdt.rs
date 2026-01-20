use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

use crate::BoxFrame;
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

impl TfdtBox {
    pub(crate) fn parse_in(cur: &mut ReadCursor<'_>) -> Result<TfdtBox> {
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

    /// Parses a `TfdtBox` from the given payload.
    pub fn parse(payload: &[u8]) -> Result<TfdtBox> {
        let mut cur = ReadCursor::new(payload);
        TfdtBox::parse_in(&mut cur)
    }

    /// Returns the size of the payload in bytes.
    pub fn size(&self) -> usize {
        4 + match self.version {
            0 => 4, // base_media_decode_time as u32
            _ => 8, // base_media_decode_time as u64
        }
    }

    pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
        cur.write_u8(self.version)?;
        cur.write_array(&self.flags.to_bytes())?;

        match self.version {
            0 => cur.write_u32_be(self.base_media_decode_time as u32)?,
            _ => cur.write_u64_be(self.base_media_decode_time)?,
        }

        if !cur.is_empty() {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Buffer larger than expected",
                    got: cur.remaining() as u64,
                },
                BoxType::TFDT,
            ));
        }

        Ok(())
    }

    /// Writes this `TfdtBox` into the given payload.
    pub fn write(&self, payload: &mut [u8]) -> Result<()> {
        let mut cur = WriteCursor::new(payload);
        self.write_in(&mut cur)
    }
}

impl TryFrom<&[u8]> for TfdtBox {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self> {
        TfdtBox::parse(value)
    }
}

impl TryFrom<BoxFrame<'_>> for TfdtBox {
    type Error = Error;

    fn try_from(value: BoxFrame<'_>) -> Result<Self> {
        if value.boxtype() != BoxType::TFDT {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::TFDT,
                found: value.boxtype(),
            }));
        }

        TfdtBox::parse(value.payload())
    }
}

/// Specification for the Track Fragment Decode Time Box (`tfdt`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TfdtSpec;

/// Flags for the Track Fragment Decode Time Box (`tfdt`).
pub type TfdtFlags = FullBoxFlags<TfdtSpec>;

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

        let mut buf = vec![0u8; original.size()];
        original.write(&mut buf).unwrap();

        let parsed = TfdtBox::parse(&buf).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn round_trip_v1() {
        let original = TfdtBox {
            version: 1,
            flags: TfdtFlags::empty(),
            base_media_decode_time: 0x123456789ABCDEF0,
        };

        let mut buf = vec![0u8; original.size()];
        original.write(&mut buf).unwrap();

        let parsed = TfdtBox::parse(&buf).unwrap();
        assert_eq!(parsed, original);
    }
}
