use crate::cursor::ReadCursor;

use crate::BoxType;
use crate::BoxView;
use crate::error::*;
use crate::header::FullBoxFlags;
use crate::header::FullBoxHeader;

/// An entry in the Sync Sample Box (`stss`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StssEntry {
    /// The sample number (1-indexed as per ISO specification).
    pub sample_number: u32,
}

/// A reference to a Sync Sample Box (`stss`).
///
/// This box identifies the sync (random access) samples within the stream.
#[derive(Debug)]
pub struct StssBoxView<'a> {
    /// The version of the box.
    pub version: u8,
    /// The flags of the box.
    pub flags: StssFlags,
    /// The number of entries in the box.
    pub entry_count: u32,
    entries: &'a [u8],
}

impl<'a> StssBoxView<'a> {
    const ENTRY_SIZE: usize = 4;

    /// Returns an iterator over the entries in the Sync Sample Box.
    pub fn entries(&self) -> impl Iterator<Item = Result<StssEntry>> + 'a {
        let entry_bytes = self.entries;
        let entry_count = self.entry_count as usize;

        entry_bytes
            .chunks_exact(Self::ENTRY_SIZE)
            .take(entry_count)
            .map(|chunk| {
                let mut cursor = ReadCursor::new(chunk);
                let sample_number = cursor
                    .read_u32_be()
                    .map_err(|e| Error::at(e.into(), cursor.position() as u64))?;
                Ok(StssEntry { sample_number })
            })
    }

    /// Checks if a given sample number is a sync sample.
    ///
    /// Note: This performs a linear search through the entries.
    /// For frequent lookups, consider collecting the entries into a set.
    pub fn is_sync_sample(&self, sample_number: u32) -> Result<bool> {
        for result in self.entries() {
            let entry = result?;
            if entry.sample_number == sample_number {
                return Ok(true);
            }
            // Since sample numbers are in increasing order, we can stop early
            if entry.sample_number > sample_number {
                return Ok(false);
            }
        }
        Ok(false)
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<StssBoxView<'a>> {
        let full_box_header = FullBoxHeader::<StssSpec>::parse_in(cur)?;

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
                BoxType::STSS,
            ));
        }

        let entries = cur.take(cur.remaining())?;

        Ok(StssBoxView {
            version: full_box_header.version(),
            flags: full_box_header.flags(),
            entry_count,
            entries,
        })
    }

    /// Parses a `StssBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<StssBoxView<'a>> {
        let mut cur = ReadCursor::new(payload);
        let this = StssBoxView::parse_in(&mut cur)?;

        Ok(this)
    }
}

impl<'a> TryFrom<&'a [u8]> for StssBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        StssBoxView::parse(value)
    }
}

impl<'a> TryFrom<&BoxView<'a>> for StssBoxView<'a> {
    type Error = Error;

    fn try_from(value: &BoxView<'a>) -> std::result::Result<Self, Self::Error> {
        if value.header.boxtype() != BoxType::STSS {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::STSS,
                found: value.header.boxtype(),
            }));
        }

        StssBoxView::parse(value.payload)
    }
}

/// Specification for the Sync Sample Box (`stss`).
pub struct StssSpec;

/// Flags for the Sync Sample Box (`stss`).
pub type StssFlags = FullBoxFlags<StssSpec>;

#[cfg(feature = "alloc")]
pub use owned::StssBox;

#[cfg(feature = "alloc")]
mod owned {
    use super::*;

    /// An owned Sync Sample Box (`stss`).
    #[derive(Debug, Clone)]
    pub struct StssBox {
        /// The version of the box.
        pub version: u8,
        /// The flags of the box.
        pub flags: StssFlags,
        /// The sync sample entries.
        pub entries: Vec<StssEntry>,
    }

    impl StssBox {
        /// Creates a `StssBox` from a `StssBoxView`.
        pub fn from_view(view: &StssBoxView<'_>) -> Result<StssBox> {
            let entries: Result<Vec<StssEntry>> = view.entries().collect();
            Ok(StssBox {
                version: view.version,
                flags: view.flags,
                entries: entries?,
            })
        }

        /// Parses a `StssBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<StssBox> {
            let view = StssBoxView::parse(payload)?;
            StssBox::from_view(&view)
        }

