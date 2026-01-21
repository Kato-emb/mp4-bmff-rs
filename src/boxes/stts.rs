use crate::cursor::ReadCursor;

use crate::RawBoxRef;
use crate::BoxType;
use crate::error::*;

use super::FullBoxFlags;

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
        let version = cur.read_u8()?;
        let flags = SttsFlags::from_bytes(cur.read_array()?);

        let entry_count = cur.read_u32_be()?;

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

impl<'a> TryFrom<RawBoxRef<'a>> for SttsBoxView<'a> {
    type Error = Error;

    fn try_from(value: RawBoxRef<'a>) -> Result<Self> {
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
    use crate::cursor::WriteCursor;

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
        const ENTRY_SIZE: usize = 8;

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

        /// Returns the size of the `SttsBox` data.
        #[inline]
        pub fn size(&self) -> usize {
            4 + 4 + self.entries.len() * Self::ENTRY_SIZE // version/flags + count + entries
        }

        pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_bytes())?;
            cur.write_u32_be(self.entries.len() as u32)?;

            for entry in &self.entries {
                cur.write_u32_be(entry.sample_count)?;
                cur.write_u32_be(entry.sample_delta)?;
            }

            if !cur.is_empty() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxSize {
                        reason: "Buffer larger than expected",
                        got: cur.remaining() as u64,
                    },
                    BoxType::STTS,
                ));
            }

            Ok(())
        }

        /// Writes this `SttsBox` into the given payload.
        pub fn write(&self, payload: &mut [u8]) -> Result<()> {
            let mut cursor = WriteCursor::new(payload);
            self.write_in(&mut cursor)
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

    #[cfg(feature = "alloc")]
    #[test]
    fn stts_box_write_and_round_trip() {
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
        let view = SttsBoxView::parse(&payload).unwrap();
        let owned = SttsBox::from_view(&view).unwrap();

        // Write to buffer
        let mut buf = vec![0u8; owned.size()];
        owned.write(&mut buf).unwrap();

        // Parse again and compare
        let reparsed = SttsBox::parse(&buf).unwrap();
        assert_eq!(reparsed.entries.len(), 3);
        for (i, entry) in reparsed.entries.iter().enumerate() {
            assert_eq!(entry.sample_count, entries[i].sample_count);
            assert_eq!(entry.sample_delta, entries[i].sample_delta);
        }

        // Error case: buffer too small
        let mut small_buf = vec![0u8; owned.size() - 1];
        assert!(owned.write(&mut small_buf).is_err());

        // Error case: entry count mismatch
        let mut bad_payload = make_full_box_header(0, 0);
        bad_payload.extend_from_slice(&2u32.to_be_bytes());
        bad_payload.extend_from_slice(&100u32.to_be_bytes());
        bad_payload.extend_from_slice(&200u32.to_be_bytes());
        assert!(SttsBoxView::parse(&bad_payload).is_err());
    }
}
