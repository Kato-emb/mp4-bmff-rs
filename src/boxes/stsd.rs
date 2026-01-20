use crate::BoxIter;
use crate::BoxType;
use crate::BoxFrame;
use crate::cursor::ReadCursor;

use crate::FullBoxFlags;
use crate::FullBoxHeader;
use crate::error::*;

use crate::boxes::Avc1BoxView;
use crate::boxes::Mp4aBoxView;

/// An enum representing the different types of entries in a Sample Description Box (`stsd`).
#[derive(Debug)]
pub enum StsdEntryView<'a> {
    /// An Mp4a Box entry.
    Mp4a(Mp4aBoxView<'a>),
    /// An Avc1 Box entry.
    Avc1(Avc1BoxView<'a>),
    /// An unrecognized box entry.
    Other(BoxFrame<'a>),
}

/// An reference to a Sample Description Box (`stsd`).

#[derive(Debug)]
pub struct StsdBoxView<'a> {
    /// The version of this Sample Description Box.
    pub version: u8,
    /// The flags of this Sample Description Box.
    pub flags: StsdFlags,
    /// The number of entries in this Sample Description Box.
    pub entry_count: u32,
    entries: &'a [u8],
}

impl<'a> StsdBoxView<'a> {
    /// Returns an iterator over the entries in the Sample Description Box.
    pub fn entries(&self) -> impl Iterator<Item = Result<StsdEntryView<'a>>> + 'a {
        BoxIter::new(self.entries).map(|box_result| {
            let view = box_result?;
            match view.boxtype() {
                BoxType::AVC1 => Avc1BoxView::parse(view.payload()).map(StsdEntryView::Avc1),
                BoxType::MP4A => Mp4aBoxView::parse(view.payload()).map(StsdEntryView::Mp4a),
                _ => Ok(StsdEntryView::Other(view)),
            }
        })
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> crate::error::Result<Self> {
        let full_box_header = FullBoxHeader::<StsdSpec>::parse_in(cur)?;

        let entry_count = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        let entries = cur.remaining_slice();
        for _ in 0..entry_count {
            BoxFrame::parse_in(cur)?;
        }

        if !cur.is_empty() {
            return Err(Error::at(
                ErrorKind::InvalidBoxSize {
                    reason: "stsd entries size does not match entry_count",
                    got: cur.remaining() as u64,
                },
                cur.position() as u64,
            ));
        }

        Ok(StsdBoxView {
            version: full_box_header.version(),
            flags: full_box_header.flags(),
            entry_count,
            entries,
        })
    }

    /// Parses an `StsdBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(payload);
        let this = StsdBoxView::parse_in(&mut cur)?;

        Ok(this)
    }
}

/// Specification type for Sample Description Box ('stsd')
pub struct StsdSpec;

/// The flags for the Sample Description Box ('stsd')
pub type StsdFlags = FullBoxFlags<StsdSpec>;

#[cfg(feature = "alloc")]
pub use owned::{
    StsdBox, //
    StsdEntry,
};

#[cfg(feature = "alloc")]
mod owned {
    use crate::boxes::Avc1Box;
    use crate::boxes::Mp4aBox;

    use super::*;

    /// An enum representing the different types of entries in a Sample Description Box (`stsd`).
    #[derive(Debug, Clone)]
    pub enum StsdEntry {
        /// An Mp4a Box entry.
        Mp4a(Mp4aBox),
        /// An Avc1 Box entry.
        Avc1(Avc1Box),
        /// An unrecognized box entry.
        Other {
            /// The box type.
            boxtype: BoxType,
            /// The box payload.
            payload: Vec<u8>,
        },
    }

    impl TryFrom<&StsdEntryView<'_>> for StsdEntry {
        type Error = Error;

        fn try_from(value: &StsdEntryView<'_>) -> Result<Self> {
            match value {
                StsdEntryView::Mp4a(view) => Ok(StsdEntry::Mp4a(Mp4aBox::from_view(view)?)),
                StsdEntryView::Avc1(view) => Ok(StsdEntry::Avc1(Avc1Box::from_view(view)?)),
                StsdEntryView::Other(view) => Ok(StsdEntry::Other {
                    boxtype: view.boxtype(),
                    payload: view.payload().to_vec(),
                }),
            }
        }
    }

    /// An owned Sample Description Box (`stsd`).
    #[derive(Debug, Clone)]
    pub struct StsdBox {
        /// The version of this Sample Description Box.
        pub version: u8,
        /// The flags of this Sample Description Box.
        pub flags: StsdFlags,
        /// The number of entries in this Sample Description Box.
        pub entry_count: u32,
        /// The entries in this Sample Description Box.
        pub entries: Vec<StsdEntry>,
    }

    impl StsdBox {
        /// Creates an owned Sample Description Box from a view.
        pub fn from_view(view: &StsdBoxView) -> Result<Self> {
            let mut entries = Vec::with_capacity(view.entry_count as usize);

            for entry_view in view.entries() {
                let entry = entry_view?;
                entries.push(StsdEntry::try_from(&entry)?);
            }

            Ok(StsdBox {
                version: view.version,
                flags: view.flags,
                entry_count: view.entry_count,
                entries,
            })
        }

        /// Parses an owned `StsdBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<Self> {
            let view = StsdBoxView::parse(payload)?;
            Self::from_view(&view)
        }
    }

    impl TryFrom<&StsdBoxView<'_>> for StsdBox {
        type Error = Error;

        fn try_from(value: &StsdBoxView<'_>) -> Result<Self> {
            StsdBox::from_view(value)
        }
    }
}
