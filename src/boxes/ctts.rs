use crate::cursor::ReadCursor;

use crate::BoxType;
use crate::BoxView;
use crate::error::*;
use crate::header::FullBoxFlags;
use crate::header::FullBoxHeader;

/// An entry in the Composition Time to Sample Box (`ctts`).
#[derive(Debug, Clone, Copy)]
pub struct CttsEntry {
    /// The number of consecutive samples with the same composition offset.
    pub sample_count: u32,
    /// The composition offset for each sample in the group.
    /// This is a signed value to handle version 1 negative offsets.
    pub sample_offset: i32,
}

/// A reference to a Composition Time to Sample Box (`ctts`).
#[derive(Debug)]
pub struct CttsBoxView<'a> {
    /// The version of the box (0 or 1).
    pub version: u8,
    /// The flags of the box.
    pub flags: CttsFlags,
    /// The number of entries in the box.
    pub entry_count: u32,
    entries: &'a [u8],
}

impl<'a> CttsBoxView<'a> {
    const ENTRY_SIZE: usize = 8;

    /// Returns an iterator over the entries in the Composition Time to Sample Box (`ctts`).
    pub fn entries(&self) -> impl Iterator<Item = Result<CttsEntry>> + 'a {
        let entry_bytes = self.entries;
        let entry_count = self.entry_count as usize;
        let version = self.version;

        entry_bytes.chunks_exact(8).take(entry_count).map(move |chunk| {
            let mut cursor = ReadCursor::new(chunk);

            let sample_count = cursor
                .read_u32_be()
                .map_err(|e| Error::at(e.into(), cursor.position() as u64))?;

            // Version 0: unsigned 32-bit offset
            // Version 1: signed 32-bit offset
            let sample_offset = if version == 0 {
                cursor
                    .read_u32_be()
                    .map_err(|e| Error::at(e.into(), cursor.position() as u64))? as i32
            } else {
                cursor
                    .read_i32_be()
                    .map_err(|e| Error::at(e.into(), cursor.position() as u64))?
            };

            Ok(CttsEntry {
                sample_count,
                sample_offset,
            })
        })
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<CttsBoxView<'a>> {
        let full_box_header = FullBoxHeader::<CttsSpec>::parse_in(cur)?;

        let version = full_box_header.version();
        if version > 1 {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxVersion {
                    reason: "ctts version must be 0 or 1",
                    got: version,
                },
                BoxType::CTTS,
            ));
        }

        let entry_count = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        let expected_size = entry_count as usize * Self::ENTRY_SIZE;

        if cur.remaining() != expected_size {
            return Err(Error::at_in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Entries length does not match entry count",
                    got: cur.remaining() as u64,
                },
                cur.position() as u64,
                BoxType::CTTS,
            ));
        }

        let entries = cur.take(cur.remaining())?;

        Ok(CttsBoxView {
            version: full_box_header.version(),
            flags: full_box_header.flags(),
            entry_count,
            entries,
        })
    }

    /// Parses a `CttsBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<CttsBoxView<'a>> {
        let mut cur = ReadCursor::new(payload);
        let this = CttsBoxView::parse_in(&mut cur)?;

        Ok(this)
    }
}

impl<'a> TryFrom<&'a [u8]> for CttsBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        CttsBoxView::parse(value)
    }
}

impl<'a> TryFrom<&BoxView<'a>> for CttsBoxView<'a> {
    type Error = Error;

    fn try_from(value: &BoxView<'a>) -> std::result::Result<Self, Self::Error> {
        if value.header.boxtype() != BoxType::CTTS {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::CTTS,
                found: value.header.boxtype(),
            }));
        }

        CttsBoxView::parse(value.payload)
    }
}

/// Specification for the Composition Time to Sample Box (`ctts`).
pub struct CttsSpec;

/// Flags for the Composition Time to Sample Box (`ctts`).
pub type CttsFlags = FullBoxFlags<CttsSpec>;

#[cfg(feature = "alloc")]
pub use owned::CttsBox;

#[cfg(feature = "alloc")]
mod owned {
    use super::*;

    /// An owned Composition Time to Sample Box (`ctts`).
    pub struct CttsBox {
        /// The version of the box.
        pub version: u8,
        /// The flags of the box.
        pub flags: CttsFlags,
        /// The entries in the box.
        pub entries: Vec<CttsEntry>,
    }

    impl CttsBox {
        /// Creates a `CttsBox` from a `CttsBoxView`.
        pub fn from_view(view: &CttsBoxView<'_>) -> Result<CttsBox> {
            let entries: Result<Vec<CttsEntry>> = view.entries().collect();
            Ok(CttsBox {
                version: view.version,
                flags: view.flags,
                entries: entries?,
            })
        }

