use crate::cursor::ReadCursor;

use crate::BoxType;
use crate::BoxView;
use crate::error::*;
use crate::header::FullBoxFlags;
use crate::header::FullBoxHeader;

/// An entry in the Chunk Offset Box (`stco`).
#[derive(Debug, Clone, Copy)]
pub struct StcoEntry {
    /// The chunk offset.
    pub chunk_offset: u32,
}

/// A reference to a Chunk Offset Box (`stco`).
#[derive(Debug)]
pub struct StcoBoxView<'a> {
    /// The version of the box.
    pub version: u8,
    /// The flags of the box.
    pub flags: StcoFlags,
    /// The number of entries in the box.
    pub entry_count: u32,
    entries: &'a [u8],
}

impl<'a> StcoBoxView<'a> {
    /// Returns an iterator over the entries in the Chunk Offset Box.
    pub fn entries(&self) -> impl Iterator<Item = Result<StcoEntry>> + 'a {
        let entry_bytes = self.entries;
        let entry_count = self.entry_count as usize;

        entry_bytes.chunks_exact(4).take(entry_count).map(|chunk| {
            let mut cursor = ReadCursor::new(chunk);

            let chunk_offset = cursor
                .read_u32_be()
                .map_err(|e| Error::at(e.into(), cursor.position() as u64))?;

            Ok(StcoEntry { chunk_offset })
        })
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<StcoBoxView<'a>> {
        let full_box_header = FullBoxHeader::<StcoSpec>::parse(cur)?;

        let entry_count = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        if !cur.remaining().is_multiple_of(4) {
            return Err(Error::at_in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Entries length is not a multiple of 4",
                    got: cur.remaining() as u64,
                },
                cur.position() as u64,
                BoxType::STCO,
            ));
        }

        let entries = cur.take(cur.remaining())?;

        Ok(StcoBoxView {
            version: full_box_header.version(),
            flags: full_box_header.flags(),
            entry_count,
            entries,
        })
    }

    /// Parses a `StcoBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<StcoBoxView<'a>> {
        let mut cur = ReadCursor::new(payload);
        let this = StcoBoxView::parse_in(&mut cur)?;

        Ok(this)
    }
}

impl<'a> TryFrom<&'a [u8]> for StcoBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        StcoBoxView::parse(value)
    }
}

impl<'a> TryFrom<BoxView<'a>> for StcoBoxView<'a> {
    type Error = Error;

    fn try_from(box_view: BoxView<'a>) -> Result<Self> {
        if box_view.header.boxtype() != BoxType::STCO {
            return Err(Error::new(ErrorKind::MissmatchedBoxType {
                expected: BoxType::STCO,
                found: box_view.header.boxtype(),
            }));
        }

        StcoBoxView::parse(box_view.payload)
    }
}

/// Specification for the Chunk Offset Box (`stco`).
pub struct StcoSpec;

/// Flags for the Chunk Offset Box (`stco`).
pub type StcoFlags = FullBoxFlags<StcoSpec>;

#[cfg(feature = "alloc")]
pub use owned::StcoBox;

#[cfg(feature = "alloc")]
mod owned {
    use super::*;

    /// An owned Chunk Offset Box (`stco`).
    pub struct StcoBox {
        /// The version of the box.
        pub version: u8,
        /// The flags of the box.
        pub flags: StcoFlags,
        /// The entries in the box.
        pub entries: Vec<StcoEntry>,
    }

    impl StcoBox {
        /// Creates a `StcoBox` from a `StcoBoxView`.
        pub fn from_view(view: &StcoBoxView<'_>) -> Result<StcoBox> {
            let entries = view.entries().collect::<Result<Vec<StcoEntry>>>()?;

            Ok(StcoBox {
                version: view.version,
                flags: view.flags,
                entries,
            })
        }

