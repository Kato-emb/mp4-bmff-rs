use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

use super::esds::EsdsBoxView;
use crate::boxes::sample_entry::VisualSampleEntry;

/// A reference to an MP4 Visual Sample Entry
#[derive(Debug)]
pub struct Mp4vSampleEntryView<'a> {
    base: VisualSampleEntry,
    content: &'a [u8],
}

impl<'a> Mp4vSampleEntryView<'a> {
    /// Returns the base Visual Sample Entry.
    pub fn base(&self) -> &VisualSampleEntry {
        &self.base
    }

    /// Returns an iterator over the child boxes of this Mp4v Sample Entry.
    pub fn boxes(&self) -> BoxIter<'a> {
        BoxIter::new(self.content)
    }

    /// Returns the ESDS box contained in this Mp4v Sample Entry.
    pub fn esds(&self) -> Result<EsdsBoxView<'a>> {
        for result in self.boxes() {
            let rawbox = result?;
            if rawbox.boxtype() == BoxType::ESDS {
                return EsdsBoxView::decode(rawbox.into_payload());
            }
        }

        Err(Error::in_box(
            ErrorKind::BoxMissing {
                required: BoxType::ESDS,
            },
            BoxType::MP4V,
        ))
    }
}

impl BoxCodec for Mp4vSampleEntryView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::MP4V
    }
}

impl<'de> BoxDecode<'de> for Mp4vSampleEntryView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = crate::cursor::ReadCursor::new(bytes);

        let base = VisualSampleEntry::parse_in(&mut cur)?;
        let content = cur.take(cur.remaining())?;

        Ok(Mp4vSampleEntryView { base, content })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use super::*;
    use crate::BoxEncode;

    use crate::codec::boxed_len;
    use crate::codec::write_box_in;
    use crate::cursor::WriteCursor;

    use crate::boxes::mp4::EsdsBox;

    /// An owned MP4V Sample Entry
    #[derive(Debug, Clone)]
    pub struct Mp4vSampleEntry {
        /// The base Visual Sample Entry
        pub base: VisualSampleEntry,
        /// The ESDS box contained in this Mp4v Sample Entry.
        pub esds: EsdsBox,
    }

    impl TryFrom<&Mp4vSampleEntryView<'_>> for Mp4vSampleEntry {
        type Error = Error;

        fn try_from(view: &Mp4vSampleEntryView<'_>) -> Result<Self> {
            let mut esds = None;

            for result in view.boxes() {
                let rawbox = result?;

                match rawbox.boxtype() {
                    BoxType::ESDS => {
                        if esds.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::ESDS,
                                },
                                BoxType::MP4V,
                            ));
                        }
                        esds = Some(EsdsBox::decode(rawbox.into_payload())?);
                    }
                    _ => continue,
                }
            }

            let esds = esds.ok_or_else(|| {
                Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::ESDS,
                    },
                    BoxType::MP4V,
                )
            })?;

            Ok(Mp4vSampleEntry {
                base: view.base,
                esds,
            })
        }
    }

    impl BoxCodec for Mp4vSampleEntry {
        fn boxtype(&self) -> BoxType {
            BoxType::MP4V
        }
    }

    impl BoxDecode<'_> for Mp4vSampleEntry {
        fn decode(bytes: &'_ [u8]) -> Result<Self> {
            let view = Mp4vSampleEntryView::decode(bytes)?;
            Mp4vSampleEntry::try_from(&view)
        }
    }

    impl BoxEncode for Mp4vSampleEntry {
        fn encoded_len(&self) -> usize {
            VisualSampleEntry::size() + boxed_len(&self.esds)
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            self.base.write_in(&mut cur)?;
            write_box_in(&mut cur, &self.esds)?;

            Ok(cur.position())
        }
    }
}

#[cfg(feature = "alloc")]
pub use owned::*;
