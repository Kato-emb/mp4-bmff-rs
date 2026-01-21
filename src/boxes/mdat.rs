use crate::cursor::ReadCursor;

use crate::BmffBox;
use crate::BoxType;
use crate::error::*;

use crate::base::codec::DecodeIn;

/// A reference to a Media Data Box (`mdat`).
pub struct MdatBoxView<'a> {
    /// The raw data of the Media Data Box (`mdat`).
    pub data: &'a [u8],
}

impl BmffBox for MdatBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::MDAT
    }

    fn payload_size(&self) -> u64 {
        self.data.len() as u64
    }
}

impl<'de> DecodeIn<'de> for MdatBoxView<'de> {
    fn decode_in(cur: &mut ReadCursor<'de>) -> Result<Self> {
        let data = cur.take(cur.remaining())?;
        Ok(MdatBoxView { data })
    }
}

#[cfg(feature = "alloc")]
pub use owned::MdatBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use crate::cursor::WriteCursor;

    use super::*;
    use crate::base::codec::EncodeIn;

    /// An owned Media Data Box (`mdat`).
    #[derive(Debug, Clone)]
    pub struct MdatBox {
        /// The raw data of the Media Data Box (`mdat`).
        pub data: Vec<u8>,
    }

    impl BmffBox for MdatBox {
        fn boxtype(&self) -> BoxType {
            BoxType::MDAT
        }

        fn payload_size(&self) -> u64 {
            self.data.len() as u64
        }
    }

    impl From<&MdatBoxView<'_>> for MdatBox {
        fn from(view: &MdatBoxView) -> Self {
            MdatBox {
                data: view.data.to_vec(),
            }
        }
    }

    impl DecodeIn<'_> for MdatBox {
        fn decode_in(cur: &mut ReadCursor<'_>) -> Result<Self> {
            let view = MdatBoxView::decode_in(cur)?;
            Ok(MdatBox::from(&view))
        }
    }

    impl EncodeIn for MdatBox {
        fn encode_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            // Write the raw data
            cur.write_slice(&self.data)?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mdat_box_view_parse() {
        use crate::BoxDecode;

        let data = b"example media data";
        let mdat_box_view = MdatBoxView::decode(data).unwrap();
        assert_eq!(mdat_box_view.data, data);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_mdat_box_owned_write() {
        use crate::BoxEncode;
        let mdat_box = MdatBox {
            data: b"example media data".to_vec(),
        };
        let mut buffer = vec![0u8; mdat_box.payload_size() as usize];
        mdat_box.encode(&mut buffer).unwrap();
        assert_eq!(&buffer, b"example media data");
    }
}
