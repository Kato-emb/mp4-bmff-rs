use crate::cursor::ReadCursor;

use crate::base::header::BoxType;
use crate::codec::BoxCodec;
use crate::codec::BoxDecode;
use crate::error::*;

use super::FullBoxFlags;

/// A single entry in the 64-bit Chunk Offset Box (`co64`).
#[derive(Debug, Clone, Copy)]
pub struct Co64Entry {
    /// The chunk offset.
    pub chunk_offset: u64,
}

/// A reference to a 64-bit Chunk Offset Box (`co64`).
#[derive(Debug)]
pub struct Co64BoxView<'a> {
    /// The version of this box.
    pub version: u8,
    /// The flags of this box.
    pub flags: Co64Flags,
    /// The number of entries in this box.
    pub entry_count: u32,
    entries: &'a [u8],
}

impl<'a> Co64BoxView<'a> {
    const ENTRY_SIZE: usize = 8;

    /// Returns an iterator over the entries in this box.
    pub fn entries(&self) -> impl Iterator<Item = Co64Entry> + 'a {
        let entry_bytes = self.entries;
        let entry_count = self.entry_count as usize;

        entry_bytes
            .chunks_exact(Self::ENTRY_SIZE)
            .take(entry_count)
            .map(|chunk| {
                let chunk_offset = u64::from_be_bytes([
                    chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
                ]);
                Co64Entry { chunk_offset }
            })
    }
}

impl<'a> TryFrom<&'a [u8]> for Co64BoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        Co64BoxView::decode(value)
    }
}

impl BoxCodec for Co64BoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::CO64
    }
}

impl<'de> BoxDecode<'de> for Co64BoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = Co64Flags::from_bytes(cur.read_array()?);

        let entry_count = cur.read_u32_be()?;
        let expected_size = entry_count as usize * Self::ENTRY_SIZE;

        if cur.remaining() != expected_size {
            return Err(Error::at_in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Entries length does not match entry count",
                    got: cur.remaining() as u64,
                },
                cur.position() as u64,
                BoxType::CO64,
            ));
        }

        let entries = cur.take(cur.remaining())?;

        Ok(Co64BoxView {
            version,
            flags,
            entry_count,
            entries,
        })
    }
}

/// Specification for the `co64` box.
pub struct Co64Spec;

/// Type alias for the `co64` box flags.
pub type Co64Flags = FullBoxFlags<Co64Spec>;

#[cfg(feature = "alloc")]
pub use owned::Co64Box;

#[cfg(feature = "alloc")]
mod owned {
    use super::*;

    use crate::BoxEncode;
    use crate::cursor::WriteCursor;
    use crate::lib::Vec;

    /// An owned 64-bit Chunk Offset Box (`co64`).
    #[derive(Debug, Clone)]
    pub struct Co64Box {
        /// The version of this box.
        pub version: u8,
        /// The flags of this box.
        pub flags: Co64Flags,
        /// The entries of this box.
        pub entries: Vec<Co64Entry>,
    }

    impl TryFrom<&Co64BoxView<'_>> for Co64Box {
        type Error = Error;

        fn try_from(view: &Co64BoxView<'_>) -> Result<Self> {
            let mut entries = Vec::with_capacity(view.entry_count as usize);

            for entry in view.entries() {
                entries.push(entry);
            }

            Ok(Co64Box {
                version: view.version,
                flags: view.flags,
                entries,
            })
        }
    }

    impl BoxCodec for Co64Box {
        fn boxtype(&self) -> BoxType {
            BoxType::CO64
        }
    }

    impl BoxDecode<'_> for Co64Box {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = Co64BoxView::decode(bytes)?;
            Co64Box::try_from(&view)
        }
    }

    impl BoxEncode for Co64Box {
        fn encode(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_bytes())?;
            cur.write_u32_be(self.entries.len() as u32)?;

            for entry in &self.entries {
                cur.write_u64_be(entry.chunk_offset)?;
            }

            Ok(cur.position())
        }
    }
}
