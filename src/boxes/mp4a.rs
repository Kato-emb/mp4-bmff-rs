use crate::BoxType;
use crate::BoxFrame;
use crate::cursor::ReadCursor;

use crate::error::*;

use crate::boxes::AudioSampleEntry;
use crate::boxes::EsdsBoxView;

/// A reference to an Mp4a Box's contents.
#[derive(Debug)]
pub struct Mp4aBoxView<'a> {
    base: AudioSampleEntry,
    /// The ESDS box contained in this Mp4a box.
    pub esds: EsdsBoxView<'a>,
}

impl<'a> Mp4aBoxView<'a> {
    /// Returns the base Audio Sample Entry.
    pub fn audio_sample_entry(&self) -> &AudioSampleEntry {
        &self.base
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<Self> {
        let base = AudioSampleEntry::parse_in(cur)?;
        let frame = BoxFrame::parse_in(cur)?;
        if frame.boxtype() != BoxType::ESDS {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxType {
                    reason: "Expected ESDS box in Mp4a box",
                    got: frame.boxtype().type_field(),
                },
                BoxType::MP4A,
            ));
        }

        let esds = EsdsBoxView::parse(frame.payload())?;
        Ok(Mp4aBoxView { base, esds })
    }

    /// Parses an `Mp4aBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(payload);
        let this = Mp4aBoxView::parse_in(&mut cur)?;

        Ok(this)
    }
}

#[cfg(feature = "alloc")]
pub use owned::Mp4aBox;

#[cfg(feature = "alloc")]
mod owned {
    use super::*;
    use crate::boxes::EsdsBox;

    /// An owned Mp4a Box (`mp4a`).
    #[derive(Debug, Clone)]
    pub struct Mp4aBox {
        /// The base Audio Sample Entry.
        pub base: AudioSampleEntry,
        /// The ESDS box contained in this Mp4a box.
        pub esds: EsdsBox,
    }

    impl Mp4aBox {
        /// Creates an `Mp4aBox` from an `Mp4aBoxView`.
        pub fn from_view(view: &Mp4aBoxView<'_>) -> Result<Self> {
            Ok(Mp4aBox {
                base: view.base,
                esds: EsdsBox::from_view(&view.esds)?,
            })
        }
    }

    impl TryFrom<&Mp4aBoxView<'_>> for Mp4aBox {
        type Error = Error;

        fn try_from(value: &Mp4aBoxView<'_>) -> Result<Self> {
            Mp4aBox::from_view(value)
        }
    }
}
