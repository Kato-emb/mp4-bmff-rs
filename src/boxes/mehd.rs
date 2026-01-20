use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

use crate::BoxFrame;
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

impl MehdBox {
    pub(crate) fn parse_in(cur: &mut ReadCursor<'_>) -> Result<MehdBox> {
        let version = cur
            .read_u8()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let flags = MehdFlags::from_bytes(
            cur.read_array()
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
        );

        let fragment_duration = match version {
            0 => cur
                .read_u32_be()
                .map_err(|e| Error::at(e.into(), cur.position() as u64))? as u64,
            1 => cur
                .read_u64_be()
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
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

    /// Parses a `MehdBox` from the given payload.
    pub fn parse(payload: &[u8]) -> Result<MehdBox> {
        let mut cur = ReadCursor::new(payload);
        let this = MehdBox::parse_in(&mut cur)?;

        if cur.remaining() > 0 {
            return Err(Error::at_in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Extra data after mehd box",
                    got: cur.remaining() as u64,
                },
                cur.position() as u64,
                BoxType::MEHD,
            ));
        }

        Ok(this)
    }

    /// Returns the size of the payload in bytes.
    pub fn size(&self) -> usize {
        4 + match self.version {
            0 => 4, // fragment_duration as u32
            _ => 8, // fragment_duration as u64
        }
    }

    pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
        cur.write_u8(self.version)
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        cur.write_array(&self.flags.to_bytes())
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        match self.version {
            0 => cur
                .write_u32_be(self.fragment_duration as u32)
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
            _ => cur
                .write_u64_be(self.fragment_duration)
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
        }

        if !cur.is_empty() {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Buffer larger than expected",
                    got: cur.remaining() as u64,
                },
                BoxType::MEHD,
            ));
        }

        Ok(())
    }

    /// Writes this `MehdBox` into the given payload.
    pub fn write(&self, payload: &mut [u8]) -> Result<()> {
        let mut cur = WriteCursor::new(payload);
        self.write_in(&mut cur)
    }
}

impl TryFrom<&[u8]> for MehdBox {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self> {
        MehdBox::parse(value)
    }
}

impl TryFrom<BoxFrame<'_>> for MehdBox {
    type Error = Error;

    fn try_from(value: BoxFrame<'_>) -> Result<Self> {
        if value.boxtype() != BoxType::MEHD {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::MEHD,
                found: value.boxtype(),
            }));
        }

        MehdBox::parse(value.payload())
    }
}

/// Specification for the Movie Extends Header Box (`mehd`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MehdSpec;

/// Flags for the Movie Extends Header Box (`mehd`).
pub type MehdFlags = FullBoxFlags<MehdSpec>;

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

        let mut buf = vec![0u8; original.size()];
        original.write(&mut buf).unwrap();

        let parsed = MehdBox::parse(&buf).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn round_trip_v1() {
        let original = MehdBox {
            version: 1,
            flags: MehdFlags::empty(),
            fragment_duration: 0x1_0000_0000,
        };

        let mut buf = vec![0u8; original.size()];
        original.write(&mut buf).unwrap();

        let parsed = MehdBox::parse(&buf).unwrap();
        assert_eq!(parsed, original);
    }
}
