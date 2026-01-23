use crate::cursor::ReadCursor;

use crate::base::header::BoxType;
use crate::codec::BoxCodec;
use crate::codec::BoxDecode;
use crate::error::*;
use crate::iter::FixedSizeEntry;
use crate::iter::FixedSizeEntryIter;

use super::FullBoxFlags;

/// A single entry in the 64-bit Chunk Offset Box (`co64`).
#[derive(Debug, Clone, Copy)]
pub struct Co64Entry {
    /// The chunk offset.
    pub chunk_offset: u64,
}

impl FixedSizeEntry for Co64Entry {
    const ENTRY_SIZE: usize = 8;

    fn from_bytes(bytes: &[u8]) -> Self {
        debug_assert_eq!(bytes.len(), Self::ENTRY_SIZE);

        Co64Entry {
            chunk_offset: u64::from_be_bytes(bytes.try_into().unwrap()),
        }
    }

    fn to_bytes(&self, bytes: &mut [u8]) {
        debug_assert_eq!(bytes.len(), Self::ENTRY_SIZE);
        bytes.copy_from_slice(&self.chunk_offset.to_be_bytes());
    }
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
    /// Returns an iterator over the entries in this box.
    pub fn entries(&self) -> FixedSizeEntryIter<'a, Co64Entry> {
        FixedSizeEntryIter::new(self.entries)
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
        let expected_size = entry_count as usize * Co64Entry::ENTRY_SIZE;

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
    #[derive(Debug, Clone, Default)]
    pub struct Co64Box {
        /// The version of this box.
        pub version: u8,
        /// The flags of this box.
        pub flags: Co64Flags,
        /// The entries of this box.
        pub entries: Vec<Co64Entry>,
    }

    impl From<&Co64BoxView<'_>> for Co64Box {
        fn from(view: &Co64BoxView<'_>) -> Self {
            let entries = view.entries().collect::<Vec<_>>();

            Co64Box {
                version: view.version,
                flags: view.flags,
                entries,
            }
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
            Ok(Co64Box::from(&view))
        }
    }

    impl BoxEncode for Co64Box {
        fn encoded_len(&self) -> usize {
            1 // version
            + 3 // flags
            + 4 // entry count
            + (self.entries.len() * Co64Entry::ENTRY_SIZE) // entries
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
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
