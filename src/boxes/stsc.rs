use crate::cursor::ReadCursor;

use crate::BoxType;
use crate::BoxView;
use crate::error::*;
use crate::header::FullBoxFlags;
use crate::header::FullBoxHeader;

/// An entry in the Sample To Chunk Box (`stsc`).
#[derive(Debug, Clone, Copy)]
pub struct StscEntry {
    /// The first chunk number.
    pub first_chunk: u32,
    /// The samples per chunk.
    pub samples_per_chunk: u32,
    /// The sample description index.
    pub sample_description_index: u32,
}

/// A reference to a Sample To Chunk Box (`stsc`).
#[derive(Debug)]
pub struct StscBoxView<'a> {
    /// The version of the box.
    pub version: u8,
    /// The flags of the box.
    pub flags: StscFlags,
    /// The number of entries in the box.
    pub entry_count: u32,
    entries: &'a [u8],
}

impl<'a> StscBoxView<'a> {
    const ENTRY_SIZE: usize = 12;

    /// Returns an iterator over the entries in the Sample To Chunk Box.
    pub fn entries(&self) -> impl Iterator<Item = Result<StscEntry>> + 'a {
        let entry_bytes = self.entries;
        let entry_count = self.entry_count as usize;

        entry_bytes.chunks_exact(12).take(entry_count).map(|chunk| {
            let mut cursor = ReadCursor::new(chunk);

            let first_chunk = cursor
                .read_u32_be()
                .map_err(|e| Error::at(e.into(), cursor.position() as u64))?;

            let samples_per_chunk = cursor
                .read_u32_be()
                .map_err(|e| Error::at(e.into(), cursor.position() as u64))?;

            let sample_description_index = cursor
                .read_u32_be()
                .map_err(|e| Error::at(e.into(), cursor.position() as u64))?;

            Ok(StscEntry {
                first_chunk,
                samples_per_chunk,
                sample_description_index,
            })
        })
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<StscBoxView<'a>> {
        let full_box_header = FullBoxHeader::<StscSpec>::parse_in(cur)?;

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
                BoxType::STSC,
            ));
        }

        let entries = cur.take(cur.remaining())?;

        Ok(StscBoxView {
            version: full_box_header.version(),
            flags: full_box_header.flags(),
            entry_count,
            entries,
        })
    }

    /// Parses a `StscBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<StscBoxView<'a>> {
        let mut cur = ReadCursor::new(payload);
        let this = StscBoxView::parse_in(&mut cur)?;

        Ok(this)
    }
}

impl<'a> TryFrom<&'a [u8]> for StscBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        StscBoxView::parse(value)
    }
}

impl<'a> TryFrom<BoxView<'a>> for StscBoxView<'a> {
    type Error = Error;

    fn try_from(value: BoxView<'a>) -> std::result::Result<Self, Self::Error> {
        if value.header.boxtype() != BoxType::STSC {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::STSC,
                found: value.header.boxtype(),
            }));
        }

        StscBoxView::parse(value.payload)
    }
}

/// Specification for the Sample To Chunk Box (`stsc`).
pub struct StscSpec;

/// Flags for the Sample To Chunk Box (`stsc`).
pub type StscFlags = FullBoxFlags<StscSpec>;

#[cfg(feature = "alloc")]
pub use owned::StscBox;

#[cfg(feature = "alloc")]
mod owned {
    use super::*;

    /// An owned Sample To Chunk Box (`stsc`).
    pub struct StscBox {
        /// The version of the Sample To Chunk Box.
        pub version: u8,
        /// The flags of the Sample To Chunk Box.
        pub flags: StscFlags,
        /// The entries in the Sample To Chunk Box.
        pub entries: Vec<StscEntry>,
    }

    impl StscBox {
        /// Creates a `StscBox` from a `StscBoxView`.
        pub fn from_view(view: &StscBoxView<'_>) -> Result<StscBox> {
            let entries = view.entries().collect::<Result<Vec<StscEntry>>>()?;

            Ok(StscBox {
                version: view.version,
                flags: view.flags,
                entries,
            })
        }

