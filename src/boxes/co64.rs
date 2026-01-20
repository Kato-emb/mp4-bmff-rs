use crate::cursor::ReadCursor;

use crate::BoxFrame;
use crate::BoxType;
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

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<Co64BoxView<'a>> {
        let version = cur
            .read_u8()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let flags = Co64Flags::from_bytes(
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

impl<'a> TryFrom<BoxFrame<'a>> for Co64BoxView<'a> {
    type Error = Error;

    fn try_from(value: BoxFrame<'a>) -> Result<Self> {
        if value.boxtype() != BoxType::CO64 {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::CO64,
                found: value.boxtype(),
            }));
        }

        Co64BoxView::parse(value.payload())
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

        /// Returns the size of the payload in bytes.
        pub fn size(&self) -> usize {
            // version(1) + flags(3) + entry_count(4) + entries(8 * n)
            1 + 3 + 4 + self.entries.len() * 8
        }

        pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            cur.write_u8(self.version)
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
            cur.write_array(&self.flags.to_bytes())
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
            cur.write_u32_be(self.entries.len() as u32)
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

            for entry in &self.entries {
                cur.write_u64_be(entry.chunk_offset)
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
            }

            if !cur.is_empty() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxSize {
                        reason: "Buffer larger than expected",
                        got: cur.remaining() as u64,
                    },
                    BoxType::CO64,
                ));
            }

            Ok(())
        }

        /// Writes this `Co64Box` into the given payload.
        pub fn write(&self, payload: &mut [u8]) -> Result<()> {
            let mut cursor = WriteCursor::new(payload);
            self.write_in(&mut cursor)
        }
    }

    impl TryFrom<&Co64BoxView<'_>> for Co64Box {
        type Error = Error;

        fn try_from(value: &Co64BoxView<'_>) -> Result<Self> {
            Co64Box::from_view(value)
        }
    }
}