        /// Checks if a given sample number is a sync sample.
        ///
        /// Note: This performs a binary search since sample numbers are in increasing order.
        pub fn is_sync_sample(&self, sample_number: u32) -> bool {
            self.entries
                .binary_search_by_key(&sample_number, |e| e.sample_number)
                .is_ok()
        }
    }

    impl TryFrom<&StssBoxView<'_>> for StssBox {
        type Error = Error;

        fn try_from(value: &StssBoxView<'_>) -> Result<Self> {
            StssBox::from_view(value)
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

    fn make_stss_payload(sample_numbers: &[u32]) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(0, 0));
        payload.extend_from_slice(&(sample_numbers.len() as u32).to_be_bytes());
        for &sample_number in sample_numbers {
            payload.extend_from_slice(&sample_number.to_be_bytes());
        }
        payload
    }

    #[test]
    fn parse_stss_empty() {
        let payload = make_stss_payload(&[]);
        let stss = StssBoxView::parse(&payload).unwrap();

        assert_eq!(stss.version, 0);
        assert_eq!(stss.entry_count, 0);
        assert_eq!(stss.entries().count(), 0);
    }

    #[test]
    fn parse_stss_single_entry() {
        let payload = make_stss_payload(&[1]);
        let stss = StssBoxView::parse(&payload).unwrap();

        assert_eq!(stss.entry_count, 1);
        let entries: Vec<StssEntry> = stss.entries().map(|r| r.unwrap()).collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].sample_number, 1);
    }

    #[test]
    fn parse_stss_multiple_entries() {
        let sync_samples = vec![1, 30, 60, 90, 120];
        let payload = make_stss_payload(&sync_samples);
        let stss = StssBoxView::parse(&payload).unwrap();

        assert_eq!(stss.entry_count, 5);
        let entries: Vec<StssEntry> = stss.entries().map(|r| r.unwrap()).collect();
        assert_eq!(entries.len(), 5);
        for (i, entry) in entries.iter().enumerate() {
            assert_eq!(entry.sample_number, sync_samples[i]);
        }
    }

    #[test]
    fn parse_stss_is_sync_sample() {
        let sync_samples = vec![1, 30, 60, 90, 120];
        let payload = make_stss_payload(&sync_samples);
        let stss = StssBoxView::parse(&payload).unwrap();

        assert!(stss.is_sync_sample(1).unwrap());
        assert!(stss.is_sync_sample(30).unwrap());
        assert!(stss.is_sync_sample(60).unwrap());
        assert!(!stss.is_sync_sample(2).unwrap());
        assert!(!stss.is_sync_sample(29).unwrap());
        assert!(!stss.is_sync_sample(150).unwrap());
    }

    #[test]
    fn parse_stss_entry_count_mismatch() {
        let mut payload = make_full_box_header(0, 0);
        payload.extend_from_slice(&3u32.to_be_bytes()); // entry_count = 3
        payload.extend_from_slice(&1u32.to_be_bytes()); // Only 1 entry

        let result = StssBoxView::parse(&payload);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::InvalidBoxSize { .. }));
        }
    }

    #[test]
    fn try_from_byte_slice() {
        let sync_samples = vec![1, 50, 100];
        let payload = make_stss_payload(&sync_samples);
        let stss = StssBoxView::try_from(payload.as_slice()).unwrap();

        assert_eq!(stss.entry_count, 3);
    }

    #[test]
    fn try_from_box_view_success() {
        let sync_samples = vec![1, 30, 60];
        let payload = make_stss_payload(&sync_samples);

        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"stss");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxView::parse_in(&mut cursor).unwrap();
        let stss = StssBoxView::try_from(&box_view).unwrap();

        assert_eq!(stss.entry_count, 3);
    }

    #[test]
    fn try_from_box_view_wrong_type() {
        let payload = make_stss_payload(&[1]);

        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"stts"); // Wrong type
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxView::parse_in(&mut cursor).unwrap();
        let result = StssBoxView::try_from(&box_view);

        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::MismatchedBoxType { .. }));
        }
    }

    #[cfg(feature = "alloc")]
    mod alloc_tests {
        use super::*;

        #[test]
        fn stss_box_from_view() {
            let sync_samples = vec![1, 30, 60, 90];
            let payload = make_stss_payload(&sync_samples);
            let view = StssBoxView::parse(&payload).unwrap();
            let stss_box = StssBox::from_view(&view).unwrap();

            assert_eq!(stss_box.entries.len(), 4);
            for (i, entry) in stss_box.entries.iter().enumerate() {
                assert_eq!(entry.sample_number, sync_samples[i]);
            }
        }

        #[test]
        fn stss_box_parse() {
            let sync_samples = vec![1, 50, 100];
            let payload = make_stss_payload(&sync_samples);
            let stss_box = StssBox::parse(&payload).unwrap();

            assert_eq!(stss_box.entries.len(), 3);
            assert_eq!(stss_box.entries[0].sample_number, 1);
            assert_eq!(stss_box.entries[1].sample_number, 50);
            assert_eq!(stss_box.entries[2].sample_number, 100);
        }

        #[test]
        fn stss_box_is_sync_sample() {
            let sync_samples = vec![1, 30, 60, 90, 120];
            let payload = make_stss_payload(&sync_samples);
            let stss_box = StssBox::parse(&payload).unwrap();

            assert!(stss_box.is_sync_sample(1));
            assert!(stss_box.is_sync_sample(30));
            assert!(stss_box.is_sync_sample(60));
            assert!(!stss_box.is_sync_sample(2));
            assert!(!stss_box.is_sync_sample(29));
            assert!(!stss_box.is_sync_sample(150));
        }

        #[test]
        fn stss_box_empty() {
            let payload = make_stss_payload(&[]);
            let stss_box = StssBox::parse(&payload).unwrap();

            assert!(stss_box.entries.is_empty());
            assert!(!stss_box.is_sync_sample(1));
        }

        #[test]
        fn stss_box_try_from() {
            let sync_samples = vec![1, 15, 30];
            let payload = make_stss_payload(&sync_samples);
            let view = StssBoxView::parse(&payload).unwrap();
            let stss_box: StssBox = (&view).try_into().unwrap();

            assert_eq!(stss_box.entries.len(), 3);
            for (i, entry) in stss_box.entries.iter().enumerate() {
                assert_eq!(entry.sample_number, sync_samples[i]);
            }
        }
    }
}