        /// Parses a `StscBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<StscBox> {
            let view = StscBoxView::parse(payload)?;
            StscBox::from_view(&view)
        }
    }

    impl TryFrom<&StscBoxView<'_>> for StscBox {
        type Error = Error;

        fn try_from(value: &StscBoxView<'_>) -> Result<Self> {
            StscBox::from_view(value)
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

    /// Creates a stsc box payload.
    fn make_stsc_payload(entries: Vec<StscEntry>) -> Vec<u8> {
        let mut payload = Vec::new();
        // FullBoxHeader: version=0, flags=0
        payload.extend_from_slice(&make_full_box_header(0, 0));
        // entry_count
        payload.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        // entries
        for entry in entries {
            payload.extend_from_slice(&entry.first_chunk.to_be_bytes());
            payload.extend_from_slice(&entry.samples_per_chunk.to_be_bytes());
            payload.extend_from_slice(&entry.sample_description_index.to_be_bytes());
        }
        payload
    }

    #[test]
    fn parse_stsc_empty() {
        let payload = make_stsc_payload(vec![]);
        let stsc = StscBoxView::parse(&payload).unwrap();

        assert_eq!(stsc.version, 0);
        assert_eq!(stsc.entry_count, 0);
        assert_eq!(stsc.entries().count(), 0);
    }

    #[test]
    fn parse_stsc_single_entry() {
        let entries = vec![StscEntry {
            first_chunk: 1,
            samples_per_chunk: 10,
            sample_description_index: 1,
        }];
        let payload = make_stsc_payload(entries.clone());
        let stsc = StscBoxView::parse(&payload).unwrap();

        assert_eq!(stsc.version, 0);
        assert_eq!(stsc.entry_count, 1);

        let parsed_entries: Vec<_> = stsc.entries().collect();
        assert_eq!(parsed_entries.len(), 1);

        let entry = parsed_entries[0].as_ref().unwrap();
        assert_eq!(entry.first_chunk, 1);
        assert_eq!(entry.samples_per_chunk, 10);
        assert_eq!(entry.sample_description_index, 1);
    }

    #[test]
    fn parse_stsc_multiple_entries() {
        let entries = vec![
            StscEntry {
                first_chunk: 1,
                samples_per_chunk: 12,
                sample_description_index: 1,
            },
            StscEntry {
                first_chunk: 5,
                samples_per_chunk: 8,
                sample_description_index: 1,
            },
            StscEntry {
                first_chunk: 10,
                samples_per_chunk: 10,
                sample_description_index: 2,
            },
        ];
        let payload = make_stsc_payload(entries.clone());
        let stsc = StscBoxView::parse(&payload).unwrap();

        assert_eq!(stsc.version, 0);
        assert_eq!(stsc.entry_count, 3);

        let parsed_entries: Vec<_> = stsc.entries().collect();
        assert_eq!(parsed_entries.len(), 3);

        for (i, parsed) in parsed_entries.iter().enumerate() {
            let entry = parsed.as_ref().unwrap();
            assert_eq!(entry.first_chunk, entries[i].first_chunk);
            assert_eq!(entry.samples_per_chunk, entries[i].samples_per_chunk);
            assert_eq!(
                entry.sample_description_index,
                entries[i].sample_description_index
            );
        }
    }

    #[test]
    fn parse_stsc_version1() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(1, 0x000123));
        payload.extend_from_slice(&1u32.to_be_bytes()); // entry_count
        payload.extend_from_slice(&1u32.to_be_bytes()); // first_chunk
        payload.extend_from_slice(&5u32.to_be_bytes()); // samples_per_chunk
        payload.extend_from_slice(&1u32.to_be_bytes()); // sample_description_index

        let stsc = StscBoxView::parse(&payload).unwrap();

        assert_eq!(stsc.version, 1);
        assert_eq!(stsc.flags.get(), 0x000123);
        assert_eq!(stsc.entry_count, 1);
    }

    // Error case tests

    #[test]
    fn parse_stsc_invalid_size() {
        let mut payload = make_full_box_header(0, 0);
        payload.extend_from_slice(&1u32.to_be_bytes()); // entry_count = 1
        payload.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7]); // Only 7 bytes (not multiple of 12)

        let result = StscBoxView::parse(&payload);
        assert!(result.is_err());

        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::InvalidBoxSize { .. }));
        }
    }

    #[test]
    fn parse_stsc_entry_count_mismatch() {
        // Parse succeeds even with mismatched entry_count,
        // but the iterator will only yield the available entries
        let mut payload = make_full_box_header(0, 0);
        payload.extend_from_slice(&3u32.to_be_bytes()); // entry_count = 3
        payload.extend_from_slice(&1u32.to_be_bytes()); // first_chunk
        payload.extend_from_slice(&10u32.to_be_bytes()); // samples_per_chunk
        payload.extend_from_slice(&1u32.to_be_bytes()); // sample_description_index
        // Only 1 entry provided instead of 3

        let stsc = StscBoxView::parse(&payload).unwrap();
        assert_eq!(stsc.entry_count, 3);

        // Iterator will only yield 1 entry (what's actually available)
        let entries: Vec<_> = stsc.entries().collect();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_ok());
    }

    #[test]
    fn try_from_byte_slice() {
        let entries = vec![StscEntry {
            first_chunk: 1,
            samples_per_chunk: 20,
            sample_description_index: 1,
        }];
        let payload = make_stsc_payload(entries);

        let stsc = StscBoxView::try_from(payload.as_slice()).unwrap();

        assert_eq!(stsc.entry_count, 1);
        let entry = stsc.entries().next().unwrap().unwrap();
        assert_eq!(entry.first_chunk, 1);
        assert_eq!(entry.samples_per_chunk, 20);
        assert_eq!(entry.sample_description_index, 1);
    }

    #[test]
    fn try_from_box_view_success() {
        let entries = vec![StscEntry {
            first_chunk: 1,
            samples_per_chunk: 15,
            sample_description_index: 2,
        }];
        let payload = make_stsc_payload(entries);

        // Create a complete box with header
        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"stsc");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxView::parse_in(&mut cursor).unwrap();
        let stsc = StscBoxView::try_from(box_view).unwrap();

        assert_eq!(stsc.entry_count, 1);
    }

    #[test]
    fn try_from_box_view_wrong_type() {
        let payload = make_stsc_payload(vec![]);

        // Create a box with wrong type
        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"stts"); // Wrong type
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxView::parse_in(&mut cursor).unwrap();
        let result = StscBoxView::try_from(box_view);

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
        fn stsc_box_from_view() {
            let entries = vec![
                StscEntry {
                    first_chunk: 1,
                    samples_per_chunk: 12,
                    sample_description_index: 1,
                },
                StscEntry {
                    first_chunk: 5,
                    samples_per_chunk: 8,
                    sample_description_index: 1,
                },
            ];
            let payload = make_stsc_payload(entries.clone());
            let stsc_view = StscBoxView::parse(&payload).unwrap();
            let stsc_box = StscBox::from_view(&stsc_view).unwrap();

            assert_eq!(stsc_box.version, stsc_view.version);
            assert_eq!(stsc_box.flags.get(), stsc_view.flags.get());
            assert_eq!(stsc_box.entries.len(), 2);
            assert_eq!(stsc_box.entries[0].first_chunk, 1);
            assert_eq!(stsc_box.entries[0].samples_per_chunk, 12);
            assert_eq!(stsc_box.entries[0].sample_description_index, 1);
            assert_eq!(stsc_box.entries[1].first_chunk, 5);
            assert_eq!(stsc_box.entries[1].samples_per_chunk, 8);
            assert_eq!(stsc_box.entries[1].sample_description_index, 1);
        }

        #[test]
        fn stsc_box_parse() {
            let entries = vec![StscEntry {
                first_chunk: 1,
                samples_per_chunk: 25,
                sample_description_index: 3,
            }];
            let payload = make_stsc_payload(entries);
            let stsc_box = StscBox::parse(&payload).unwrap();

            assert_eq!(stsc_box.entries.len(), 1);
            assert_eq!(stsc_box.entries[0].first_chunk, 1);
            assert_eq!(stsc_box.entries[0].samples_per_chunk, 25);
            assert_eq!(stsc_box.entries[0].sample_description_index, 3);
        }

        #[test]
        fn stsc_box_try_from() {
            let entries = vec![StscEntry {
                first_chunk: 10,
                samples_per_chunk: 5,
                sample_description_index: 2,
            }];
            let payload = make_stsc_payload(entries);
            let stsc_view = StscBoxView::parse(&payload).unwrap();
            let stsc_box: StscBox = (&stsc_view).try_into().unwrap();

            assert_eq!(stsc_box.entries.len(), 1);
            assert_eq!(stsc_box.entries[0].first_chunk, 10);
            assert_eq!(stsc_box.entries[0].samples_per_chunk, 5);
            assert_eq!(stsc_box.entries[0].sample_description_index, 2);
        }

        #[test]
        fn stsc_box_empty() {
            let payload = make_stsc_payload(vec![]);
            let stsc_box = StscBox::parse(&payload).unwrap();

            assert_eq!(stsc_box.entries.len(), 0);
        }
    }
}
