use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::cursor::ReadCursor;
use crate::iter::BoxIter;

use crate::error::*;

use crate::boxes::AudioSampleEntry;
use crate::boxes::EsdsBoxView;

/// A reference to an Mp4a Box's contents.
#[derive(Debug)]
pub struct Mp4aBoxView<'a> {
    base: AudioSampleEntry,
    payload: &'a [u8],
}

impl<'a> Mp4aBoxView<'a> {
    /// Returns the base Audio Sample Entry.
    pub fn audio_sample_entry(&self) -> &AudioSampleEntry {
        &self.base
    }

    /// Returns an iterator over the child boxes of this Mp4a box.
    pub fn children(&self) -> BoxIter<'a> {
        BoxIter::new(self.payload)
    }

    /// Returns the ESDS box contained in this Mp4a box.
    pub fn esds(&self) -> Result<EsdsBoxView<'a>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::ESDS {
                return EsdsBoxView::decode(child.into_payload());
            }
        }

        Err(Error::in_box(
            ErrorKind::BoxMissing {
                required: BoxType::ESDS,
            },
            BoxType::MP4A,
        ))
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
        let payload = cur.remaining_slice();

        Ok(Mp4aBoxView { base, payload })
    }
}

#[cfg(feature = "alloc")]
pub use owned::Mp4aBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::BoxEncode;
    use crate::cursor::WriteCursor;

    use crate::codec::boxed_len;
    use crate::codec::write_box_in;

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
            let esds = view.esds()?;

            Ok(Mp4aBox {
                base: view.base,
                esds: EsdsBox::try_from(&esds)?,
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
        fn encoded_len(&self) -> usize {
            let base_len = AudioSampleEntry::size();
            let esds_len = boxed_len(&self.esds);
            base_len + esds_len
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            self.base.write_in(&mut cur)?;
            write_box_in(&mut cur, &self.esds)?;

            Ok(cur.position())
        }
    }
}
