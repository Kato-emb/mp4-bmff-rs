use crate::cursor::ReadCursor;

use crate::BoxType;
use crate::BoxFrame;
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
                let sample_number = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
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

impl<'a> TryFrom<BoxFrame<'a>> for StssBoxView<'a> {
    type Error = Error;

    fn try_from(value: BoxFrame<'a>) -> Result<Self> {
        if value.boxtype() != BoxType::STSS {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::STSS,
                found: value.boxtype(),
            }));
        }

        StssBoxView::parse(value.payload())
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

    fn make_stss_payload(sample_numbers: Vec<u32>) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(0, 0));
        payload.extend_from_slice(&(sample_numbers.len() as u32).to_be_bytes());
        for sample_number in sample_numbers {
            payload.extend_from_slice(&sample_number.to_be_bytes());
        }
        payload
    }

    #[test]
    fn parse_and_iterate_samples() {
        // Empty
        let payload = make_stss_payload(vec![]);
        let stss = StssBoxView::parse(&payload).unwrap();
        assert_eq!(stss.entry_count, 0);
        assert_eq!(stss.entries().count(), 0);

        // Single
        let payload = make_stss_payload(vec![10]);
        let stss = StssBoxView::parse(&payload).unwrap();
        let entry = stss.entries().next().unwrap().unwrap();
        assert_eq!(entry.sample_number, 10);

        // Multiple
        let samples = vec![1, 5, 10, 15];
        let payload = make_stss_payload(samples.clone());
        let stss = StssBoxView::parse(&payload).unwrap();
        let parsed: Vec<_> = stss.entries().map(|r| r.unwrap().sample_number).collect();
        assert_eq!(parsed, samples);
    }

    #[test]
    fn invalid_size() {
        let mut payload = make_full_box_header(0, 0);
        payload.extend_from_slice(&2u32.to_be_bytes());
        payload.extend_from_slice(&1u32.to_be_bytes());
        assert!(StssBoxView::parse(&payload).is_err());
    }

    #[test]
    fn wrong_box_type() {
        let payload = make_stss_payload(vec![]);
        let mut box_data = Vec::new();
        box_data.extend_from_slice(&(8 + payload.len() as u32).to_be_bytes());
        box_data.extend_from_slice(b"stsc");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxFrame::parse_in(&mut cursor).unwrap();
        let result = StssBoxView::try_from(box_view);
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn owned_conversion() {
        let samples = vec![1, 5, 10];
        let payload = make_stss_payload(samples.clone());
        let view = StssBoxView::parse(&payload).unwrap();
        let owned = StssBox::from_view(&view).unwrap();
        assert_eq!(owned.entries.len(), 3);
        assert_eq!(owned.entries[0].sample_number, 1);
        assert!(owned.is_sync_sample(1));
        assert!(!owned.is_sync_sample(2));
    }
}
