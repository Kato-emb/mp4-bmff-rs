use crate::cursor::ReadCursor;

use crate::BoxType;
use crate::BoxView;
use crate::error::*;
use crate::header::FullBoxFlags;
use crate::header::FullBoxHeader;

/// An entry in the Decoding Time to Sample Box (`stts`).
#[derive(Debug, Clone, Copy)]
pub struct SttsEntry {
    /// The number of consecutive samples with the same duration.
    pub sample_count: u32,
    /// The duration of each sample in the group.
    pub sample_delta: u32,
}

/// A reference to a Decoding Time to Sample Box (`stts`).
#[derive(Debug)]
pub struct SttsBoxView<'a> {
    /// The version of the box.
    pub version: u8,
    /// The flags of the box.
    pub flags: SttsFlags,
    /// The number of entries in the box.
    pub entry_count: u32,
    entries: &'a [u8],
}

impl<'a> SttsBoxView<'a> {
    const ENTRY_SIZE: usize = 8;

    /// Returns an iterator over the entries in the Decoding Time to Sample Box (`stts`).
    pub fn entries(&self) -> impl Iterator<Item = Result<SttsEntry>> + 'a {
        let entry_bytes = self.entries;
        let entry_count = self.entry_count as usize;

        entry_bytes.chunks_exact(8).take(entry_count).map(|chunk| {
            let sample_count = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            let sample_delta = u32::from_be_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);

            Ok(SttsEntry {
                sample_count,
                sample_delta,
            })
        })
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<SttsBoxView<'a>> {
        let full_box_header = FullBoxHeader::<SttsSpec>::parse_in(cur)?;

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
                BoxType::STTS,
            ));
        }

        let entries = cur.take(cur.remaining())?;

        Ok(SttsBoxView {
            version: full_box_header.version(),
            flags: full_box_header.flags(),
            entry_count,
            entries,
        })
    }

    /// Parses a `SttsBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<SttsBoxView<'a>> {
        let mut cur = ReadCursor::new(payload);
        let this = SttsBoxView::parse_in(&mut cur)?;

        Ok(this)
    }
}

impl<'a> TryFrom<&'a [u8]> for SttsBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        SttsBoxView::parse(value)
    }
}

impl<'a> TryFrom<&BoxView<'a>> for SttsBoxView<'a> {
    type Error = Error;

    fn try_from(value: &BoxView<'a>) -> std::result::Result<Self, Self::Error> {
        if value.header.boxtype() != BoxType::STTS {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::STTS,
                found: value.header.boxtype(),
            }));
        }

        SttsBoxView::parse(value.payload)
    }
}

/// Specification for the Decoding Time to Sample Box (`stts`).
pub struct SttsSpec;

/// Flags for the Decoding Time to Sample Box (`stts`).
pub type SttsFlags = FullBoxFlags<SttsSpec>;

#[cfg(feature = "alloc")]
pub use owned::SttsBox;

#[cfg(feature = "alloc")]
mod owned {
    use super::*;

    /// An owned Decoding Time to Sample Box (`stts`).
    pub struct SttsBox {
        /// The version of the box.
        pub version: u8,
        /// The flags of the box.
        pub flags: SttsFlags,
        /// The entries in the box.
        pub entries: Vec<SttsEntry>,
    }

    impl SttsBox {
        /// Creates a `SttsBox` from a `SttsBoxView`.
        pub fn from_view(view: &SttsBoxView<'_>) -> Result<SttsBox> {
            let entries: Result<Vec<SttsEntry>> = view.entries().collect();
            Ok(SttsBox {
                version: view.version,
                flags: view.flags,
                entries: entries?,
            })
        }

