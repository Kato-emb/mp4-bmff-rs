use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::RawBoxRef;
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
        let frame = RawBoxRef::parse_in(cur)?;
        if frame.boxtype() != BoxType::ESDS {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxType {
                    reason: "Expected ESDS box in Mp4a box",
                    got: frame.boxtype().type_field(),
                },
                BoxType::MP4A,
            ));
        }

        let esds = EsdsBoxView::decode(frame.into_payload())?;
        Ok(Mp4aBoxView { base, esds })
    }

    /// Parses an `Mp4aBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(payload);
        let this = Mp4aBoxView::parse_in(&mut cur)?;

        Ok(this)
    }
}

impl BoxCodec for Mp4aBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::MP4A
    }
}

impl<'de> BoxDecode<'de> for Mp4aBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let base = AudioSampleEntry::parse_in(&mut cur)?;
        let frame = RawBoxRef::parse_in(&mut cur)?;
        if frame.boxtype() != BoxType::ESDS {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxType {
                    reason: "Expected ESDS box in Mp4a box",
                    got: frame.boxtype().type_field(),
                },
                BoxType::MP4A,
            ));
        }

        let esds = EsdsBoxView::decode(frame.into_payload())?;
        Ok(Mp4aBoxView { base, esds })
    }
}

#[cfg(feature = "alloc")]
pub use owned::Mp4aBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::BoxEncode;
    use crate::cursor::WriteCursor;

    use crate::base::writer::write_box_in;

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

    impl TryFrom<&Mp4aBoxView<'_>> for Mp4aBox {
        type Error = Error;

        fn try_from(view: &Mp4aBoxView<'_>) -> Result<Self> {
            Ok(Mp4aBox {
                base: view.base,
                esds: EsdsBox::try_from(&view.esds)?,
            })
        }
    }

    impl BoxCodec for Mp4aBox {
        fn boxtype(&self) -> BoxType {
            BoxType::MP4A
        }
    }

    impl BoxDecode<'_> for Mp4aBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = Mp4aBoxView::decode(bytes)?;
            Mp4aBox::try_from(&view)
        }
    }

    impl BoxEncode for Mp4aBox {
        fn encode(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            self.base.write_in(&mut cur)?;
            write_box_in(&mut cur, &self.esds)?;

            Ok(cur.position())
        }
    }
}
