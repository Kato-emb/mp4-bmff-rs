use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

use crate::BoxFrame;
use crate::BoxType;
use crate::error::*;

use super::FullBoxFlags;

/// A Null Media Header Box (`nmhd`).
///
/// This box is used for tracks that do not otherwise have a specific media header
/// (neither `vmhd`, `smhd`, nor `hmhd`). This includes some metadata tracks.
#[derive(Debug, Clone, Copy)]
pub struct NmhdBox {
    /// Box version (should be 0).
    pub version: u8,
    /// Box flags (should be 0).
    pub flags: NmhdFlags,
}

impl Default for NmhdBox {
    fn default() -> Self {
        NmhdBox {
            version: 0,
            flags: NmhdFlags::empty(),
        }
    }
}

impl NmhdBox {
    pub(crate) fn parse_in(cur: &mut ReadCursor<'_>) -> Result<NmhdBox> {
        let version = cur
            .read_u8()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let flags = NmhdFlags::from_bytes(
            cur.read_array()
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
        );

        if !cur.is_empty() {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Extra data after parsing nmhd",
                    got: cur.remaining() as u64,
                },
                BoxType::NMHD,
            ));
        }

        Ok(NmhdBox { version, flags })
    }

    /// Parses a `NmhdBox` from the given payload.
    pub fn parse(payload: &[u8]) -> Result<NmhdBox> {
        let mut cursor = ReadCursor::new(payload);
        NmhdBox::parse_in(&mut cursor)
    }

    /// Returns the size of the payload in bytes.
    pub fn size(&self) -> usize {
        // version(1) + flags(3) = 4
        4
    }

    pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
        // Write version (1 byte)
        cur.write_u8(self.version)
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        // Write flags (3 bytes)
        cur.write_array(&self.flags.to_bytes())
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        if !cur.is_empty() {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Buffer larger than expected",
                    got: cur.remaining() as u64,
                },
                BoxType::NMHD,
            ));
        }

        Ok(())
    }

    /// Writes this `NmhdBox` into the given payload.
    pub fn write(&self, payload: &mut [u8]) -> Result<()> {
        let mut cursor = WriteCursor::new(payload);
        self.write_in(&mut cursor)
    }
}

impl TryFrom<&[u8]> for NmhdBox {
    type Error = Error;

    fn try_from(payload: &[u8]) -> Result<Self> {
        NmhdBox::parse(payload)
    }
}

impl TryFrom<BoxFrame<'_>> for NmhdBox {
    type Error = Error;

    fn try_from(value: BoxFrame<'_>) -> Result<Self> {
        if value.boxtype() != BoxType::NMHD {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::NMHD,
                found: value.boxtype(),
            }));
        }

        NmhdBox::parse(value.payload())
    }
}

/// Specification for Null Media Header Box (`nmhd`).
pub struct NmhdSpec;

/// Flags for Null Media Header Box (`nmhd`).
pub type NmhdFlags = FullBoxFlags<NmhdSpec>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let original = NmhdBox {
            version: 0,
            flags: NmhdFlags::new(1),
        };

        let mut buf = vec![0u8; original.size()];
        original.write(&mut buf).unwrap();

        let reparsed = NmhdBox::parse(&buf).unwrap();
        assert_eq!(reparsed.version, original.version);
        assert_eq!(reparsed.flags.get(), original.flags.get());
    }
}
