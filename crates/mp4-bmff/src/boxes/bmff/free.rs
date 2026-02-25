//! Free Space Box (`free`) implementation.
//!
//! The Free Space Box contains free space that may be skipped by parsers.
//! It can be used to reserve space in a file for later use or to pad
//! the file to a specific size.

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;

/// A reference to a Free Space Box (`free`).
///
/// The Free Space Box contains data that is irrelevant and may be ignored
/// by conformant readers. It is commonly used to reserve space in a file
/// that may be overwritten later, or to pad the file to a specific alignment.
///
/// The `skip` box type is functionally identical to `free`.
///
/// # Structure
///
/// - `data`: Arbitrary padding bytes. The contents are not defined and may be any value.
pub struct FreeBoxView<'a> {
    /// Arbitrary padding bytes (contents are undefined).
    pub data: &'a [u8],
}

impl core::fmt::Debug for FreeBoxView<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FreeBoxView")
            .field("data_length", &self.data.len())
            .finish()
    }
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
    use alloc::vec::Vec;

    use super::*;
    use crate::BoxEncode;

    use crate::cursor::WriteCursor;

    /// An owned Free Space Box (`free`).
    ///
    /// This is the owned variant of [`FreeBoxView`] that stores the padding
    /// data in a heap-allocated vector.
    ///
    /// # Example
    ///
    /// ```
    /// use mp4_bmff::boxes::bmff::FreeBox;
    ///
    /// // Create a free box with 1024 bytes of padding
    /// let free = FreeBox {
    ///     data: vec![0u8; 1024],
    /// };
    /// ```
    #[derive(Debug, Clone)]
    pub struct FreeBox {
        /// Arbitrary padding bytes (contents are undefined).
        pub data: Vec<u8>,
    }

    impl From<&FreeBoxView<'_>> for FreeBox {
        fn from(view: &FreeBoxView<'_>) -> Self {
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
