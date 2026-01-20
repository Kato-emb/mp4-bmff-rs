use crate::cursor::ReadCursor;

use crate::BoxType;
use crate::BoxFrame;
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
            let first_chunk = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            let samples_per_chunk = u32::from_be_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
            let sample_description_index =
                u32::from_be_bytes([chunk[8], chunk[9], chunk[10], chunk[11]]);

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

impl<'a> TryFrom<BoxFrame<'a>> for StscBoxView<'a> {
    type Error = Error;

    fn try_from(value: BoxFrame<'a>) -> Result<Self> {
        if value.boxtype() != BoxType::STSC {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::STSC,
                found: value.boxtype(),
            }));
        }

        StscBoxView::parse(value.payload())
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

    fn make_full_box_header(version: u8, flags: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(version);
        data.extend_from_slice(&flags.to_be_bytes()[1..4]);
        data
    }

    fn make_stsc_payload(entries: Vec<StscEntry>) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(0, 0));
        payload.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        for entry in entries {
            payload.extend_from_slice(&entry.first_chunk.to_be_bytes());
            payload.extend_from_slice(&entry.samples_per_chunk.to_be_bytes());
            payload.extend_from_slice(&entry.sample_description_index.to_be_bytes());
        }
        payload
    }

    #[test]
    fn parse_and_iterate_entries() {
        // Empty
        let payload = make_stsc_payload(vec![]);
        let stsc = StscBoxView::parse(&payload).unwrap();
        assert_eq!(stsc.entry_count, 0);
        assert_eq!(stsc.entries().count(), 0);

        // Single
        let payload = make_stsc_payload(vec![StscEntry {
            first_chunk: 1,
            samples_per_chunk: 10,
            sample_description_index: 1,
        }]);
        let stsc = StscBoxView::parse(&payload).unwrap();
        let entry = stsc.entries().next().unwrap().unwrap();
        assert_eq!(entry.first_chunk, 1);
        assert_eq!(entry.samples_per_chunk, 10);

        // Multiple
        let entries = vec![
            StscEntry { first_chunk: 1, samples_per_chunk: 10, sample_description_index: 1 },
            StscEntry { first_chunk: 5, samples_per_chunk: 20, sample_description_index: 2 },
        ];
        let payload = make_stsc_payload(entries.clone());
        let stsc = StscBoxView::parse(&payload).unwrap();
        let parsed: Vec<_> = stsc.entries().map(|r| r.unwrap()).collect();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].first_chunk, 1);
        assert_eq!(parsed[1].samples_per_chunk, 20);
    }

    #[test]
    fn invalid_size() {
        let mut payload = make_full_box_header(0, 0);
        payload.extend_from_slice(&2u32.to_be_bytes());
        payload.extend_from_slice(&1u32.to_be_bytes());
        payload.extend_from_slice(&10u32.to_be_bytes());
        payload.extend_from_slice(&1u32.to_be_bytes());
        assert!(StscBoxView::parse(&payload).is_err());
    }

    #[test]
    fn wrong_box_type() {
        let payload = make_stsc_payload(vec![]);
        let mut box_data = Vec::new();
        box_data.extend_from_slice(&(8 + payload.len() as u32).to_be_bytes());
        box_data.extend_from_slice(b"stco");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxFrame::parse_in(&mut cursor).unwrap();
        let result = StscBoxView::try_from(box_view);
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn owned_conversion() {
        let entries = vec![
            StscEntry { first_chunk: 1, samples_per_chunk: 10, sample_description_index: 1 },
            StscEntry { first_chunk: 5, samples_per_chunk: 20, sample_description_index: 2 },
        ];
        let payload = make_stsc_payload(entries.clone());
        let view = StscBoxView::parse(&payload).unwrap();
        let owned = StscBox::from_view(&view).unwrap();
        assert_eq!(owned.entries.len(), 2);
        assert_eq!(owned.entries[0].first_chunk, 1);
        assert_eq!(owned.entries[1].samples_per_chunk, 20);
    }
}
