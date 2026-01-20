use core::mem;

use crate::cursor::ReadCursor;

use crate::BoxType;
use crate::BoxFrame;
use crate::FullBoxFlags;
use crate::FullBoxHeader;
use crate::error::*;

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
        let full_box_header = FullBoxHeader::<HmhdSpec>::parse_in(cur)?;

        let max_pdu_size = cur
            .read_u16_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        let avg_pdu_size = cur
            .read_u16_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        let max_bitrate = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        let avg_bitrate = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        // Skip reserved (4 bytes)
        cur.advance(Self::RESERVED_SIZE)
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

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
            version: full_box_header.version(),
            flags: full_box_header.flags(),
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
}

impl TryFrom<&[u8]> for HmhdBox {
    type Error = Error;

    fn try_from(payload: &[u8]) -> Result<Self> {
        HmhdBox::parse(payload)
    }
}

impl TryFrom<BoxFrame<'_>> for HmhdBox {
    type Error = Error;

    fn try_from(value: BoxFrame<'_>) -> Result<Self> {
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

    fn make_hmhd_payload(
        version: u8,
        flags: u32,
        max_pdu_size: u16,
        avg_pdu_size: u16,
        max_bitrate: u32,
        avg_bitrate: u32,
    ) -> Vec<u8> {
        let mut data = Vec::new();

        // FullBoxHeader: version (1 byte) + flags (3 bytes)
        data.push(version);
        data.extend_from_slice(&flags.to_be_bytes()[1..4]);

        // maxPDUsize (2 bytes)
        data.extend_from_slice(&max_pdu_size.to_be_bytes());

        // avgPDUsize (2 bytes)
        data.extend_from_slice(&avg_pdu_size.to_be_bytes());

        // maxbitrate (4 bytes)
        data.extend_from_slice(&max_bitrate.to_be_bytes());

        // avgbitrate (4 bytes)
        data.extend_from_slice(&avg_bitrate.to_be_bytes());

        // reserved (4 bytes)
        data.extend_from_slice(&[0u8; 4]);

        data
    }

    #[test]
    fn parse_hmhd_default() {
        let payload = make_hmhd_payload(0, 0, 0, 0, 0, 0);
        let hmhd = HmhdBox::parse(&payload).unwrap();

        assert_eq!(hmhd.version, 0);
        assert_eq!(hmhd.flags.get(), 0);
        assert_eq!(hmhd.max_pdu_size, 0);
        assert_eq!(hmhd.avg_pdu_size, 0);
        assert_eq!(hmhd.max_bitrate, 0);
        assert_eq!(hmhd.avg_bitrate, 0);
    }

    #[test]
    fn parse_hmhd_with_values() {
        let payload = make_hmhd_payload(0, 0, 1500, 1200, 1_000_000, 800_000);
        let hmhd = HmhdBox::parse(&payload).unwrap();

        assert_eq!(hmhd.max_pdu_size, 1500);
        assert_eq!(hmhd.avg_pdu_size, 1200);
        assert_eq!(hmhd.max_bitrate, 1_000_000);
        assert_eq!(hmhd.avg_bitrate, 800_000);
    }

    #[test]
    fn parse_hmhd_extra_data() {
        let mut payload = make_hmhd_payload(0, 0, 0, 0, 0, 0);
        payload.extend_from_slice(&[0xFF, 0xFF]); // Extra data

        let result = HmhdBox::parse(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn parse_hmhd_truncated() {
        let payload = make_hmhd_payload(0, 0, 0, 0, 0, 0);
        let truncated = &payload[..payload.len() - 1];

        let result = HmhdBox::parse(truncated);
        assert!(result.is_err());
    }

    #[test]
    fn hmhd_default() {
        let hmhd = HmhdBox::default();

        assert_eq!(hmhd.version, 0);
        assert_eq!(hmhd.flags.get(), 0);
        assert_eq!(hmhd.max_pdu_size, 0);
        assert_eq!(hmhd.avg_pdu_size, 0);
        assert_eq!(hmhd.max_bitrate, 0);
        assert_eq!(hmhd.avg_bitrate, 0);
    }
}
