use crate::cursor::ReadCursor;

use crate::BoxFrame;
use crate::BoxType;
use crate::error::*;

use super::FullBoxFlags;

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
    const ENTRY_SIZE: usize = 4;

    /// Returns an iterator over the entries in the Chunk Offset Box.
    pub fn entries(&self) -> impl Iterator<Item = StcoEntry> + 'a {
        let entry_bytes = self.entries;
        let entry_count = self.entry_count as usize;

        entry_bytes.chunks_exact(4).take(entry_count).map(|chunk| {
            let chunk_offset = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            StcoEntry { chunk_offset }
        })
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<StcoBoxView<'a>> {
        let version = cur
            .read_u8()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let flags = StcoFlags::from_bytes(
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
                BoxType::STCO,
            ));
        }

        let entries = cur.take(cur.remaining())?;

        Ok(StcoBoxView {
            version,
            flags,
            entry_count,
            entries,
        })
    }

    /// Parses a `StcoBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<StcoBoxView<'a>> {
        let mut cursor = ReadCursor::new(payload);
        let this = StcoBoxView::parse_in(&mut cursor)?;

        Ok(this)
    }
}

impl<'a> TryFrom<&'a [u8]> for StcoBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        StcoBoxView::parse(value)
    }
}

impl<'a> TryFrom<BoxFrame<'a>> for StcoBoxView<'a> {
    type Error = Error;

    fn try_from(value: BoxFrame<'a>) -> Result<Self> {
        if value.boxtype() != BoxType::STCO {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::STCO,
                found: value.boxtype(),
            }));
        }

        StcoBoxView::parse(value.payload())
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
    #[derive(Debug, Clone)]
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
            let mut entries = Vec::with_capacity(view.entry_count as usize);

            for entry in view.entries() {
                entries.push(entry);
            }

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

    fn make_full_box_header(version: u8, flags: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(version);
        data.extend_from_slice(&flags.to_be_bytes()[1..4]);
        data
    }

    fn make_stco_payload(offsets: Vec<u32>) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(0, 0));
        payload.extend_from_slice(&(offsets.len() as u32).to_be_bytes());
        for offset in offsets {
            payload.extend_from_slice(&offset.to_be_bytes());
        }
        payload
    }

    #[test]
    fn parse_and_iterate_offsets() {
        // Empty
        let payload = make_stco_payload(vec![]);
        let stco = StcoBoxView::parse(&payload).unwrap();
        assert_eq!(stco.entry_count, 0);
        assert_eq!(stco.entries().count(), 0);

        // Single
        let payload = make_stco_payload(vec![1000]);
        let stco = StcoBoxView::parse(&payload).unwrap();
        assert_eq!(stco.entry_count, 1);
        assert_eq!(stco.entries().next().unwrap().chunk_offset, 1000);

        // Multiple
        let offsets = vec![100, 200, 300, 400];
        let payload = make_stco_payload(offsets.clone());
        let stco = StcoBoxView::parse(&payload).unwrap();
        let parsed: Vec<_> = stco.entries().map(|e| e.chunk_offset).collect();
        assert_eq!(parsed, offsets);
    }

    #[test]
    fn invalid_size() {
        let mut payload = make_full_box_header(0, 0);
        payload.extend_from_slice(&2u32.to_be_bytes());
        payload.extend_from_slice(&100u32.to_be_bytes());
        assert!(StcoBoxView::parse(&payload).is_err());
    }

    #[test]
    fn wrong_box_type() {
        let payload = make_stco_payload(vec![]);
        let mut box_data = Vec::new();
        box_data.extend_from_slice(&(8 + payload.len() as u32).to_be_bytes());
        box_data.extend_from_slice(b"stsc");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxFrame::parse_in(&mut cursor).unwrap();
        let result = StcoBoxView::try_from(box_view);
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn owned_conversion() {
        let offsets = vec![100, 200, 300];
        let payload = make_stco_payload(offsets.clone());
        let view = StcoBoxView::parse(&payload).unwrap();
        let owned = StcoBox::from_view(&view).unwrap();
        assert_eq!(owned.entries.len(), 3);
        assert_eq!(owned.entries[0].chunk_offset, 100);
    }
}
