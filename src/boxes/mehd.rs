use crate::cursor::ReadCursor;

use crate::BoxType;
use crate::BoxView;
use crate::error::*;
use crate::header::FullBoxFlags;
use crate::header::FullBoxHeader;

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
        let full_box_header = FullBoxHeader::<MehdSpec>::parse_in(cur)?;
        let version = full_box_header.version();

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
            flags: full_box_header.flags(),
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
}

impl TryFrom<&[u8]> for MehdBox {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self> {
        MehdBox::parse(value)
    }
}

impl TryFrom<&BoxView<'_>> for MehdBox {
    type Error = Error;

    fn try_from(value: &BoxView<'_>) -> Result<Self> {
        if value.header.boxtype() != BoxType::MEHD {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::MEHD,
                found: value.header.boxtype(),
            }));
        }

        MehdBox::parse(value.payload)
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

    fn make_full_box_header(version: u8, flags: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(version);
        data.extend_from_slice(&flags.to_be_bytes()[1..4]);
        data
    }

    fn make_mehd_payload_v0(fragment_duration: u32) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(0, 0));
        payload.extend_from_slice(&fragment_duration.to_be_bytes());
        payload
    }

    fn make_mehd_payload_v1(fragment_duration: u64) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(1, 0));
        payload.extend_from_slice(&fragment_duration.to_be_bytes());
        payload
    }

    #[test]
    fn parse_mehd_v0() {
        let payload = make_mehd_payload_v0(12345);
        let mehd = MehdBox::parse(&payload).unwrap();

        assert_eq!(mehd.version, 0);
        assert_eq!(mehd.fragment_duration, 12345);
    }

    #[test]
    fn parse_mehd_v1() {
        let payload = make_mehd_payload_v1(0x1_0000_0000);
        let mehd = MehdBox::parse(&payload).unwrap();

        assert_eq!(mehd.version, 1);
        assert_eq!(mehd.fragment_duration, 0x1_0000_0000);
    }

    #[test]
    fn parse_mehd_v0_extra_data() {
        let mut payload = make_mehd_payload_v0(1000);
        payload.extend_from_slice(&[0, 0, 0, 0]); // extra data

        let result = MehdBox::parse(&payload);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::InvalidBoxSize { .. }));
        }
    }

    #[test]
    fn parse_mehd_truncated() {
        let payload = make_full_box_header(0, 0);
        // Missing fragment_duration

        let result = MehdBox::parse(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn try_from_box_view_success() {
        let payload = make_mehd_payload_v0(5000);

        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"mehd");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxView::parse_in(&mut cursor).unwrap();
        let mehd = MehdBox::try_from(&box_view).unwrap();

        assert_eq!(mehd.fragment_duration, 5000);
    }

    #[test]
    fn try_from_box_view_wrong_type() {
        let payload = make_mehd_payload_v0(1000);

        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"mvhd"); // Wrong type
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxView::parse_in(&mut cursor).unwrap();
        let result = MehdBox::try_from(&box_view);

        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::MismatchedBoxType { .. }));
        }
    }

    #[test]
    fn try_from_byte_slice() {
        let payload = make_mehd_payload_v1(999999);
        let mehd = MehdBox::try_from(payload.as_slice()).unwrap();

        assert_eq!(mehd.version, 1);
        assert_eq!(mehd.fragment_duration, 999999);
    }
}
