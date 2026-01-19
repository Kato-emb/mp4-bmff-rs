use crate::cursor::ReadCursor;

use crate::BoxType;
use crate::BoxView;
use crate::error::*;
use crate::header::FullBoxFlags;
use crate::header::FullBoxHeader;

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

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<Co64BoxView<'a>> {
        let full_box_header = FullBoxHeader::<Co64Spec>::parse_in(cur)?;

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
                BoxType::CO64,
            ));
        }

        let entries = cur.take(cur.remaining())?;

        Ok(Co64BoxView {
            version: full_box_header.version(),
            flags: full_box_header.flags(),
            entry_count,
            entries,
        })
    }

    /// Parses a `Co64BoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<Co64BoxView<'a>> {
        let mut cursor = ReadCursor::new(payload);
        let this = Co64BoxView::parse_in(&mut cursor)?;

        Ok(this)
    }
}

impl<'a> TryFrom<&'a [u8]> for Co64BoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        Co64BoxView::parse(value)
    }
}

impl<'a> TryFrom<BoxView<'a>> for Co64BoxView<'a> {
    type Error = Error;

    fn try_from(value: BoxView<'a>) -> Result<Self> {
        if value.header.boxtype() != BoxType::CO64 {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::CO64,
                found: value.header.boxtype(),
            }));
        }

        Co64BoxView::parse(value.payload)
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

    impl Co64Box {
        /// Creates a `Co64Box` from a `Co64BoxView`.
        pub fn from_view(view: &Co64BoxView<'_>) -> Result<Co64Box> {
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

        /// Parses a `Co64Box` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<Co64Box> {
            let view = Co64BoxView::parse(payload)?;
            Co64Box::from_view(&view)
        }
    }

    impl TryFrom<&Co64BoxView<'_>> for Co64Box {
        type Error = Error;

        fn try_from(value: &Co64BoxView<'_>) -> Result<Self> {
            Co64Box::from_view(value)
        }
    }
}
