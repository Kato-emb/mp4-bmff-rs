use crate::cursor::ReadCursor;

use crate::BoxType;
use crate::BoxView;
use crate::error::*;
use crate::header::FullBoxFlags;
use crate::header::FullBoxHeader;

/// Track Extends Defaults Box (`trex`).
///
/// This box sets up default values used by the movie fragments.
/// Each track in the Movie Box that will be extended by a movie fragment
/// shall have exactly one `trex` box in the `mvex` box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrexBox {
    /// The version of the box.
    pub version: u8,
    /// The flags of the box.
    pub flags: TrexFlags,
    /// The track ID of the track for which these defaults apply.
    pub track_id: u32,
    /// The default sample description index.
    pub default_sample_description_index: u32,
    /// The default sample duration.
    pub default_sample_duration: u32,
    /// The default sample size.
    pub default_sample_size: u32,
    /// The default sample flags.
    pub default_sample_flags: u32,
}

impl TrexBox {
    pub(crate) fn parse_in(cur: &mut ReadCursor<'_>) -> Result<TrexBox> {
        let full_box_header = FullBoxHeader::<TrexSpec>::parse_in(cur)?;

        let track_id = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        let default_sample_description_index = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        let default_sample_duration = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        let default_sample_size = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        let default_sample_flags = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        Ok(TrexBox {
            version: full_box_header.version(),
            flags: full_box_header.flags(),
            track_id,
            default_sample_description_index,
            default_sample_duration,
            default_sample_size,
            default_sample_flags,
        })
    }

    /// Parses a `TrexBox` from the given payload.
    pub fn parse(payload: &[u8]) -> Result<TrexBox> {
        let mut cur = ReadCursor::new(payload);
        let this = TrexBox::parse_in(&mut cur)?;

        if cur.remaining() > 0 {
            return Err(Error::at_in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Extra data after trex box",
                    got: cur.remaining() as u64,
                },
                cur.position() as u64,
                BoxType::TREX,
            ));
        }

        Ok(this)
    }
}

impl TryFrom<&[u8]> for TrexBox {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self> {
        TrexBox::parse(value)
    }
}

impl TryFrom<&BoxView<'_>> for TrexBox {
    type Error = Error;

    fn try_from(value: &BoxView<'_>) -> Result<Self> {
        if value.header.boxtype() != BoxType::TREX {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::TREX,
                found: value.header.boxtype(),
            }));
        }

        TrexBox::parse(value.payload)
    }
}

/// Specification for the Track Extends Defaults Box (`trex`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrexSpec;

/// Flags for the Track Extends Defaults Box (`trex`).
pub type TrexFlags = FullBoxFlags<TrexSpec>;

#[cfg(test)]
mod tests {
    use super::*;

    fn make_full_box_header(version: u8, flags: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(version);
        data.extend_from_slice(&flags.to_be_bytes()[1..4]);
        data
    }

    fn make_trex_payload(
        track_id: u32,
        default_sample_description_index: u32,
        default_sample_duration: u32,
        default_sample_size: u32,
        default_sample_flags: u32,
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(0, 0));
        payload.extend_from_slice(&track_id.to_be_bytes());
        payload.extend_from_slice(&default_sample_description_index.to_be_bytes());
        payload.extend_from_slice(&default_sample_duration.to_be_bytes());
        payload.extend_from_slice(&default_sample_size.to_be_bytes());
        payload.extend_from_slice(&default_sample_flags.to_be_bytes());
        payload
    }

    #[test]
    fn parse_trex_basic() {
        let payload = make_trex_payload(1, 1, 1024, 512, 0x00010000);
        let trex = TrexBox::parse(&payload).unwrap();

        assert_eq!(trex.version, 0);
        assert_eq!(trex.track_id, 1);
        assert_eq!(trex.default_sample_description_index, 1);
        assert_eq!(trex.default_sample_duration, 1024);
        assert_eq!(trex.default_sample_size, 512);
        assert_eq!(trex.default_sample_flags, 0x00010000);
    }

    #[test]
    fn parse_trex_zeros() {
        let payload = make_trex_payload(2, 0, 0, 0, 0);
        let trex = TrexBox::parse(&payload).unwrap();

        assert_eq!(trex.track_id, 2);
        assert_eq!(trex.default_sample_description_index, 0);
        assert_eq!(trex.default_sample_duration, 0);
        assert_eq!(trex.default_sample_size, 0);
        assert_eq!(trex.default_sample_flags, 0);
    }

    #[test]
    fn parse_trex_extra_data() {
        let mut payload = make_trex_payload(1, 1, 1024, 512, 0);
        payload.extend_from_slice(&[0, 0, 0, 0]); // extra data

        let result = TrexBox::parse(&payload);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::InvalidBoxSize { .. }));
        }
    }

    #[test]
    fn parse_trex_truncated() {
        let mut payload = make_full_box_header(0, 0);
        payload.extend_from_slice(&1u32.to_be_bytes()); // only track_id

        let result = TrexBox::parse(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn try_from_box_view_success() {
        let payload = make_trex_payload(1, 1, 1000, 100, 0);

        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"trex");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxView::parse_in(&mut cursor).unwrap();
        let trex = TrexBox::try_from(&box_view).unwrap();

        assert_eq!(trex.track_id, 1);
    }

    #[test]
    fn try_from_box_view_wrong_type() {
        let payload = make_trex_payload(1, 1, 1000, 100, 0);

        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"trak"); // Wrong type
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxView::parse_in(&mut cursor).unwrap();
        let result = TrexBox::try_from(&box_view);

        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::MismatchedBoxType { .. }));
        }
    }

    #[test]
    fn try_from_byte_slice() {
        let payload = make_trex_payload(3, 2, 2048, 1024, 0x00020000);
        let trex = TrexBox::try_from(payload.as_slice()).unwrap();

        assert_eq!(trex.track_id, 3);
        assert_eq!(trex.default_sample_description_index, 2);
        assert_eq!(trex.default_sample_duration, 2048);
        assert_eq!(trex.default_sample_size, 1024);
        assert_eq!(trex.default_sample_flags, 0x00020000);
    }
}
