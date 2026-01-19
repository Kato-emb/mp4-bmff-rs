use crate::cursor::ReadCursor;
use crate::types::FourCC;

use crate::BoxType;
use crate::BoxView;
use crate::error::*;
use crate::header::FullBoxFlags;
use crate::header::FullBoxHeader;

/// An entry in the Sample to Group Box (`sbgp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SbgpEntry {
    /// The number of consecutive samples with the same group description.
    pub sample_count: u32,
    /// An index that identifies a sample group description entry.
    /// The value 0 means that the samples are not assigned to any group.
    pub group_description_index: u32,
}

/// A reference to a Sample to Group Box (`sbgp`).
#[derive(Debug)]
pub struct SbgpBoxView<'a> {
    /// The version of the box (0 or 1).
    pub version: u8,
    /// The flags of the box.
    pub flags: SbgpFlags,
    /// A FourCC identifying the type of grouping.
    pub grouping_type: FourCC,
    /// An additional parameter for the grouping (version 1 only).
    pub grouping_type_parameter: Option<u32>,
    /// The number of entries.
    pub entry_count: u32,
    entries: &'a [u8],
}

impl<'a> SbgpBoxView<'a> {
    const ENTRY_SIZE: usize = 8;

    /// Returns an iterator over the entries in the Sample to Group Box.
    pub fn entries(&self) -> impl Iterator<Item = Result<SbgpEntry>> + 'a {
        let entry_bytes = self.entries;
        let entry_count = self.entry_count as usize;

        entry_bytes
            .chunks_exact(Self::ENTRY_SIZE)
            .take(entry_count)
            .map(|chunk| {
                let mut cursor = ReadCursor::new(chunk);

                let sample_count = cursor
                    .read_u32_be()
                    .map_err(|e| Error::at(e.into(), cursor.position() as u64))?;

                let group_description_index = cursor
                    .read_u32_be()
                    .map_err(|e| Error::at(e.into(), cursor.position() as u64))?;

                Ok(SbgpEntry {
                    sample_count,
                    group_description_index,
                })
            })
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<SbgpBoxView<'a>> {
        let full_box_header = FullBoxHeader::<SbgpSpec>::parse_in(cur)?;
        let version = full_box_header.version();

        if version > 1 {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxVersion {
                    reason: "sbgp version must be 0 or 1",
                    got: version,
                },
                BoxType::SBGP,
            ));
        }

        let grouping_type_bytes = cur
            .read_array::<4>()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let grouping_type = FourCC::new(grouping_type_bytes);

        let grouping_type_parameter = if version == 1 {
            Some(
                cur.read_u32_be()
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
            )
        } else {
            None
        };

        let entry_count = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        let expected_size = entry_count as usize * Self::ENTRY_SIZE;
        if cur.remaining() != expected_size {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "sbgp entries size does not match entry_count",
                    got: cur.remaining() as u64,
                },
                BoxType::SBGP,
            ));
        }

        let entries = cur.take(cur.remaining())?;

        Ok(SbgpBoxView {
            version,
            flags: full_box_header.flags(),
            grouping_type,
            grouping_type_parameter,
            entry_count,
            entries,
        })
    }

    /// Parses a `SbgpBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<SbgpBoxView<'a>> {
        let mut cur = ReadCursor::new(payload);
        SbgpBoxView::parse_in(&mut cur)
    }
}

impl<'a> TryFrom<&'a [u8]> for SbgpBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        SbgpBoxView::parse(value)
    }
}

impl<'a> TryFrom<&BoxView<'a>> for SbgpBoxView<'a> {
    type Error = Error;

    fn try_from(value: &BoxView<'a>) -> Result<Self> {
        if value.header.boxtype() != BoxType::SBGP {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::SBGP,
                found: value.header.boxtype(),
            }));
        }

        SbgpBoxView::parse(value.payload)
    }
}

/// Specification for the Sample to Group Box (`sbgp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SbgpSpec;

/// Flags for the Sample to Group Box (`sbgp`).
pub type SbgpFlags = FullBoxFlags<SbgpSpec>;

#[cfg(feature = "alloc")]
pub use owned::SbgpBox;

#[cfg(feature = "alloc")]
mod owned {
    extern crate alloc;
    use alloc::vec::Vec;

    use super::*;

    /// An owned Sample to Group Box (`sbgp`).
    #[derive(Debug, Clone)]
    pub struct SbgpBox {
        /// The version of the box (0 or 1).
        pub version: u8,
        /// The flags of the box.
        pub flags: SbgpFlags,
        /// A FourCC identifying the type of grouping.
        pub grouping_type: FourCC,
        /// An additional parameter for the grouping (version 1 only).
        pub grouping_type_parameter: Option<u32>,
        /// The entries.
        pub entries: Vec<SbgpEntry>,
    }

    impl SbgpBox {
        /// Creates a `SbgpBox` from a `SbgpBoxView`.
        pub fn from_view(view: &SbgpBoxView<'_>) -> Result<SbgpBox> {
            let entries: Result<Vec<SbgpEntry>> = view.entries().collect();
            Ok(SbgpBox {
                version: view.version,
                flags: view.flags,
                grouping_type: view.grouping_type,
                grouping_type_parameter: view.grouping_type_parameter,
                entries: entries?,
            })
        }

