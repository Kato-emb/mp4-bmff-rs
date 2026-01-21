use crate::BmffBox;
use crate::BoxType;
use crate::error::*;

use crate::base::codec::DecodeIn;
use crate::cursor::ReadCursor;

/// A reference to a Free Space Box (`free`).
#[derive(Debug)]
pub struct FreeBoxView<'a> {
    /// The raw data of the Free Space Box (`free`).
    pub data: &'a [u8],
}

impl BmffBox for FreeBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::FREE
    }

    fn payload_size(&self) -> u64 {
        self.data.len() as u64
    }
}

impl<'de> DecodeIn<'de> for FreeBoxView<'de> {
    fn decode_in(cur: &mut ReadCursor<'de>) -> Result<Self> {
        let data = cur.take(cur.remaining())?;
        Ok(FreeBoxView { data })
    }
}

#[cfg(feature = "alloc")]
pub use owned::FreeBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use super::*;

    use crate::base::codec::EncodeIn;
    use crate::cursor::WriteCursor;

    /// An owned Free Space Box (`free`).
    #[derive(Debug, Clone)]
    pub struct FreeBox {
        /// The raw data of the Free Space Box (`free`).
        pub data: Vec<u8>,
    }

    impl BmffBox for FreeBox {
        fn boxtype(&self) -> BoxType {
            BoxType::FREE
        }

        fn payload_size(&self) -> u64 {
            self.data.len() as u64
        }
    }

    impl From<&FreeBoxView<'_>> for FreeBox {
        fn from(view: &FreeBoxView) -> Self {
            let data = view.data.to_vec();
            FreeBox { data }
        }
    }

    impl DecodeIn<'_> for FreeBox {
        fn decode_in(cur: &mut ReadCursor<'_>) -> Result<Self> {
            let view = FreeBoxView::decode_in(cur)?;
            Ok(FreeBox::from(&view))
        }
    }

    impl EncodeIn for FreeBox {
        fn encode_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            // Write the data
            cur.write_slice(&self.data)?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BoxDecode;

    #[test]
    fn test_free_box_view_parse() {
        let data = b"example free space data";
        let free_box_view = FreeBoxView::decode(data).unwrap();
        assert_eq!(free_box_view.data, data);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_free_box_owned_write() {
        use crate::BoxEncode;
        let free_box = FreeBox {
            data: b"example free space data".to_vec(),
        };

        let mut buffer = vec![0u8; free_box.payload_size() as usize];
        free_box.encode(&mut buffer).unwrap();

        let expected = b"example free space data";

        assert_eq!(buffer, expected);
    }
}
