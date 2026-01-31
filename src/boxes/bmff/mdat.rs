use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;

/// A reference to a Media Data Box (`mdat`).
pub struct MdatBoxView<'a> {
    /// The raw data of the Media Data Box (`mdat`).
    pub data: &'a [u8],
}

impl BoxCodec for MdatBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::MDAT
    }
}

impl<'de> BoxDecode<'de> for MdatBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        Ok(MdatBoxView { data: bytes })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use super::*;
    use crate::BoxEncode;

    use crate::cursor::WriteCursor;

    /// An owned Media Data Box (`mdat`).
    #[derive(Debug, Clone)]
    pub struct MdatBox {
        /// The raw data of the Media Data Box (`mdat`).
        pub data: Vec<u8>,
    }

    impl From<&MdatBoxView<'_>> for MdatBox {
        fn from(view: &MdatBoxView) -> Self {
            MdatBox {
                data: view.data.to_vec(),
            }
        }
    }

    impl MdatBoxView<'_> {
        /// Converts this view into an owned `MdatBox`.
        pub fn to_owned(&self) -> MdatBox {
            MdatBox::from(self)
        }
    }

    impl BoxCodec for MdatBox {
        fn boxtype(&self) -> BoxType {
            BoxType::MDAT
        }
    }

    impl BoxDecode<'_> for MdatBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = MdatBoxView::decode(bytes)?;
            Ok(MdatBox::from(&view))
        }
    }

    impl BoxEncode for MdatBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            self.data.len()
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);
            cur.write_slice(&self.data)?;

            Ok(cur.position())
        }
    }
}

#[cfg(feature = "alloc")]
pub use owned::MdatBox;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mdat_box_view_parse() {
        let data = b"example media data";
        let mdat_box_view = MdatBoxView::decode(data).unwrap();
        assert_eq!(mdat_box_view.data, data);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_mdat_box_owned_parse() {
        let data = b"example media data";
        let mdat_box = MdatBox::decode(data).unwrap();
        assert_eq!(mdat_box.data, data);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_mdat_box_owned_write() {
        use crate::BoxEncode;
        let mdat_box = MdatBox {
            data: b"example media data".to_vec(),
        };
        let mut buffer = vec![0u8; 18];
        mdat_box.encode_into(&mut buffer).unwrap();
        assert_eq!(&buffer, b"example media data");
    }
}
