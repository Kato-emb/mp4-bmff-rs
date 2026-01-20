use crate::cursor::ReadCursor;

use crate::BoxType;
use crate::BoxFrame;
use crate::error::*;
use crate::header::FullBoxFlags;
use crate::header::FullBoxHeader;

/// Track Fragment Header Box (`tfhd`).
///
/// This box contains the track fragment header for a track fragment.
/// Each track fragment has exactly one track fragment header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TfhdBox {
    /// The version of the box (should be 0).
    pub version: u8,
    /// The flags of the box.
    pub flags: TfhdFlags,
    /// The track ID for this fragment.
    pub track_id: u32,
    /// The base offset to use when calculating data offsets.
    /// Present if `base_data_offset_present` flag is set.
    pub base_data_offset: Option<u64>,
    /// The sample description index.
    /// Present if `sample_description_index_present` flag is set.
    pub sample_description_index: Option<u32>,
    /// The default sample duration.
    /// Present if `default_sample_duration_present` flag is set.
    pub default_sample_duration: Option<u32>,
    /// The default sample size.
    /// Present if `default_sample_size_present` flag is set.
    pub default_sample_size: Option<u32>,
    /// The default sample flags.
    /// Present if `default_sample_flags_present` flag is set.
    pub default_sample_flags: Option<u32>,
}

impl TfhdBox {
    pub(crate) fn parse_in(cur: &mut ReadCursor<'_>) -> Result<TfhdBox> {
        let full_box_header = FullBoxHeader::<TfhdSpec>::parse_in(cur)?;
        let flags = full_box_header.flags();

        let track_id = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        let base_data_offset = if flags.base_data_offset_present() {
            Some(
                cur.read_u64_be()
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
            )
        } else {
            None
        };

        let sample_description_index = if flags.sample_description_index_present() {
            Some(
                cur.read_u32_be()
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
            )
        } else {
            None
        };

        let default_sample_duration = if flags.default_sample_duration_present() {
            Some(
                cur.read_u32_be()
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
            )
        } else {
            None
        };

        let default_sample_size = if flags.default_sample_size_present() {
            Some(
                cur.read_u32_be()
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
            )
        } else {
            None
        };

        let default_sample_flags = if flags.default_sample_flags_present() {
            Some(
                cur.read_u32_be()
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
            )
        } else {
            None
        };

        if !cur.is_empty() {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Extra data after tfhd fields",
                    got: cur.remaining() as u64,
                },
                BoxType::TFHD,
            ));
        }

        Ok(TfhdBox {
            version: full_box_header.version(),
            flags,
            track_id,
            base_data_offset,
            sample_description_index,
            default_sample_duration,
            default_sample_size,
            default_sample_flags,
        })
    }

    /// Parses a `TfhdBox` from the given payload.
    pub fn parse(payload: &[u8]) -> Result<TfhdBox> {
        let mut cur = ReadCursor::new(payload);
        TfhdBox::parse_in(&mut cur)
    }
}

impl TryFrom<&[u8]> for TfhdBox {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self> {
        TfhdBox::parse(value)
    }
}

impl TryFrom<BoxFrame<'_>> for TfhdBox {
    type Error = Error;

    fn try_from(value: BoxFrame<'_>) -> Result<Self> {
        if value.boxtype() != BoxType::TFHD {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::TFHD,
                found: value.boxtype(),
            }));
        }

        TfhdBox::parse(value.payload())
    }
}

/// Specification for the Track Fragment Header Box (`tfhd`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TfhdSpec;

/// Flags for the Track Fragment Header Box (`tfhd`).
pub type TfhdFlags = FullBoxFlags<TfhdSpec>;

impl TfhdFlags {
    /// If set, base_data_offset is present.
    pub const BASE_DATA_OFFSET_PRESENT: TfhdFlags = TfhdFlags::from_bits_truncate(0x000001);
    /// If set, sample_description_index is present.
    pub const SAMPLE_DESCRIPTION_INDEX_PRESENT: TfhdFlags = TfhdFlags::from_bits_truncate(0x000002);
    /// If set, default_sample_duration is present.
    pub const DEFAULT_SAMPLE_DURATION_PRESENT: TfhdFlags = TfhdFlags::from_bits_truncate(0x000008);
    /// If set, default_sample_size is present.
    pub const DEFAULT_SAMPLE_SIZE_PRESENT: TfhdFlags = TfhdFlags::from_bits_truncate(0x000010);
    /// If set, default_sample_flags is present.
    pub const DEFAULT_SAMPLE_FLAGS_PRESENT: TfhdFlags = TfhdFlags::from_bits_truncate(0x000020);
    /// If set, there are no samples for this track in this fragment.
    pub const DURATION_IS_EMPTY: TfhdFlags = TfhdFlags::from_bits_truncate(0x010000);
    /// If set, the base_data_offset is relative to the first byte of the enclosing moof.
    pub const DEFAULT_BASE_IS_MOOF: TfhdFlags = TfhdFlags::from_bits_truncate(0x020000);