        /// Parses a `CttsBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<CttsBox> {
            let view = CttsBoxView::parse(payload)?;
            CttsBox::from_view(&view)
        }
    }

    impl TryFrom<&CttsBoxView<'_>> for CttsBox {
        type Error = Error;

        fn try_from(value: &CttsBoxView<'_>) -> Result<Self> {
            CttsBox::from_view(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a FullBoxHeader payload (version + flags).
    fn make_full_box_header(version: u8, flags: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(version);
        data.extend_from_slice(&flags.to_be_bytes()[1..4]); // Only 3 bytes for flags
        data
    }

    /// Creates a ctts box payload with version 0 (unsigned offsets).
    fn make_ctts_payload_v0(entries: Vec<(u32, u32)>) -> Vec<u8> {
        let mut payload = Vec::new();
        // FullBoxHeader: version=0, flags=0
        payload.extend_from_slice(&make_full_box_header(0, 0));
        // entry_count
        payload.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        // entries
        for (sample_count, sample_offset) in entries {
            payload.extend_from_slice(&sample_count.to_be_bytes());
            payload.extend_from_slice(&sample_offset.to_be_bytes());
        }
        payload
    }

    /// Creates a ctts box payload with version 1 (signed offsets).
    fn make_ctts_payload_v1(entries: Vec<(u32, i32)>) -> Vec<u8> {
        let mut payload = Vec::new();
        // FullBoxHeader: version=1, flags=0
        payload.extend_from_slice(&make_full_box_header(1, 0));
        // entry_count
        payload.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        // entries
        for (sample_count, sample_offset) in entries {
            payload.extend_from_slice(&sample_count.to_be_bytes());
            payload.extend_from_slice(&sample_offset.to_be_bytes());
        }
        payload
    }

    #[test]
    fn parse_ctts_empty() {
        let payload = make_ctts_payload_v0(vec![]);
        let ctts = CttsBoxView::parse(&payload).unwrap();

        assert_eq!(ctts.version, 0);
        assert_eq!(ctts.entry_count, 0);
        assert_eq!(ctts.entries().count(), 0);
    }

    #[test]
    fn parse_ctts_single_entry_v0() {
        let entries = vec![(100, 1000u32)];
        let payload = make_ctts_payload_v0(entries.clone());
        let ctts = CttsBoxView::parse(&payload).unwrap();

        assert_eq!(ctts.version, 0);
        assert_eq!(ctts.entry_count, 1);

        let parsed_entries: Vec<_> = ctts.entries().collect();
        assert_eq!(parsed_entries.len(), 1);

        let entry = parsed_entries[0].as_ref().unwrap();
        assert_eq!(entry.sample_count, 100);
        assert_eq!(entry.sample_offset, 1000);
    }

    #[test]
    fn parse_ctts_multiple_entries_v0() {
        let entries = vec![(100, 1000u32), (200, 2000u32), (300, 3000u32)];
        let payload = make_ctts_payload_v0(entries.clone());
        let ctts = CttsBoxView::parse(&payload).unwrap();

        assert_eq!(ctts.version, 0);
        assert_eq!(ctts.entry_count, 3);

        let parsed_entries: Vec<_> = ctts.entries().collect();
        assert_eq!(parsed_entries.len(), 3);

        for (i, parsed) in parsed_entries.iter().enumerate() {
            let entry = parsed.as_ref().unwrap();
            assert_eq!(entry.sample_count, entries[i].0);
            assert_eq!(entry.sample_offset, entries[i].1 as i32);
        }
    }

    #[test]
    fn parse_ctts_version1_positive_offset() {
        let entries = vec![(50, 100i32)];
        let payload = make_ctts_payload_v1(entries.clone());
        let ctts = CttsBoxView::parse(&payload).unwrap();

        assert_eq!(ctts.version, 1);
        assert_eq!(ctts.entry_count, 1);

        let entry = ctts.entries().next().unwrap().unwrap();
        assert_eq!(entry.sample_count, 50);
        assert_eq!(entry.sample_offset, 100);
    }

    #[test]
    fn parse_ctts_version1_negative_offset() {
        let entries = vec![(50, -100i32), (100, -200i32)];
        let payload = make_ctts_payload_v1(entries.clone());
        let ctts = CttsBoxView::parse(&payload).unwrap();

        assert_eq!(ctts.version, 1);
        assert_eq!(ctts.entry_count, 2);

        let parsed_entries: Vec<_> = ctts.entries().collect();
        assert_eq!(parsed_entries.len(), 2);

        let entry0 = parsed_entries[0].as_ref().unwrap();
        assert_eq!(entry0.sample_count, 50);
        assert_eq!(entry0.sample_offset, -100);

        let entry1 = parsed_entries[1].as_ref().unwrap();
        assert_eq!(entry1.sample_count, 100);
        assert_eq!(entry1.sample_offset, -200);
    }

    #[test]
    fn parse_ctts_version1_with_flags() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(1, 0x000042));
        payload.extend_from_slice(&1u32.to_be_bytes()); // entry_count
        payload.extend_from_slice(&50u32.to_be_bytes()); // sample_count
        payload.extend_from_slice(&(-50i32).to_be_bytes()); // sample_offset

        let ctts = CttsBoxView::parse(&payload).unwrap();

        assert_eq!(ctts.version, 1);
        assert_eq!(ctts.flags.get(), 0x000042);
        assert_eq!(ctts.entry_count, 1);

        let entry = ctts.entries().next().unwrap().unwrap();
        assert_eq!(entry.sample_offset, -50);
    }

    // Error case tests

    #[test]
    fn parse_ctts_invalid_size() {
        let mut payload = make_full_box_header(0, 0);
        payload.extend_from_slice(&1u32.to_be_bytes()); // entry_count = 1
        payload.extend_from_slice(&[1, 2, 3, 4, 5]); // Only 5 bytes (not multiple of 8)

        let result = CttsBoxView::parse(&payload);
        assert!(result.is_err());

        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::InvalidBoxSize { .. }));
        }
    }

    #[test]
    fn parse_ctts_entry_count_mismatch() {
        // Parse fails when entry_count does not match the actual payload size
        let mut payload = make_full_box_header(0, 0);
        payload.extend_from_slice(&2u32.to_be_bytes()); // entry_count = 2
        payload.extend_from_slice(&100u32.to_be_bytes()); // sample_count
        payload.extend_from_slice(&200u32.to_be_bytes()); // sample_offset
        // Second entry is missing

        let result = CttsBoxView::parse(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn try_from_byte_slice() {
        let entries = vec![(42, 84u32)];
        let payload = make_ctts_payload_v0(entries);

        let ctts = CttsBoxView::try_from(payload.as_slice()).unwrap();

        assert_eq!(ctts.entry_count, 1);
        let entry = ctts.entries().next().unwrap().unwrap();
        assert_eq!(entry.sample_count, 42);
        assert_eq!(entry.sample_offset, 84);
    }

    #[test]
    fn try_from_box_view_success() {
        let entries = vec![(10, 20u32)];
        let payload = make_ctts_payload_v0(entries);

        // Create a complete box with header
        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"ctts");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxView::parse_in(&mut cursor).unwrap();
        let ctts = CttsBoxView::try_from(&box_view).unwrap();

        assert_eq!(ctts.entry_count, 1);
    }

    #[test]
    fn try_from_box_view_wrong_type() {
        let payload = make_ctts_payload_v0(vec![]);

        // Create a box with wrong type
        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"stts"); // Wrong type
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxView::parse_in(&mut cursor).unwrap();
        let result = CttsBoxView::try_from(&box_view);

        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::MismatchedBoxType { .. }));
        }
    }

    // Owned type tests (requires alloc feature)

    #[cfg(feature = "alloc")]
    mod alloc_tests {
        use super::*;

        #[test]
        fn ctts_box_from_view() {
            let entries = vec![(100, 1000u32), (200, 2000u32)];
            let payload = make_ctts_payload_v0(entries.clone());
            let ctts_view = CttsBoxView::parse(&payload).unwrap();
            let ctts_box = CttsBox::from_view(&ctts_view).unwrap();

            assert_eq!(ctts_box.version, ctts_view.version);
            assert_eq!(ctts_box.flags.get(), ctts_view.flags.get());
            assert_eq!(ctts_box.entries.len(), 2);
            assert_eq!(ctts_box.entries[0].sample_count, 100);
            assert_eq!(ctts_box.entries[0].sample_offset, 1000);
            assert_eq!(ctts_box.entries[1].sample_count, 200);
            assert_eq!(ctts_box.entries[1].sample_offset, 2000);
        }

        #[test]
        fn ctts_box_parse() {
            let entries = vec![(42, 84u32)];
            let payload = make_ctts_payload_v0(entries);
            let ctts_box = CttsBox::parse(&payload).unwrap();

            assert_eq!(ctts_box.entries.len(), 1);
            assert_eq!(ctts_box.entries[0].sample_count, 42);
            assert_eq!(ctts_box.entries[0].sample_offset, 84);
        }

        #[test]
        fn ctts_box_try_from() {
            let entries = vec![(5, 10u32)];
            let payload = make_ctts_payload_v0(entries);
            let ctts_view = CttsBoxView::parse(&payload).unwrap();
            let ctts_box: CttsBox = (&ctts_view).try_into().unwrap();

            assert_eq!(ctts_box.entries.len(), 1);
            assert_eq!(ctts_box.entries[0].sample_count, 5);
            assert_eq!(ctts_box.entries[0].sample_offset, 10);
        }

        #[test]
        fn ctts_box_empty() {
            let payload = make_ctts_payload_v0(vec![]);
            let ctts_box = CttsBox::parse(&payload).unwrap();

            assert_eq!(ctts_box.entries.len(), 0);
        }

        #[test]
        fn ctts_box_version1_negative() {
            let entries = vec![(10, -50i32), (20, -100i32)];
            let payload = make_ctts_payload_v1(entries);
            let ctts_box = CttsBox::parse(&payload).unwrap();

            assert_eq!(ctts_box.version, 1);
            assert_eq!(ctts_box.entries.len(), 2);
            assert_eq!(ctts_box.entries[0].sample_offset, -50);
            assert_eq!(ctts_box.entries[1].sample_offset, -100);
        }
    }
}
