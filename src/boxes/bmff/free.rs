use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;

/// A reference to a Free Space Box (`free`).
#[derive(Debug)]
pub struct FreeBoxView<'a> {
    /// The raw data of the Free Space Box (`free`).
    pub data: &'a [u8],
}

impl BoxCodec for FreeBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::FREE
    }
}

impl<'de> BoxDecode<'de> for FreeBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        Ok(FreeBoxView { data: bytes })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use super::*;
    use crate::BoxEncode;

    use crate::cursor::WriteCursor;

    /// An owned Free Space Box (`free`).
    #[derive(Debug, Clone)]
    pub struct FreeBox {
        /// The raw data of the Free Space Box (`free`).
        pub data: Vec<u8>,
    }

    impl From<&FreeBoxView<'_>> for FreeBox {
        fn from(view: &FreeBoxView) -> Self {
            let data = view.data.to_vec();
            FreeBox { data }
        }
    }

    impl FreeBoxView<'_> {
        /// Converts this view into an owned `FreeBox`.
        pub fn to_owned(&self) -> FreeBox {
            FreeBox::from(self)
        }
    }

    impl BoxCodec for FreeBox {
        fn boxtype(&self) -> BoxType {
            BoxType::FREE
        }
    }

    impl BoxDecode<'_> for FreeBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = FreeBoxView::decode(bytes)?;
            Ok(FreeBox::from(&view))
        }
    }

    impl BoxEncode for FreeBox {
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
pub use owned::FreeBox;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_free_box_view_decode() {
        let data = b"example free space data";
        let free_box_view = FreeBoxView::decode(data).unwrap();
        assert_eq!(free_box_view.data, data);
    }

    #[test]
    fn test_free_box_view_empty() {
        let data: &[u8] = &[];
        let free_box_view = FreeBoxView::decode(data).unwrap();
        assert_eq!(free_box_view.data.len(), 0);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_free_box_round_trip() {
        use crate::BoxEncode;

        let original = b"round trip test data";
        let free_box = FreeBox::decode(original).unwrap();

        let mut encoded = vec![0u8; free_box.encoded_len()];
        free_box.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], original);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_free_box_to_owned() {
        let data = b"to_owned test";
        let view = FreeBoxView::decode(data).unwrap();
        let owned = view.to_owned();

        assert_eq!(owned.data, data);
    }
}
