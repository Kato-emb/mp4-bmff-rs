use crate::cursor::ReadCursor;

use crate::BoxType;
use crate::BoxFrame;
use crate::error::*;
use crate::header::FullBoxFlags;
use crate::header::FullBoxHeader;

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
        let full_box_header = FullBoxHeader::<MfhdSpec>::parse_in(cur)?;

        let sequence_number = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

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
            version: full_box_header.version(),
            flags: full_box_header.flags(),
            sequence_number,
        })
    }

    /// Parses a `MfhdBox` from the given payload.
    pub fn parse(payload: &[u8]) -> Result<MfhdBox> {
        let mut cur = ReadCursor::new(payload);
        MfhdBox::parse_in(&mut cur)
    }
}

impl TryFrom<&[u8]> for MfhdBox {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self> {
        MfhdBox::parse(value)
    }
}

impl TryFrom<BoxFrame<'_>> for MfhdBox {
    type Error = Error;

    fn try_from(value: BoxFrame<'_>) -> Result<Self> {
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

    fn make_mfhd_payload(version: u8, flags: u32, sequence_number: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(version);
        data.extend_from_slice(&flags.to_be_bytes()[1..4]);
        data.extend_from_slice(&sequence_number.to_be_bytes());
        data
    }

    #[test]
    fn parse_mfhd_basic() {
        let payload = make_mfhd_payload(0, 0, 1);
        let mfhd = MfhdBox::parse(&payload).unwrap();

        assert_eq!(mfhd.version, 0);
        assert_eq!(mfhd.flags.get(), 0);
        assert_eq!(mfhd.sequence_number, 1);
    }

    #[test]
    fn parse_mfhd_large_sequence() {
        let payload = make_mfhd_payload(0, 0, 0xDEADBEEF);
        let mfhd = MfhdBox::parse(&payload).unwrap();

        assert_eq!(mfhd.sequence_number, 0xDEADBEEF);
    }

    #[test]
    fn parse_mfhd_extra_data() {
        let mut payload = make_mfhd_payload(0, 0, 1);
        payload.extend_from_slice(&[0xFF, 0xFF]);

        let result = MfhdBox::parse(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn parse_mfhd_truncated() {
        let payload = make_mfhd_payload(0, 0, 1);
        let truncated = &payload[..payload.len() - 1];

        let result = MfhdBox::parse(truncated);
        assert!(result.is_err());
    }

    #[test]
    fn try_from_byte_slice() {
        let payload = make_mfhd_payload(0, 0, 42);
        let mfhd = MfhdBox::try_from(payload.as_slice()).unwrap();

        assert_eq!(mfhd.sequence_number, 42);
    }

    #[test]
    fn try_from_box_view_success() {
        let payload = make_mfhd_payload(0, 0, 100);

        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"mfhd");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxFrame::parse_in(&mut cursor).unwrap();
        let mfhd = MfhdBox::try_from(box_view).unwrap();

        assert_eq!(mfhd.sequence_number, 100);
    }

    #[test]
    fn try_from_box_view_wrong_type() {
        let payload = make_mfhd_payload(0, 0, 1);

        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"moov");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxFrame::parse_in(&mut cursor).unwrap();
        let result = MfhdBox::try_from(box_view);

        assert!(result.is_err());
    }
}
