//! Segment Type Box (`styp`) implementation.
//!
//! The Segment Type Box identifies the specifications to which a segment
//! conforms, similar to how `ftyp` identifies a complete file. It appears
//! at the beginning of each media segment in segmented/streaming delivery.
//!
//! This box has the same structure as the File Type Box (`ftyp`) but uses
//! a different box type to distinguish segment-level branding.

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::types::*;

use crate::cursor::ReadCursor;

/// A reference to a Segment Type Box (`styp`).
///
/// Identifies the specifications a media segment conforms to. Used in
/// segmented delivery (DASH, HLS) to brand individual segments.
///
/// # Structure
///
/// - `major_brand`: Primary specification the segment conforms to.
/// - `minor_version`: Informative version of the major brand.
/// - `compatible_brands`: List of specifications this segment is compatible with.
#[derive(Debug)]
pub struct StypBoxView<'a> {
    /// Primary brand/specification this segment conforms to.
    pub major_brand: FourCC,
    /// Informative version number of the major brand.
    pub minor_version: u32,
    compatible_brands: &'a [u8],
}

impl<'a> StypBoxView<'a> {
    /// Returns an iterator over the compatible brands.
    pub fn compatible_brands(&self) -> impl Iterator<Item = FourCC> + 'a {
        self.compatible_brands
            .chunks_exact(4)
            .map(|chunk| FourCC::new([chunk[0], chunk[1], chunk[2], chunk[3]]))
    }
}

impl BoxCodec for StypBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::STYP
    }
}

impl<'de> BoxDecode<'de> for StypBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        // Read major_brand (4 bytes)
        let major_brand = cur.read_array::<4>()?;
        let major_brand = FourCC::new(major_brand);

        // Read minor_version (4 bytes)
        let minor_version = cur.read_u32_be()?;

        // The remaining bytes are compatible_brands
        let remaining = cur.remaining();
        if !remaining.is_multiple_of(4) {
            return Err(Error::at_in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Compatible brands length is not a multiple of 4",
                    got: remaining as u64,
                },
                cur.position() as u64,
                BoxType::STYP,
            ));
        }

        let compatible_brands = cur.take(remaining)?;

        Ok(StypBoxView {
            major_brand,
            minor_version,
            compatible_brands,
        })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use alloc::vec::Vec;

    use super::*;
    use crate::BoxEncode;

    use crate::cursor::WriteCursor;

    /// An owned Segment Type Box (`styp`).
    ///
    /// This is the owned variant of [`StypBoxView`] that stores compatible
    /// brands in a heap-allocated vector.
    ///
    /// # Structure
    ///
    /// - `major_brand`: Primary specification the segment conforms to.
    /// - `minor_version`: Informative version of the major brand.
    /// - `compatible_brands`: Other specifications this segment supports.
    #[derive(Debug, Clone)]
    pub struct StypBox {
        /// Primary brand/specification this segment conforms to.
        pub major_brand: FourCC,
        /// Informative version number of the major brand.
        pub minor_version: u32,
        /// List of compatible brands/specifications.
        pub compatible_brands: Vec<FourCC>,
    }

    impl BoxCodec for StypBox {
        fn boxtype(&self) -> BoxType {
            BoxType::STYP
        }
    }

    impl From<&StypBoxView<'_>> for StypBox {
        fn from(view: &StypBoxView<'_>) -> Self {
            let compatible_brands = view.compatible_brands().collect::<Vec<FourCC>>();

            StypBox {
                major_brand: view.major_brand,
                minor_version: view.minor_version,
                compatible_brands,
            }
        }
    }

    impl StypBoxView<'_> {
        /// Converts this view into an owned `StypBox`.
        pub fn to_owned(&self) -> StypBox {
            StypBox::from(self)
        }
    }

    impl BoxDecode<'_> for StypBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = StypBoxView::decode(bytes)?;
            Ok(StypBox::from(&view))
        }
    }

    impl BoxEncode for StypBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            4 // major_brand(4)
                + 4 // minor_version(4)
                + (4 * self.compatible_brands.len()) // compatible_brands
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            // Write major_brand (4 bytes)
            cur.write_array(self.major_brand.as_bytes())?;

            // Write minor_version (4 bytes)
            cur.write_u32_be(self.minor_version)?;

            // Write compatible_brands
            for brand in &self.compatible_brands {
                cur.write_array(brand.as_bytes())?;
            }

            Ok(cur.position())
        }
    }
}

#[cfg(feature = "alloc")]
pub use owned::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_data() -> [u8; 20] {
        [
            b'i', b's', b'o', b'm', // major_brand
            0x00, 0x00, 0x02, 0x00, // minor_version (512)
            b'i', b's', b'o', b'm', // compatible_brand 1
            b'i', b's', b'o', b'2', // compatible_brand 2
            b'a', b'v', b'c', b'1', // compatible_brand 3
        ]
    }

    #[test]
    fn test_styp_box_view_decode() {
        let data = raw_data();
        let styp_view = StypBoxView::decode(&data).unwrap();

        assert_eq!(styp_view.major_brand, FourCC::new(*b"isom"));
        assert_eq!(styp_view.minor_version, 512);

        let compatible_brands: Vec<FourCC> = styp_view.compatible_brands().collect();
        assert_eq!(
            compatible_brands,
            vec![
                FourCC::new(*b"isom"),
                FourCC::new(*b"iso2"),
                FourCC::new(*b"avc1"),
            ]
        );
    }

    #[test]
    fn test_styp_box_view_no_compatible_brands() {
        let data: [u8; 8] = [
            b'i', b's', b'o', b'm', // major_brand
            0x00, 0x00, 0x00, 0x01, // minor_version (1)
        ];

        let styp_view = StypBoxView::decode(&data).unwrap();
        assert_eq!(styp_view.major_brand, FourCC::new(*b"isom"));
        assert_eq!(styp_view.minor_version, 1);
        assert_eq!(styp_view.compatible_brands().count(), 0);
    }

    #[test]
    fn test_styp_box_view_invalid_size() {
        // 9 bytes: major_brand(4) + minor_version(4) + 1 extra byte (not multiple of 4)
        let data: [u8; 9] = [
            b'i', b's', b'o', b'm', // major_brand
            0x00, 0x00, 0x00, 0x01, // minor_version
            0x00, // invalid extra byte
        ];

        let result = StypBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_styp_box_view_truncated() {
        // Only 4 bytes, missing minor_version
        let data: [u8; 4] = [b'i', b's', b'o', b'm'];

        let result = StypBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_styp_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_data();
        let styp_box = StypBox::decode(&original).unwrap();
        let mut encoded = vec![0u8; styp_box.encoded_len()];
        styp_box.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_styp_box_to_owned() {
        let data = raw_data();
        let view = StypBoxView::decode(&data).unwrap();
        let owned = view.to_owned();

        assert_eq!(owned.major_brand, view.major_brand);
        assert_eq!(owned.minor_version, view.minor_version);
        assert_eq!(
            owned.compatible_brands.len(),
            view.compatible_brands().count()
        );
    }
}