        /// Parses a `StcoBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<StcoBox> {
            let view = StcoBoxView::parse(payload)?;
            StcoBox::from_view(&view)
        }
    }

    impl TryFrom<&StcoBoxView<'_>> for StcoBox {
        type Error = Error;

        fn try_from(value: &StcoBoxView<'_>) -> Result<Self> {
            StcoBox::from_view(value)
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

    /// Creates a stco box payload.
    fn make_stco_payload(entries: Vec<StcoEntry>) -> Vec<u8> {
        let mut payload = Vec::new();
        // FullBoxHeader: version=0, flags=0
        payload.extend_from_slice(&make_full_box_header(0, 0));
        // entry_count
        payload.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        // entries
        for entry in entries {
            payload.extend_from_slice(&entry.chunk_offset.to_be_bytes());
        }
        payload
    }

    #[test]
    fn parse_stco_empty() {
        let payload = make_stco_payload(vec![]);
        let stco = StcoBoxView::parse(&payload).unwrap();

        assert_eq!(stco.version, 0);
        assert_eq!(stco.entry_count, 0);
        assert_eq!(stco.entries().count(), 0);
    }

    #[test]
    fn parse_stco_single_entry() {
        let entries = vec![StcoEntry { chunk_offset: 1000 }];
        let payload = make_stco_payload(entries.clone());
        let stco = StcoBoxView::parse(&payload).unwrap();

        assert_eq!(stco.version, 0);
        assert_eq!(stco.entry_count, 1);

        let parsed_entries: Vec<_> = stco.entries().collect();
        assert_eq!(parsed_entries.len(), 1);

        let entry = parsed_entries[0].as_ref().unwrap();
        assert_eq!(entry.chunk_offset, 1000);
    }

    #[test]
    fn parse_stco_multiple_entries() {
        let entries = vec![
            StcoEntry { chunk_offset: 1000 },
            StcoEntry { chunk_offset: 5000 },
            StcoEntry {
                chunk_offset: 10000,
            },
            StcoEntry {
                chunk_offset: 20000,
            },
        ];
        let payload = make_stco_payload(entries.clone());
        let stco = StcoBoxView::parse(&payload).unwrap();

        assert_eq!(stco.version, 0);
        assert_eq!(stco.entry_count, 4);

        let parsed_entries: Vec<_> = stco.entries().collect();
        assert_eq!(parsed_entries.len(), 4);

        for (i, parsed) in parsed_entries.iter().enumerate() {
            let entry = parsed.as_ref().unwrap();
            assert_eq!(entry.chunk_offset, entries[i].chunk_offset);
        }
    }

    #[test]
    fn parse_stco_version1() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(1, 0x000456));
        payload.extend_from_slice(&2u32.to_be_bytes()); // entry_count
        payload.extend_from_slice(&100u32.to_be_bytes()); // chunk_offset
        payload.extend_from_slice(&200u32.to_be_bytes()); // chunk_offset

        let stco = StcoBoxView::parse(&payload).unwrap();

        assert_eq!(stco.version, 1);
        assert_eq!(stco.flags.get(), 0x000456);
        assert_eq!(stco.entry_count, 2);
    }

    #[test]
    fn parse_stco_large_offsets() {
        let entries = vec![
            StcoEntry {
                chunk_offset: 0xFFFF_FFFF, // Max u32
            },
            StcoEntry { chunk_offset: 0 }, // Min u32
            StcoEntry {
                chunk_offset: 0x8000_0000, // Middle value
            },
        ];
        let payload = make_stco_payload(entries.clone());
        let stco = StcoBoxView::parse(&payload).unwrap();

        let parsed_entries: Vec<_> = stco.entries().collect();
        assert_eq!(parsed_entries.len(), 3);
        assert_eq!(
            parsed_entries[0].as_ref().unwrap().chunk_offset,
            0xFFFF_FFFF
        );
        assert_eq!(parsed_entries[1].as_ref().unwrap().chunk_offset, 0);
        assert_eq!(
            parsed_entries[2].as_ref().unwrap().chunk_offset,
            0x8000_0000
        );
    }

    // Error case tests

    #[test]
    fn parse_stco_invalid_size() {
        let mut payload = make_full_box_header(0, 0);
        payload.extend_from_slice(&1u32.to_be_bytes()); // entry_count = 1
        payload.extend_from_slice(&[1, 2, 3]); // Only 3 bytes (not multiple of 4)

        let result = StcoBoxView::parse(&payload);
        assert!(result.is_err());

        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::InvalidBoxSize { .. }));
        }
    }

    #[test]
    fn parse_stco_entry_count_mismatch() {
        // Parse succeeds even with mismatched entry_count,
        // but the iterator will only yield the available entries
        let mut payload = make_full_box_header(0, 0);
        payload.extend_from_slice(&5u32.to_be_bytes()); // entry_count = 5
        payload.extend_from_slice(&100u32.to_be_bytes()); // chunk_offset
        payload.extend_from_slice(&200u32.to_be_bytes()); // chunk_offset
        // Only 2 entries provided instead of 5

        let stco = StcoBoxView::parse(&payload).unwrap();
        assert_eq!(stco.entry_count, 5);

        // Iterator will only yield 2 entries (what's actually available)
        let entries: Vec<_> = stco.entries().collect();
        assert_eq!(entries.len(), 2);
        assert!(entries[0].is_ok());
        assert!(entries[1].is_ok());
    }

    #[test]
    fn try_from_byte_slice() {
        let entries = vec![StcoEntry {
            chunk_offset: 12345,
        }];
        let payload = make_stco_payload(entries);

        let stco = StcoBoxView::try_from(payload.as_slice()).unwrap();

        assert_eq!(stco.entry_count, 1);
        let entry = stco.entries().next().unwrap().unwrap();
        assert_eq!(entry.chunk_offset, 12345);
    }

    #[test]
    fn try_from_box_view_success() {
        let entries = vec![
            StcoEntry { chunk_offset: 1024 },
            StcoEntry { chunk_offset: 2048 },
        ];
        let payload = make_stco_payload(entries);

        // Create a complete box with header
        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"stco");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxView::parse_in(&mut cursor).unwrap();
        let stco = StcoBoxView::try_from(box_view).unwrap();

        assert_eq!(stco.entry_count, 2);
    }

    #[test]
    fn try_from_box_view_wrong_type() {
        let payload = make_stco_payload(vec![]);

        // Create a box with wrong type
        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"stsc"); // Wrong type
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxView::parse_in(&mut cursor).unwrap();
        let result = StcoBoxView::try_from(box_view);

        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::MissmatchedBoxType { .. }));
        }
    }

    // Owned type tests (requires alloc feature)

    #[cfg(feature = "alloc")]
    mod alloc_tests {
        use super::*;

        #[test]
        fn stco_box_from_view() {
            let entries = vec![
                StcoEntry { chunk_offset: 1000 },
                StcoEntry { chunk_offset: 2000 },
                StcoEntry { chunk_offset: 3000 },
            ];
            let payload = make_stco_payload(entries.clone());
            let stco_view = StcoBoxView::parse(&payload).unwrap();
            let stco_box = StcoBox::from_view(&stco_view).unwrap();

            assert_eq!(stco_box.version, stco_view.version);
            assert_eq!(stco_box.flags.get(), stco_view.flags.get());
            assert_eq!(stco_box.entries.len(), 3);
            assert_eq!(stco_box.entries[0].chunk_offset, 1000);
            assert_eq!(stco_box.entries[1].chunk_offset, 2000);
            assert_eq!(stco_box.entries[2].chunk_offset, 3000);
        }

        #[test]
        fn stco_box_parse() {
            let entries = vec![StcoEntry {
                chunk_offset: 99999,
            }];
            let payload = make_stco_payload(entries);
            let stco_box = StcoBox::parse(&payload).unwrap();

            assert_eq!(stco_box.entries.len(), 1);
            assert_eq!(stco_box.entries[0].chunk_offset, 99999);
        }

        #[test]
        fn stco_box_try_from() {
            let entries = vec![
                StcoEntry { chunk_offset: 500 },
                StcoEntry { chunk_offset: 1500 },
            ];
            let payload = make_stco_payload(entries);
            let stco_view = StcoBoxView::parse(&payload).unwrap();
            let stco_box: StcoBox = (&stco_view).try_into().unwrap();

            assert_eq!(stco_box.entries.len(), 2);
            assert_eq!(stco_box.entries[0].chunk_offset, 500);
            assert_eq!(stco_box.entries[1].chunk_offset, 1500);
        }

        #[test]
        fn stco_box_empty() {
            let payload = make_stco_payload(vec![]);
            let stco_box = StcoBox::parse(&payload).unwrap();

            assert_eq!(stco_box.entries.len(), 0);
        }
    }
}
