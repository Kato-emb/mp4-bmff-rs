use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

use crate::RawBoxRef;
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

impl MfhdBox {
    pub(crate) fn parse_in(cur: &mut ReadCursor<'_>) -> Result<MfhdBox> {
        let version = cur.read_u8()?;
        let flags = MfhdFlags::from_bytes(cur.read_array()?);

        let sequence_number = cur.read_u32_be()?;

        if !cur.is_empty() {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Extra data after mfhd fields",
                    got: cur.remaining() as u64,
                },
                BoxType::MFHD,
            ));
        }

        Ok(MfhdBox {
            version,
            flags,
            sequence_number,
        })
    }

    /// Parses a `MfhdBox` from the given payload.
    pub fn parse(payload: &[u8]) -> Result<MfhdBox> {
        let mut cur = ReadCursor::new(payload);
        MfhdBox::parse_in(&mut cur)
    }

    /// Returns the size of the payload in bytes.
    pub fn size(&self) -> usize {
        4 + 4 // version(1) + flags(3) + sequence_number(4)
    }

    pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
        cur.write_u8(self.version)?;
        cur.write_array(&self.flags.to_bytes())?;
        cur.write_u32_be(self.sequence_number)?;

        if !cur.is_empty() {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Buffer larger than expected",
                    got: cur.remaining() as u64,
                },
                BoxType::MFHD,
            ));
        }

        Ok(())
    }

    /// Writes this `MfhdBox` into the given payload.
    pub fn write(&self, payload: &mut [u8]) -> Result<()> {
        let mut cur = WriteCursor::new(payload);
        self.write_in(&mut cur)
    }
}

impl TryFrom<&[u8]> for MfhdBox {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self> {
        MfhdBox::parse(value)
    }
}

impl TryFrom<RawBoxRef<'_>> for MfhdBox {
    type Error = Error;

    fn try_from(value: RawBoxRef<'_>) -> Result<Self> {
        if value.boxtype() != BoxType::MFHD {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::MFHD,
                found: value.boxtype(),
            }));
        }

        MfhdBox::parse(value.payload())
    }
}

/// Specification for the Movie Fragment Header Box (`mfhd`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MfhdSpec;

/// Flags for the Movie Fragment Header Box (`mfhd`).
pub type MfhdFlags = FullBoxFlags<MfhdSpec>;

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

        let mut buf = vec![0u8; original.size()];
        original.write(&mut buf).unwrap();

        let parsed = MfhdBox::parse(&buf).unwrap();
        assert_eq!(parsed, original);
    }
}
