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

impl<'a> TryFrom<&'a [u8]> for FreeBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        FreeBoxView::decode(value)
    }
}

#[cfg(feature = "alloc")]
pub use owned::FreeBox;

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
        fn encode(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);
            cur.write_slice(&self.data)?;

            Ok(cur.position())
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

        let mut buffer = vec![0u8; 23];
        free_box.encode(&mut buffer).unwrap();

        let expected = b"example free space data";

        assert_eq!(buffer, expected);
    }
}
