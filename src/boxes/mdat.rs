use crate::cursor::ReadCursor;

use crate::BoxType;
use crate::error::*;

/// A reference to a Media Data Box (`mdat`).
pub struct MdatBoxView<'a> {
    /// The raw data of the Media Data Box (`mdat`).
    pub data: &'a [u8],
}

impl<'a> MdatBoxView<'a> {
    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<MdatBoxView<'a>> {
        let data = cur.take(cur.remaining())?;
        Ok(MdatBoxView { data })
    }

    /// Parses an `MdatBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<MdatBoxView<'a>> {
        let mut cursor = ReadCursor::new(payload);
        let this = MdatBoxView::parse_in(&mut cursor)?;

        Ok(this)
    }
}

#[cfg(feature = "alloc")]
pub use owned::MdatBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use crate::cursor::WriteCursor;

    use super::*;

    /// An owned Media Data Box (`mdat`).
    #[derive(Debug, Clone)]
    pub struct MdatBox {
        /// The raw data of the Media Data Box (`mdat`).
        pub data: Vec<u8>,
    }

    impl MdatBox {
        /// Creates an `MdatBox` from an `MdatBoxView`.
        pub fn from_view(view: &MdatBoxView<'_>) -> Self {
            MdatBox {
                data: view.data.to_vec(),
            }
        }

        /// Parses an `MdatBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<Self> {
            let mdat_view = MdatBoxView::parse(payload)?;
            Ok(Self::from_view(&mdat_view))
        }

        /// Returns the size of the `MdatBox` data.
        #[inline]
        pub fn size(&self) -> usize {
            self.data.len()
        }

        pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            cur.write_slice(&self.data)
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

            if !cur.is_empty() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxSize {
                        reason: "Buffer larger than expected",
                        got: cur.remaining() as u64,
                    },
                    BoxType::MDAT,
                ));
            }

            Ok(())
        }

        /// Writes this `MdatBox` into the given payload.
        pub fn write(&self, payload: &mut [u8]) -> Result<()> {
            let mut cursor = WriteCursor::new(payload);
            self.write_in(&mut cursor)
        }
    }

    impl MdatBoxView<'_> {
        /// Converts this `MdatBoxView` into an owned `MdatBox`.
        pub fn to_owned(&self) -> MdatBox {
            MdatBox::from_view(self)
        }
    }

    impl From<MdatBoxView<'_>> for MdatBox {
        fn from(view: MdatBoxView) -> Self {
            Self::from_view(&view)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mdat_box_view_parse() {
        let data = b"example media data";
        let mdat_box_view = MdatBoxView::parse(data).unwrap();
        assert_eq!(mdat_box_view.data, data);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_mdat_box_owned_write() {
        let mdat_box = MdatBox {
            data: b"example media data".to_vec(),
        };
        let mut buffer = vec![0u8; mdat_box.size()];
        mdat_box.write(&mut buffer).unwrap();
        assert_eq!(&buffer, b"example media data");
    }
}