    /// Returns true if base_data_offset is present.
    pub fn base_data_offset_present(&self) -> bool {
        self.contains(Self::BASE_DATA_OFFSET_PRESENT)
    }

    /// Returns true if sample_description_index is present.
    pub fn sample_description_index_present(&self) -> bool {
        self.contains(Self::SAMPLE_DESCRIPTION_INDEX_PRESENT)
    }

    /// Returns true if default_sample_duration is present.
    pub fn default_sample_duration_present(&self) -> bool {
        self.contains(Self::DEFAULT_SAMPLE_DURATION_PRESENT)
    }

    /// Returns true if default_sample_size is present.
    pub fn default_sample_size_present(&self) -> bool {
        self.contains(Self::DEFAULT_SAMPLE_SIZE_PRESENT)
    }

    /// Returns true if default_sample_flags is present.
    pub fn default_sample_flags_present(&self) -> bool {
        self.contains(Self::DEFAULT_SAMPLE_FLAGS_PRESENT)
    }

    /// Returns true if duration_is_empty is set.
    pub fn duration_is_empty(&self) -> bool {
        self.contains(Self::DURATION_IS_EMPTY)
    }

    /// Returns true if default_base_is_moof is set.
    pub fn default_base_is_moof(&self) -> bool {
        self.contains(Self::DEFAULT_BASE_IS_MOOF)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tfhd_payload(flags: u32, track_id: u32, optional_fields: &[u8]) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(0); // version
        data.extend_from_slice(&flags.to_be_bytes()[1..4]);
        data.extend_from_slice(&track_id.to_be_bytes());
        data.extend_from_slice(optional_fields);
        data
    }

    #[test]
    fn parse_tfhd_minimal() {
        let payload = make_tfhd_payload(0, 1, &[]);
        let tfhd = TfhdBox::parse(&payload).unwrap();

        assert_eq!(tfhd.version, 0);
        assert_eq!(tfhd.track_id, 1);
        assert!(tfhd.base_data_offset.is_none());
        assert!(tfhd.sample_description_index.is_none());
        assert!(tfhd.default_sample_duration.is_none());
        assert!(tfhd.default_sample_size.is_none());
        assert!(tfhd.default_sample_flags.is_none());
    }

    #[test]
    fn parse_tfhd_with_base_data_offset() {
        let mut optional = Vec::new();
        optional.extend_from_slice(&0x123456789ABCDEFu64.to_be_bytes());

        let payload = make_tfhd_payload(0x000001, 2, &optional);
        let tfhd = TfhdBox::parse(&payload).unwrap();

        assert_eq!(tfhd.track_id, 2);
        assert_eq!(tfhd.base_data_offset, Some(0x123456789ABCDEF));
    }

    #[test]
    fn parse_tfhd_with_all_optional() {
        let flags = 0x000001 | 0x000002 | 0x000008 | 0x000010 | 0x000020;

        let mut optional = Vec::new();
        optional.extend_from_slice(&100u64.to_be_bytes()); // base_data_offset
        optional.extend_from_slice(&1u32.to_be_bytes()); // sample_description_index
        optional.extend_from_slice(&1000u32.to_be_bytes()); // default_sample_duration
        optional.extend_from_slice(&512u32.to_be_bytes()); // default_sample_size
        optional.extend_from_slice(&0x02000000u32.to_be_bytes()); // default_sample_flags

        let payload = make_tfhd_payload(flags, 3, &optional);
        let tfhd = TfhdBox::parse(&payload).unwrap();

        assert_eq!(tfhd.track_id, 3);
        assert_eq!(tfhd.base_data_offset, Some(100));
        assert_eq!(tfhd.sample_description_index, Some(1));
        assert_eq!(tfhd.default_sample_duration, Some(1000));
        assert_eq!(tfhd.default_sample_size, Some(512));
        assert_eq!(tfhd.default_sample_flags, Some(0x02000000));
    }

    #[test]
    fn parse_tfhd_default_base_is_moof() {
        let flags = 0x020000;
        let payload = make_tfhd_payload(flags, 1, &[]);
        let tfhd = TfhdBox::parse(&payload).unwrap();

        assert!(tfhd.flags.default_base_is_moof());
    }

    #[test]
    fn parse_tfhd_extra_data() {
        let mut payload = make_tfhd_payload(0, 1, &[]);
        payload.extend_from_slice(&[0xFF, 0xFF]);

        let result = TfhdBox::parse(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn try_from_box_view_success() {
        let payload = make_tfhd_payload(0, 42, &[]);

        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"tfhd");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxFrame::parse_in(&mut cursor).unwrap();
        let tfhd = TfhdBox::try_from(box_view).unwrap();

        assert_eq!(tfhd.track_id, 42);
    }

    #[test]
    fn try_from_box_view_wrong_type() {
        let payload = make_tfhd_payload(0, 1, &[]);

        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"traf");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxFrame::parse_in(&mut cursor).unwrap();
        let result = TfhdBox::try_from(box_view);

        assert!(result.is_err());
    }
}
