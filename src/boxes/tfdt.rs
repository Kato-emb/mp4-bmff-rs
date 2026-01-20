use crate::cursor::ReadCursor;

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
        let version = cur
            .read_u8()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let flags = TfdtFlags::from_bytes(
            cur.read_array()
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
        );

        let base_media_decode_time = match version {
            0 => cur
                .read_u32_be()
                .map_err(|e| Error::at(e.into(), cur.position() as u64))? as u64,
            1 => cur
                .read_u64_be()
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
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

    fn make_tfdt_payload_v0(base_media_decode_time: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(0); // version
        data.extend_from_slice(&[0, 0, 0]); // flags
        data.extend_from_slice(&base_media_decode_time.to_be_bytes());
        data
    }

    fn make_tfdt_payload_v1(base_media_decode_time: u64) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(1); // version
        data.extend_from_slice(&[0, 0, 0]); // flags
        data.extend_from_slice(&base_media_decode_time.to_be_bytes());
        data
    }

    #[test]
    fn parse_tfdt_v0() {
        let payload = make_tfdt_payload_v0(12345);
        let tfdt = TfdtBox::parse(&payload).unwrap();

        assert_eq!(tfdt.version, 0);
        assert_eq!(tfdt.base_media_decode_time, 12345);
    }

    #[test]
    fn parse_tfdt_v1() {
        let payload = make_tfdt_payload_v1(0x123456789ABCDEF0);
        let tfdt = TfdtBox::parse(&payload).unwrap();

        assert_eq!(tfdt.version, 1);
        assert_eq!(tfdt.base_media_decode_time, 0x123456789ABCDEF0);
    }

    #[test]
    fn parse_tfdt_invalid_version() {
        let mut data = Vec::new();
        data.push(2); // invalid version
        data.extend_from_slice(&[0, 0, 0]); // flags
        data.extend_from_slice(&0u32.to_be_bytes());

        let result = TfdtBox::parse(&data);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::InvalidBoxVersion { .. }));
        }
    }

    #[test]
    fn parse_tfdt_extra_data() {
        let mut payload = make_tfdt_payload_v0(0);
        payload.extend_from_slice(&[0xFF, 0xFF]);

        let result = TfdtBox::parse(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn parse_tfdt_truncated() {
        let payload = make_tfdt_payload_v0(0);
        let truncated = &payload[..payload.len() - 1];

        let result = TfdtBox::parse(truncated);
        assert!(result.is_err());
    }

    #[test]
    fn try_from_box_view_success() {
        let payload = make_tfdt_payload_v0(1000);

        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"tfdt");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxFrame::parse_in(&mut cursor).unwrap();
        let tfdt = TfdtBox::try_from(box_view).unwrap();

        assert_eq!(tfdt.base_media_decode_time, 1000);
    }

    #[test]
    fn try_from_box_view_wrong_type() {
        let payload = make_tfdt_payload_v0(0);

        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"tfhd");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxFrame::parse_in(&mut cursor).unwrap();
        let result = TfdtBox::try_from(box_view);

        assert!(result.is_err());
    }
}
