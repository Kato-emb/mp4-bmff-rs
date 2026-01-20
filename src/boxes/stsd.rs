use crate::cursor::ReadCursor;

use crate::BoxFrame;
use crate::BoxIter;
use crate::BoxType;
use crate::error::*;

use super::FullBoxFlags;

use super::Avc1BoxView;
use super::Mp4aBoxView;

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
        let version = cur
            .read_u8()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let flags = StsdFlags::from_bytes(
            cur.read_array()
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
        );

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
            version,
            flags,
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
    use crate::BoxFrameMut;
    use crate::boxes::Avc1Box;
    use crate::boxes::Mp4aBox;
    use crate::cursor::WriteCursor;

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

    impl StsdEntry {
        /// Returns the box type for this entry.
        pub fn boxtype(&self) -> BoxType {
            match self {
                StsdEntry::Mp4a(_) => BoxType::MP4A,
                StsdEntry::Avc1(_) => BoxType::AVC1,
                StsdEntry::Other { boxtype, .. } => *boxtype,
            }
        }

        /// Returns the size of the payload in bytes (not including box header).
        pub fn payload_size(&self) -> usize {
            match self {
                StsdEntry::Mp4a(box_) => box_.size(),
                StsdEntry::Avc1(box_) => box_.size(),
                StsdEntry::Other { payload, .. } => payload.len(),
            }
        }

        /// Returns the total frame size (including box header).
        pub fn frame_size(&self) -> usize {
            BoxFrameMut::required_len(self.boxtype(), self.payload_size())
        }

        pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            let boxtype = self.boxtype();
            let payload_size = self.payload_size();
            let frame_size = self.frame_size();

            let buf = cur.take_mut(frame_size)?;
            let mut frame = BoxFrameMut::new(buf, boxtype, payload_size)?;

            match self {
                StsdEntry::Mp4a(box_) => box_.write(frame.payload_mut())?,
                StsdEntry::Avc1(box_) => box_.write(frame.payload_mut())?,
                StsdEntry::Other { payload, .. } => {
                    frame.payload_mut().copy_from_slice(payload);
                }
            }

            Ok(())
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

        /// Returns the size of the payload in bytes.
        pub fn size(&self) -> usize {
            // version(1) + flags(3) + entry_count(4) + entries
            let entries_size: usize = self.entries.iter().map(|e| e.frame_size()).sum();
            1 + 3 + 4 + entries_size
        }

        pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            cur.write_u8(self.version)
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
            cur.write_array(&self.flags.to_bytes())
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
            cur.write_u32_be(self.entries.len() as u32)
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

            for entry in &self.entries {
                entry.write_in(cur)?;
            }

            if !cur.is_empty() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxSize {
                        reason: "Buffer larger than expected",
                        got: cur.remaining() as u64,
                    },
                    BoxType::STSD,
                ));
            }

            Ok(())
        }

        /// Writes this `StsdBox` into the given payload.
        pub fn write(&self, payload: &mut [u8]) -> Result<()> {
            let mut cursor = WriteCursor::new(payload);
            self.write_in(&mut cursor)
        }
    }

    impl TryFrom<&StsdBoxView<'_>> for StsdBox {
        type Error = Error;

        fn try_from(value: &StsdBoxView<'_>) -> Result<Self> {
            StsdBox::from_view(value)
        }
    }
}