        /// Parses a `SttsBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<SttsBox> {
            let view = SttsBoxView::parse(payload)?;
            SttsBox::from_view(&view)
        }
    }

    impl TryFrom<&SttsBoxView<'_>> for SttsBox {
        type Error = Error;

        fn try_from(value: &SttsBoxView<'_>) -> Result<Self> {
            SttsBox::from_view(value)
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

    /// Creates a stts box payload.
    fn make_stts_payload(entries: Vec<SttsEntry>) -> Vec<u8> {
        let mut payload = Vec::new();
        // FullBoxHeader: version=0, flags=0
        payload.extend_from_slice(&make_full_box_header(0, 0));
        // entry_count
        payload.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        // entries
        for entry in entries {
            payload.extend_from_slice(&entry.sample_count.to_be_bytes());
            payload.extend_from_slice(&entry.sample_delta.to_be_bytes());
        }
        payload
    }

    #[test]
    fn parse_stts_empty() {
        let payload = make_stts_payload(vec![]);
        let stts = SttsBoxView::parse(&payload).unwrap();

        assert_eq!(stts.version, 0);
        assert_eq!(stts.entry_count, 0);
        assert_eq!(stts.entries().count(), 0);
    }

    #[test]
    fn parse_stts_single_entry() {
        let entries = vec![SttsEntry {
            sample_count: 100,
            sample_delta: 1000,
        }];
        let payload = make_stts_payload(entries.clone());
        let stts = SttsBoxView::parse(&payload).unwrap();

        assert_eq!(stts.version, 0);
        assert_eq!(stts.entry_count, 1);

        let parsed_entries: Vec<_> = stts.entries().collect();
        assert_eq!(parsed_entries.len(), 1);

        let entry = parsed_entries[0].as_ref().unwrap();
        assert_eq!(entry.sample_count, 100);
        assert_eq!(entry.sample_delta, 1000);
    }

    #[test]
    fn parse_stts_multiple_entries() {
        let entries = vec![
            SttsEntry {
                sample_count: 100,
                sample_delta: 1000,
            },
            SttsEntry {
                sample_count: 200,
                sample_delta: 2000,
            },
            SttsEntry {
                sample_count: 300,
                sample_delta: 3000,
            },
        ];
        let payload = make_stts_payload(entries.clone());
        let stts = SttsBoxView::parse(&payload).unwrap();

        assert_eq!(stts.version, 0);
        assert_eq!(stts.entry_count, 3);

        let parsed_entries: Vec<_> = stts.entries().collect();
        assert_eq!(parsed_entries.len(), 3);

        for (i, parsed) in parsed_entries.iter().enumerate() {
            let entry = parsed.as_ref().unwrap();
            assert_eq!(entry.sample_count, entries[i].sample_count);
            assert_eq!(entry.sample_delta, entries[i].sample_delta);
        }
    }

    #[test]
    fn parse_stts_version1() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(1, 0x000042));
        payload.extend_from_slice(&1u32.to_be_bytes()); // entry_count
        payload.extend_from_slice(&50u32.to_be_bytes()); // sample_count
        payload.extend_from_slice(&100u32.to_be_bytes()); // sample_delta

        let stts = SttsBoxView::parse(&payload).unwrap();

        assert_eq!(stts.version, 1);
        assert_eq!(stts.flags.get(), 0x000042);
        assert_eq!(stts.entry_count, 1);
    }

    // Error case tests

    #[test]
    fn parse_stts_invalid_size() {
        let mut payload = make_full_box_header(0, 0);
        payload.extend_from_slice(&1u32.to_be_bytes()); // entry_count = 1
        payload.extend_from_slice(&[1, 2, 3, 4, 5]); // Only 5 bytes (not multiple of 8)

        let result = SttsBoxView::parse(&payload);
        assert!(result.is_err());

        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::InvalidBoxSize { .. }));
        }
    }

    #[test]
    fn parse_stts_entry_count_mismatch() {
        // Parse fails when entry_count does not match the actual payload size
        let mut payload = make_full_box_header(0, 0);
        payload.extend_from_slice(&2u32.to_be_bytes()); // entry_count = 2
        payload.extend_from_slice(&100u32.to_be_bytes()); // sample_count
        payload.extend_from_slice(&200u32.to_be_bytes()); // sample_delta
        // Second entry is missing

        let result = SttsBoxView::parse(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn try_from_byte_slice() {
        let entries = vec![SttsEntry {
            sample_count: 42,
            sample_delta: 84,
        }];
        let payload = make_stts_payload(entries);

        let stts = SttsBoxView::try_from(payload.as_slice()).unwrap();

        assert_eq!(stts.entry_count, 1);
        let entry = stts.entries().next().unwrap().unwrap();
        assert_eq!(entry.sample_count, 42);
        assert_eq!(entry.sample_delta, 84);
    }

    #[test]
    fn try_from_box_view_success() {
        let entries = vec![SttsEntry {
            sample_count: 10,
            sample_delta: 20,
        }];
        let payload = make_stts_payload(entries);

        // Create a complete box with header
        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"stts");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxView::parse_in(&mut cursor).unwrap();
        let stts = SttsBoxView::try_from(&box_view).unwrap();

        assert_eq!(stts.entry_count, 1);
    }

    #[test]
    fn try_from_box_view_wrong_type() {
        let payload = make_stts_payload(vec![]);

        // Create a box with wrong type
        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"stsc"); // Wrong type
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxView::parse_in(&mut cursor).unwrap();
        let result = SttsBoxView::try_from(&box_view);

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
        fn stts_box_from_view() {
            let entries = vec![
                SttsEntry {
                    sample_count: 100,
                    sample_delta: 1000,
                },
                SttsEntry {
                    sample_count: 200,
                    sample_delta: 2000,
                },
            ];
            let payload = make_stts_payload(entries.clone());
            let stts_view = SttsBoxView::parse(&payload).unwrap();
            let stts_box = SttsBox::from_view(&stts_view).unwrap();

            assert_eq!(stts_box.version, stts_view.version);
            assert_eq!(stts_box.flags.get(), stts_view.flags.get());
            assert_eq!(stts_box.entries.len(), 2);
            assert_eq!(stts_box.entries[0].sample_count, 100);
            assert_eq!(stts_box.entries[0].sample_delta, 1000);
            assert_eq!(stts_box.entries[1].sample_count, 200);
            assert_eq!(stts_box.entries[1].sample_delta, 2000);
        }

        #[test]
        fn stts_box_parse() {
            let entries = vec![SttsEntry {
                sample_count: 42,
                sample_delta: 84,
            }];
            let payload = make_stts_payload(entries);
            let stts_box = SttsBox::parse(&payload).unwrap();

            assert_eq!(stts_box.entries.len(), 1);
            assert_eq!(stts_box.entries[0].sample_count, 42);
            assert_eq!(stts_box.entries[0].sample_delta, 84);
        }

        #[test]
        fn stts_box_try_from() {
            let entries = vec![SttsEntry {
                sample_count: 5,
                sample_delta: 10,
            }];
            let payload = make_stts_payload(entries);
            let stts_view = SttsBoxView::parse(&payload).unwrap();
            let stts_box: SttsBox = (&stts_view).try_into().unwrap();

            assert_eq!(stts_box.entries.len(), 1);
            assert_eq!(stts_box.entries[0].sample_count, 5);
            assert_eq!(stts_box.entries[0].sample_delta, 10);
        }

        #[test]
        fn stts_box_empty() {
            let payload = make_stts_payload(vec![]);
            let stts_box = SttsBox::parse(&payload).unwrap();

            assert_eq!(stts_box.entries.len(), 0);
        }
    }
}
