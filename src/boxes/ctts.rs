use crate::cursor::ReadCursor;

use crate::BoxType;
use crate::BoxFrame;
use crate::error::*;
use crate::header::FullBoxFlags;

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

        entry_bytes
            .chunks_exact(Self::ENTRY_SIZE)
            .take(entry_count)
            .map(move |chunk| {
                let sample_count = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);

                // Version 0: unsigned 32-bit offset
                // Version 1: signed 32-bit offset
                let sample_offset = if version == 0 {
                    let offset = u32::from_be_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
                    offset as i32
                } else {
                    i32::from_be_bytes([chunk[4], chunk[5], chunk[6], chunk[7]])
                };

                Ok(CttsEntry {
                    sample_count,
                    sample_offset,
                })
            })
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<CttsBoxView<'a>> {
        let version = cur
            .read_u8()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let flags = CttsFlags::from_bytes(
            cur.read_array()
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
        );

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
            version,
            flags,
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

impl<'a> TryFrom<BoxFrame<'a>> for CttsBoxView<'a> {
    type Error = Error;

    fn try_from(value: BoxFrame<'a>) -> Result<Self> {
        if value.boxtype() != BoxType::CTTS {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::CTTS,
                found: value.boxtype(),
            }));
        }

        CttsBoxView::parse(value.payload())
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

    fn make_full_box_header(version: u8, flags: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(version);
        data.extend_from_slice(&flags.to_be_bytes()[1..4]);
        data
    }

    fn make_ctts_payload_v0(entries: Vec<CttsEntry>) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(0, 0));
        payload.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        for entry in entries {
            payload.extend_from_slice(&entry.sample_count.to_be_bytes());
            payload.extend_from_slice(&(entry.sample_offset as u32).to_be_bytes());
        }
        payload
    }

    fn make_ctts_payload_v1(entries: Vec<CttsEntry>) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(1, 0));
        payload.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        for entry in entries {
            payload.extend_from_slice(&entry.sample_count.to_be_bytes());
            payload.extend_from_slice(&entry.sample_offset.to_be_bytes());
        }
        payload
    }

    #[test]
    fn parse_and_iterate_entries() {
        // Empty v0
        let payload = make_ctts_payload_v0(vec![]);
        let ctts = CttsBoxView::parse(&payload).unwrap();
        assert_eq!(ctts.version, 0);
        assert_eq!(ctts.entries().count(), 0);

        // v0 with entries
        let entries = vec![
            CttsEntry { sample_count: 10, sample_offset: 100 },
            CttsEntry { sample_count: 20, sample_offset: 200 },
        ];
        let payload = make_ctts_payload_v0(entries.clone());
        let ctts = CttsBoxView::parse(&payload).unwrap();
        let parsed: Vec<_> = ctts.entries().map(|r| r.unwrap()).collect();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].sample_offset, 100);

        // v1 with negative offset
        let entries = vec![CttsEntry { sample_count: 5, sample_offset: -50 }];
        let payload = make_ctts_payload_v1(entries);
        let ctts = CttsBoxView::parse(&payload).unwrap();
        assert_eq!(ctts.version, 1);
        let entry = ctts.entries().next().unwrap().unwrap();
        assert_eq!(entry.sample_offset, -50);
    }

    #[test]
    fn invalid_size() {
        let mut payload = make_full_box_header(0, 0);
        payload.extend_from_slice(&2u32.to_be_bytes());
        payload.extend_from_slice(&10u32.to_be_bytes());
        payload.extend_from_slice(&100u32.to_be_bytes());
        assert!(CttsBoxView::parse(&payload).is_err());
    }

    #[test]
    fn wrong_box_type() {
        let payload = make_ctts_payload_v0(vec![]);
        let mut box_data = Vec::new();
        box_data.extend_from_slice(&(8 + payload.len() as u32).to_be_bytes());
        box_data.extend_from_slice(b"stts");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxFrame::parse_in(&mut cursor).unwrap();
        let result = CttsBoxView::try_from(box_view);
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn owned_conversion() {
        let entries = vec![
            CttsEntry { sample_count: 10, sample_offset: 100 },
            CttsEntry { sample_count: 20, sample_offset: -50 },
        ];
        let payload = make_ctts_payload_v1(entries.clone());
        let view = CttsBoxView::parse(&payload).unwrap();
        let owned = CttsBox::from_view(&view).unwrap();
        assert_eq!(owned.entries.len(), 2);
        assert_eq!(owned.entries[0].sample_offset, 100);
        assert_eq!(owned.entries[1].sample_offset, -50);
    }
}
