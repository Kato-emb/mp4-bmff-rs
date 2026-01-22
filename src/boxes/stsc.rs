use crate::cursor::ReadCursor;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;

use super::FullBoxFlags;

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
}

/// Specification for the Sample To Chunk Box (`stsc`).
pub struct StscSpec;

/// Flags for the Sample To Chunk Box (`stsc`).
pub type StscFlags = FullBoxFlags<StscSpec>;

impl<'a> TryFrom<&'a [u8]> for StscBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        StscBoxView::decode(value)
    }
}

impl BoxCodec for StscBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::STSC
    }
}

impl<'de> BoxDecode<'de> for StscBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = StscFlags::from_bytes(cur.read_array()?);

        let entry_count = cur.read_u32_be()?;

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
            version,
            flags,
            entry_count,
            entries,
        })
    }
}

#[cfg(feature = "alloc")]
pub use owned::StscBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::BoxEncode;
    use crate::cursor::WriteCursor;

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

    impl TryFrom<&StscBoxView<'_>> for StscBox {
        type Error = Error;

        fn try_from(view: &StscBoxView<'_>) -> Result<Self> {
            let entries = view.entries().collect::<Result<Vec<StscEntry>>>()?;

            Ok(StscBox {
                version: view.version,
                flags: view.flags,
                entries,
            })
        }
    }

    impl BoxCodec for StscBox {
        fn boxtype(&self) -> BoxType {
            BoxType::STSC
        }
    }

    impl BoxDecode<'_> for StscBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = StscBoxView::decode(bytes)?;
            StscBox::try_from(&view)
        }
    }

    impl BoxEncode for StscBox {
        fn encode(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_bytes())?;
            cur.write_u32_be(self.entries.len() as u32)?;

            for entry in &self.entries {
                cur.write_u32_be(entry.first_chunk)?;
                cur.write_u32_be(entry.samples_per_chunk)?;
                cur.write_u32_be(entry.sample_description_index)?;
            }

            Ok(cur.position())
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

    #[cfg(feature = "alloc")]
    #[test]
    fn stsc_box_write_and_round_trip() {
        // Multiple entries

        use crate::BoxEncode;
        let entries = vec![
            StscEntry {
                first_chunk: 1,
                samples_per_chunk: 10,
                sample_description_index: 1,
            },
            StscEntry {
                first_chunk: 5,
                samples_per_chunk: 20,
                sample_description_index: 2,
            },
            StscEntry {
                first_chunk: 10,
                samples_per_chunk: 15,
                sample_description_index: 1,
            },
        ];
        let payload = make_stsc_payload(entries.clone());
        let view = StscBoxView::decode(&payload).unwrap();
        let owned = StscBox::try_from(&view).unwrap();

        // Write to buffer
        let mut buf = vec![0u8; 256];
        let written = owned.encode(&mut buf).unwrap();

        // Parse again and compare
        let reparsed = StscBox::decode(&buf[..written]).unwrap();
        assert_eq!(reparsed.entries.len(), 3);
        for (i, entry) in reparsed.entries.iter().enumerate() {
            assert_eq!(entry.first_chunk, entries[i].first_chunk);
            assert_eq!(entry.samples_per_chunk, entries[i].samples_per_chunk);
            assert_eq!(
                entry.sample_description_index,
                entries[i].sample_description_index
            );
        }

        // Error case: buffer too small
        let mut small_buf = vec![0u8; 10];
        assert!(owned.encode(&mut small_buf).is_err());

        // Error case: entry count mismatch
        let mut bad_payload = make_full_box_header(0, 0);
        bad_payload.extend_from_slice(&2u32.to_be_bytes());
        bad_payload.extend_from_slice(&1u32.to_be_bytes());
        bad_payload.extend_from_slice(&10u32.to_be_bytes());
        bad_payload.extend_from_slice(&1u32.to_be_bytes());
        assert!(StscBoxView::decode(&bad_payload).is_err());
    }
}
