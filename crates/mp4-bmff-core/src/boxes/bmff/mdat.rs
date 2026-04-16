//! Media Data Box (`mdat`) implementation.
//!
//! The Media Data Box contains the actual media data (audio, video, etc.)
//! for the presentation. The data is referenced by offset from the
//! Sample Table Box (`stbl`) or Track Fragment Box (`traf`).

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;

/// A reference to a Media Data Box (`mdat`).
///
/// The Media Data Box contains the actual media samples (encoded audio frames,
/// video frames, etc.). This is typically the largest box in a media file.
/// The samples are not parsed by this box; instead, the Sample Table Box (`stbl`)
/// provides byte offsets and sizes to locate individual samples within this data.
///
/// A file may contain multiple `mdat` boxes, and they may appear before or after
/// the Movie Box (`moov`). When the `moov` box appears before `mdat`, the file
/// is optimized for streaming (fast start).
///
/// # Structure
///
/// - `data`: Raw media sample data. The format depends on the codec specified
///   in the Sample Description Box (`stsd`).
pub struct MdatBoxView<'a> {
    /// Raw media sample data referenced by the Sample Table Box.
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
    ///
    /// This is the owned variant of [`MdatBoxView`] that stores the media
    /// sample data in a heap-allocated vector.
    ///
    /// # Example
    ///
    /// ```
    /// use mp4_bmff::boxes::bmff::MdatBox;
    ///
    /// // Create an mdat box containing raw media samples
    /// let mdat = MdatBox {
    ///     data: vec![0x00, 0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x1e], // H.264 NAL unit
    /// };
    /// ```
    #[derive(Debug, Clone)]
    pub struct MdatBox {
        /// Raw media sample data referenced by the Sample Table Box.
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
    fn test_mdat_box_view_decode() {
        let data = b"example media data";
        let mdat_box_view = MdatBoxView::decode(data).unwrap();
        assert_eq!(mdat_box_view.data, data);
    }

    #[test]
    fn test_mdat_box_view_empty() {
        let data: &[u8] = &[];
        let mdat_box_view = MdatBoxView::decode(data).unwrap();
        assert_eq!(mdat_box_view.data.len(), 0);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_mdat_box_round_trip() {
        use crate::BoxEncode;

        let original = b"round trip media data test";
        let mdat_box = MdatBox::decode(original).unwrap();

        let mut encoded = vec![0u8; mdat_box.encoded_len()];
        mdat_box.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], original);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_mdat_box_to_owned() {
        let data = b"to_owned test";
        let view = MdatBoxView::decode(data).unwrap();
        let owned = view.to_owned();

        assert_eq!(owned.data, data);
    }
}
