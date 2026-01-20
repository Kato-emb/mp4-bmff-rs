use crate::cursor::ReadCursor;

use crate::BoxType;
use crate::BoxFrame;
use crate::FullBoxFlags;
use crate::error::*;

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

        Ok(NmhdBox {
            version,
            flags,
        })
    }

    /// Parses a `NmhdBox` from the given payload.
    pub fn parse(payload: &[u8]) -> Result<NmhdBox> {
        let mut cursor = ReadCursor::new(payload);
        NmhdBox::parse_in(&mut cursor)
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

    fn make_nmhd_payload(version: u8, flags: u32) -> Vec<u8> {
        let mut data = Vec::new();

        // FullBoxHeader: version (1 byte) + flags (3 bytes)
        data.push(version);
        data.extend_from_slice(&flags.to_be_bytes()[1..4]);

        data
    }

    #[test]
    fn parse_nmhd_default() {
        let payload = make_nmhd_payload(0, 0);
        let nmhd = NmhdBox::parse(&payload).unwrap();

        assert_eq!(nmhd.version, 0);
        assert_eq!(nmhd.flags.get(), 0);
    }

    #[test]
    fn parse_nmhd_with_flags() {
        let payload = make_nmhd_payload(0, 0x000001);
        let nmhd = NmhdBox::parse(&payload).unwrap();

        assert_eq!(nmhd.version, 0);
        assert_eq!(nmhd.flags.get(), 1);
    }

    #[test]
    fn parse_nmhd_extra_data() {
        let mut payload = make_nmhd_payload(0, 0);
        payload.extend_from_slice(&[0xFF, 0xFF]); // Extra data

        let result = NmhdBox::parse(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn parse_nmhd_truncated() {
        let payload = make_nmhd_payload(0, 0);
        let truncated = &payload[..payload.len() - 1];

        let result = NmhdBox::parse(truncated);
        assert!(result.is_err());
    }

    #[test]
    fn nmhd_default() {
        let nmhd = NmhdBox::default();

        assert_eq!(nmhd.version, 0);
        assert_eq!(nmhd.flags.get(), 0);
    }
}
