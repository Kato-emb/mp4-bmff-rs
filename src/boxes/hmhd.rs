use core::mem;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

use crate::RawBoxRef;
use crate::BoxType;
use crate::error::*;

use super::FullBoxFlags;

/// A Hint Media Header Box (`hmhd`).
///
/// This box contains general information, independent of the protocol,
/// for hint tracks. A hint track should contain either an `hmhd` box or
/// an `nmhd` box within its Media Information Box.
#[derive(Debug, Clone, Copy)]
pub struct HmhdBox {
    /// Box version (should be 0).
    pub version: u8,
    /// Box flags (should be 0).
    pub flags: HmhdFlags,
    /// Maximum PDU size in bytes.
    pub max_pdu_size: u16,
    /// Average PDU size in bytes.
    pub avg_pdu_size: u16,
    /// Maximum bitrate in bits per second.
    pub max_bitrate: u32,
    /// Average bitrate in bits per second.
    pub avg_bitrate: u32,
}

impl Default for HmhdBox {
    fn default() -> Self {
        HmhdBox {
            version: 0,
            flags: HmhdFlags::empty(),
            max_pdu_size: 0,
            avg_pdu_size: 0,
            max_bitrate: 0,
            avg_bitrate: 0,
        }
    }
}

impl HmhdBox {
    const RESERVED_SIZE: usize = mem::size_of::<u32>();

    pub(crate) fn parse_in(cur: &mut ReadCursor<'_>) -> Result<HmhdBox> {
        let version = cur.read_u8()?;
        let flags = HmhdFlags::from_bytes(cur.read_array()?);

        let max_pdu_size = cur.read_u16_be()?;
        let avg_pdu_size = cur.read_u16_be()?;
        let max_bitrate = cur.read_u32_be()?;
        let avg_bitrate = cur.read_u32_be()?;

        // Skip reserved (4 bytes)
        cur.advance(Self::RESERVED_SIZE)?;

        if !cur.is_empty() {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Extra data after parsing hmhd",
                    got: cur.remaining() as u64,
                },
                BoxType::HMHD,
            ));
        }

        Ok(HmhdBox {
            version,
            flags,
            max_pdu_size,
            avg_pdu_size,
            max_bitrate,
            avg_bitrate,
        })
    }

    /// Parses a `HmhdBox` from the given payload.
    pub fn parse(payload: &[u8]) -> Result<HmhdBox> {
        let mut cursor = ReadCursor::new(payload);
        HmhdBox::parse_in(&mut cursor)
    }

    /// Returns the size of the payload in bytes.
    pub fn size(&self) -> usize {
        // version(1) + flags(3) + max_pdu_size(2) + avg_pdu_size(2) +
        // max_bitrate(4) + avg_bitrate(4) + reserved(4) = 20
        20
    }

    pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
        // Write version (1 byte)
        cur.write_u8(self.version)?;

        // Write flags (3 bytes)
        cur.write_array(&self.flags.to_bytes())?;

        // Write max_pdu_size (2 bytes)
        cur.write_u16_be(self.max_pdu_size)?;

        // Write avg_pdu_size (2 bytes)
        cur.write_u16_be(self.avg_pdu_size)?;

        // Write max_bitrate (4 bytes)
        cur.write_u32_be(self.max_bitrate)?;

        // Write avg_bitrate (4 bytes)
        cur.write_u32_be(self.avg_bitrate)?;

        // Write reserved (4 bytes)
        cur.reserve_zeros(Self::RESERVED_SIZE)?;

        if !cur.is_empty() {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Buffer larger than expected",
                    got: cur.remaining() as u64,
                },
                BoxType::HMHD,
            ));
        }

        Ok(())
    }

    /// Writes this `HmhdBox` into the given payload.
    pub fn write(&self, payload: &mut [u8]) -> Result<()> {
        let mut cursor = WriteCursor::new(payload);
        self.write_in(&mut cursor)
    }
}

impl TryFrom<&[u8]> for HmhdBox {
    type Error = Error;

    fn try_from(payload: &[u8]) -> Result<Self> {
        HmhdBox::parse(payload)
    }
}

impl TryFrom<RawBoxRef<'_>> for HmhdBox {
    type Error = Error;

    fn try_from(value: RawBoxRef<'_>) -> Result<Self> {
        if value.boxtype() != BoxType::HMHD {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::HMHD,
                found: value.boxtype(),
            }));
        }

        HmhdBox::parse(value.payload())
    }
}

/// Specification for Hint Media Header Box (`hmhd`).
pub struct HmhdSpec;

/// Flags for Hint Media Header Box (`hmhd`).
pub type HmhdFlags = FullBoxFlags<HmhdSpec>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let original = HmhdBox {
            version: 0,
            flags: HmhdFlags::empty(),
            max_pdu_size: 1500,
            avg_pdu_size: 1200,
            max_bitrate: 1_000_000,
            avg_bitrate: 800_000,
        };

        let mut buf = vec![0u8; original.size()];
        original.write(&mut buf).unwrap();

        let reparsed = HmhdBox::parse(&buf).unwrap();
        assert_eq!(reparsed.version, original.version);
        assert_eq!(reparsed.flags.get(), original.flags.get());
        assert_eq!(reparsed.max_pdu_size, original.max_pdu_size);
        assert_eq!(reparsed.avg_pdu_size, original.avg_pdu_size);
        assert_eq!(reparsed.max_bitrate, original.max_bitrate);
        assert_eq!(reparsed.avg_bitrate, original.avg_bitrate);
    }
}
