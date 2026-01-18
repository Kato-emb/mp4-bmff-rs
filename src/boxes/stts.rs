use crate::cursor::ReadCursor;

use crate::BoxType;
use crate::BoxView;
use crate::error::*;
use crate::header::FullBoxFlags;
use crate::header::FullBoxHeader;

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
    /// Returns an iterator over the entries in the Decoding Time to Sample Box (`stts`).
    pub fn entries(&self) -> impl Iterator<Item = Result<SttsEntry>> + 'a {
        let entry_bytes = self.entries;
        let entry_count = self.entry_count as usize;

        entry_bytes.chunks_exact(8).take(entry_count).map(|chunk| {
            if chunk.len() != 8 {
                return Err(Error::new(ErrorKind::NotEnoughBytes {
                    expected: 8,
                    remaining: chunk.len(),
                }));
            }

            let sample_count = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            let sample_delta = u32::from_be_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);

            Ok(SttsEntry {
                sample_count,
                sample_delta,
            })
        })
    }

    /// Parses a `SttsBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<SttsBoxView<'a>> {
        let mut cur = ReadCursor::new(payload);

        let full_box_header = FullBoxHeader::<SttsSpec>::parse(&mut cur)?;

        let entry_count = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        let entries = cur.remaining_slice();
        if !cur.remaining().is_multiple_of(8) {
            return Err(Error::at_in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Entries length is not a multiple of 8",
                    got: cur.remaining() as u64,
                },
                cur.position() as u64,
                BoxType::STTS,
            ));
        }

        Ok(SttsBoxView {
            version: full_box_header.version(),
            flags: full_box_header.flags(),
            entry_count,
            entries,
        })
    }
}

impl<'a> TryFrom<&'a [u8]> for SttsBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        SttsBoxView::parse(value)
    }
}

impl<'a> TryFrom<BoxView<'a>> for SttsBoxView<'a> {
    type Error = Error;

    fn try_from(value: BoxView<'a>) -> std::result::Result<Self, Self::Error> {
        if value.header.boxtype() != BoxType::STTS {
            return Err(Error::new(ErrorKind::MissmatchedBoxType {
                expected: BoxType::STTS,
                found: value.header.boxtype(),
            }));
        }

        SttsBoxView::parse(value.payload)
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