        /// Parses a `SbgpBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<SbgpBox> {
            let view = SbgpBoxView::parse(payload)?;
            SbgpBox::from_view(&view)
        }
    }

    impl TryFrom<&SbgpBoxView<'_>> for SbgpBox {
        type Error = Error;

        fn try_from(value: &SbgpBoxView<'_>) -> Result<Self> {
            SbgpBox::from_view(value)
        }
    }

    impl TryFrom<&BoxView<'_>> for SbgpBox {
        type Error = Error;

        fn try_from(value: &BoxView<'_>) -> Result<Self> {
            let view = SbgpBoxView::try_from(value)?;
            SbgpBox::from_view(&view)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sbgp_payload_v0(grouping_type: &[u8; 4], entries: &[(u32, u32)]) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(0); // version
        data.extend_from_slice(&[0, 0, 0]); // flags
        data.extend_from_slice(grouping_type);
        data.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        for (sample_count, group_description_index) in entries {
            data.extend_from_slice(&sample_count.to_be_bytes());
            data.extend_from_slice(&group_description_index.to_be_bytes());
        }
        data
    }

    fn make_sbgp_payload_v1(
        grouping_type: &[u8; 4],
        grouping_type_parameter: u32,
        entries: &[(u32, u32)],
    ) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(1); // version
        data.extend_from_slice(&[0, 0, 0]); // flags
        data.extend_from_slice(grouping_type);
        data.extend_from_slice(&grouping_type_parameter.to_be_bytes());
        data.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        for (sample_count, group_description_index) in entries {
            data.extend_from_slice(&sample_count.to_be_bytes());
            data.extend_from_slice(&group_description_index.to_be_bytes());
        }
        data
    }

    #[test]
    fn parse_sbgp_v0_empty() {
        let payload = make_sbgp_payload_v0(b"roll", &[]);
        let sbgp = SbgpBoxView::parse(&payload).unwrap();

        assert_eq!(sbgp.version, 0);
        assert_eq!(sbgp.grouping_type, FourCC::new(*b"roll"));
        assert!(sbgp.grouping_type_parameter.is_none());
        assert_eq!(sbgp.entry_count, 0);
        assert_eq!(sbgp.entries().count(), 0);
    }

    #[test]
    fn parse_sbgp_v0_with_entries() {
        let entries = vec![(10, 1), (20, 2), (30, 0)];
        let payload = make_sbgp_payload_v0(b"seig", &entries);
        let sbgp = SbgpBoxView::parse(&payload).unwrap();

        assert_eq!(sbgp.version, 0);
        assert_eq!(sbgp.grouping_type, FourCC::new(*b"seig"));
        assert_eq!(sbgp.entry_count, 3);

        let parsed_entries: Vec<_> = sbgp.entries().collect();
        assert_eq!(parsed_entries.len(), 3);
        assert_eq!(parsed_entries[0].as_ref().unwrap().sample_count, 10);
        assert_eq!(parsed_entries[0].as_ref().unwrap().group_description_index, 1);
        assert_eq!(parsed_entries[1].as_ref().unwrap().sample_count, 20);
        assert_eq!(parsed_entries[2].as_ref().unwrap().group_description_index, 0);
    }

    #[test]
    fn parse_sbgp_v1_with_parameter() {
        let entries = vec![(5, 1)];
        let payload = make_sbgp_payload_v1(b"roll", 0x12345678, &entries);
        let sbgp = SbgpBoxView::parse(&payload).unwrap();

        assert_eq!(sbgp.version, 1);
        assert_eq!(sbgp.grouping_type, FourCC::new(*b"roll"));
        assert_eq!(sbgp.grouping_type_parameter, Some(0x12345678));
        assert_eq!(sbgp.entry_count, 1);
    }

    #[test]
    fn parse_sbgp_invalid_version() {
        let mut data = Vec::new();
        data.push(2); // invalid version
        data.extend_from_slice(&[0, 0, 0]);
        data.extend_from_slice(b"roll");
        data.extend_from_slice(&0u32.to_be_bytes());

        let result = SbgpBoxView::parse(&data);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::InvalidBoxVersion { .. }));
        }
    }

    #[test]
    fn parse_sbgp_size_mismatch() {
        let mut payload = make_sbgp_payload_v0(b"roll", &[(10, 1)]);
        // Corrupt by adding extra data
        payload.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);

        let result = SbgpBoxView::parse(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn try_from_box_view_success() {
        let payload = make_sbgp_payload_v0(b"roll", &[]);

        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"sbgp");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxView::parse_in(&mut cursor).unwrap();
        let sbgp = SbgpBoxView::try_from(&box_view).unwrap();

        assert_eq!(sbgp.grouping_type, FourCC::new(*b"roll"));
    }

    #[test]
    fn try_from_box_view_wrong_type() {
        let payload = make_sbgp_payload_v0(b"roll", &[]);

        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"sgpd");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxView::parse_in(&mut cursor).unwrap();
        let result = SbgpBoxView::try_from(&box_view);

        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    mod alloc_tests {
        use super::*;

        #[test]
        fn sbgp_box_from_view() {
            let entries = vec![(10, 1), (20, 2)];
            let payload = make_sbgp_payload_v0(b"roll", &entries);
            let view = SbgpBoxView::parse(&payload).unwrap();
            let owned = SbgpBox::from_view(&view).unwrap();

            assert_eq!(owned.grouping_type, FourCC::new(*b"roll"));
            assert_eq!(owned.entries.len(), 2);
            assert_eq!(owned.entries[0].sample_count, 10);
            assert_eq!(owned.entries[1].group_description_index, 2);
        }

        #[test]
        fn sbgp_box_parse() {
            let payload = make_sbgp_payload_v0(b"seig", &[]);
            let owned = SbgpBox::parse(&payload).unwrap();

            assert_eq!(owned.grouping_type, FourCC::new(*b"seig"));
            assert_eq!(owned.entries.len(), 0);
        }
    }
}
