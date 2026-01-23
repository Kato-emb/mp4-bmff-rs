use crate::cursor::ReadCursor;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::RawBoxRef;
use crate::error::*;
use crate::iter::BoxIter;

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
    Other(RawBoxRef<'a>),
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
                BoxType::AVC1 => Avc1BoxView::decode(view.into_payload()).map(StsdEntryView::Avc1),
                BoxType::MP4A => Mp4aBoxView::decode(view.into_payload()).map(StsdEntryView::Mp4a),
                _ => Ok(StsdEntryView::Other(view)),
            }
        })
    }
}

/// Specification type for Sample Description Box ('stsd')
pub struct StsdSpec;

/// The flags for the Sample Description Box ('stsd')
pub type StsdFlags = FullBoxFlags<StsdSpec>;

impl BoxCodec for StsdBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::STSD
    }
}

impl<'de> BoxDecode<'de> for StsdBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = StsdFlags::from_bytes(cur.read_array()?);

        let entry_count = cur.read_u32_be()?;

        let entries = cur.remaining_slice();

        let mut count = 0;
        for result in BoxIter::new(entries).take(entry_count as usize) {
            result?;
            count += 1;
        }

        if count < entry_count {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxField {
                    field: "entry_count",
                    reason: "does not match actual number of entries",
                },
                BoxType::STSD,
            ));
        }

        Ok(StsdBoxView {
            version,
            flags,
            entry_count,
            entries,
        })
    }
}

#[cfg(feature = "alloc")]
pub use owned::{
    StsdBox, //
    StsdEntry,
};

#[cfg(feature = "alloc")]
mod owned {
    use crate::BoxEncode;
    use crate::cursor::WriteCursor;

    use crate::codec::boxed_len;
    use crate::codec::write_box_in;

    use super::*;
    use crate::boxes::Avc1Box;
    use crate::boxes::Mp4aBox;

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
                StsdEntryView::Mp4a(view) => Ok(StsdEntry::Mp4a(Mp4aBox::try_from(view)?)),
                StsdEntryView::Avc1(view) => Ok(StsdEntry::Avc1(Avc1Box::try_from(view)?)),
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

    impl TryFrom<&StsdBoxView<'_>> for StsdBox {
        type Error = Error;

        fn try_from(view: &StsdBoxView<'_>) -> Result<Self> {
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
    }

    impl BoxCodec for StsdBox {
        fn boxtype(&self) -> BoxType {
            BoxType::STSD
        }
    }

    impl BoxDecode<'_> for StsdBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = StsdBoxView::decode(bytes)?;
            StsdBox::try_from(&view)
        }
    }

    impl BoxEncode for StsdBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            let mut len = 1 // version
                + 3 // flags
                + 4; // entry_count

            for entry in &self.entries {
                match entry {
                    StsdEntry::Mp4a(box_) => {
                        len += boxed_len(box_);
                    }
                    StsdEntry::Avc1(box_) => {
                        len += boxed_len(box_);
                    }
                    StsdEntry::Other {
                        boxtype: _,
                        payload,
                    } => {
                        // TODO: handle Other box length properly
                        let header = 8; // 4 bytes size + 4 bytes type
                        len += header + payload.len(); // box header + payload
                    }
                }
            }

            len
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_bytes())?;
            cur.write_u32_be(self.entries.len() as u32)?;

            for entry in &self.entries {
                match entry {
                    StsdEntry::Mp4a(box_) => write_box_in(&mut cur, box_)?,
                    StsdEntry::Avc1(box_) => write_box_in(&mut cur, box_)?,
                    StsdEntry::Other {
                        boxtype,
                        payload: _,
                    } => {
                        todo!("Write Other box type {}", boxtype)
                    }
                }
            }

            Ok(cur.position())
        }
    }
}
