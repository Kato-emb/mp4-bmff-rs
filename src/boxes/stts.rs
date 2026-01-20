use crate::cursor::ReadCursor;

use crate::BoxType;
use crate::BoxFrame;
use crate::error::*;
use crate::header::FullBoxFlags;

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
        let version = cur
            .read_u8()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let flags = SttsFlags::from_bytes(
            cur.read_array()
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
        );

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
            version,
            flags,
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

impl<'a> TryFrom<BoxFrame<'a>> for SttsBoxView<'a> {
    type Error = Error;

    fn try_from(value: BoxFrame<'a>) -> Result<Self> {
        if value.boxtype() != BoxType::STTS {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::STTS,
                found: value.boxtype(),
            }));
        }

        SttsBoxView::parse(value.payload())
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

    fn make_full_box_header(version: u8, flags: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(version);
        data.extend_from_slice(&flags.to_be_bytes()[1..4]);
        data
    }

    fn make_stts_payload(entries: Vec<SttsEntry>) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(0, 0));
        payload.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        for entry in entries {
            payload.extend_from_slice(&entry.sample_count.to_be_bytes());
            payload.extend_from_slice(&entry.sample_delta.to_be_bytes());
        }
        payload
    }

    #[test]
    fn parse_and_iterate_entries() {
        // Empty case
        let payload = make_stts_payload(vec![]);
        let stts = SttsBoxView::parse(&payload).unwrap();
        assert_eq!(stts.entry_count, 0);
        assert_eq!(stts.entries().count(), 0);

        // Single entry
        let payload = make_stts_payload(vec![SttsEntry {
            sample_count: 100,
            sample_delta: 1000,
        }]);
        let stts = SttsBoxView::parse(&payload).unwrap();
        assert_eq!(stts.entry_count, 1);
        let entry = stts.entries().next().unwrap().unwrap();
        assert_eq!(entry.sample_count, 100);
        assert_eq!(entry.sample_delta, 1000);

        // Multiple entries
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
        assert_eq!(stts.entry_count, 3);
        let parsed: Vec<_> = stts.entries().map(|r| r.unwrap()).collect();
        assert_eq!(parsed.len(), 3);
        for (i, entry) in parsed.iter().enumerate() {
            assert_eq!(entry.sample_count, entries[i].sample_count);
            assert_eq!(entry.sample_delta, entries[i].sample_delta);
        }
    }

    #[test]
    fn invalid_size() {
        // Entry count mismatch
        let mut payload = make_full_box_header(0, 0);
        payload.extend_from_slice(&2u32.to_be_bytes());
        payload.extend_from_slice(&100u32.to_be_bytes());
        payload.extend_from_slice(&200u32.to_be_bytes());
        assert!(SttsBoxView::parse(&payload).is_err());
    }

    #[test]
    fn wrong_box_type() {
        let payload = make_stts_payload(vec![]);
        let mut box_data = Vec::new();
        box_data.extend_from_slice(&(8 + payload.len() as u32).to_be_bytes());
        box_data.extend_from_slice(b"stsc");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxFrame::parse_in(&mut cursor).unwrap();
        let result = SttsBoxView::try_from(box_view);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().kind(),
            ErrorKind::MismatchedBoxType { .. }
        ));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn owned_conversion() {
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
        let view = SttsBoxView::parse(&payload).unwrap();
        let owned = SttsBox::from_view(&view).unwrap();

        assert_eq!(owned.entries.len(), 2);
        assert_eq!(owned.entries[0].sample_count, 100);
        assert_eq!(owned.entries[1].sample_delta, 2000);
    }
}
