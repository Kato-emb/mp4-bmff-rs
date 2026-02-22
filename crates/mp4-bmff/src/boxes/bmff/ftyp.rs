//! File Type Box (`ftyp`) implementation.
//!
//! The File Type Box identifies the specifications to which the file complies.
//! It is typically placed at the beginning of the file to allow readers to
//! quickly determine whether they can process the file.

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::types::*;

use crate::cursor::ReadCursor;

/// A reference to a File Type Box (`ftyp`).
///
/// The File Type Box contains brand information that identifies the
/// specifications with which the file is compliant. This is typically
/// the first box in an ISO Base Media File Format file.
///
/// # Structure
///
/// - `major_brand`: The brand identifier for the best use of the file.
/// - `minor_version`: An informative integer for the minor version of the major brand.
/// - `compatible_brands`: A list of brands with which the file is compatible.
#[derive(Debug)]
pub struct FtypBoxView<'a> {
    /// The brand identifier for the best use of the file.
    pub major_brand: FourCC,
    /// The minor version of the major brand.
    pub minor_version: u32,
    compatible_brands: &'a [u8],
}

impl<'a> FtypBoxView<'a> {
    /// Returns an iterator over the compatible brands.
    pub fn compatible_brands(&self) -> impl Iterator<Item = FourCC> + 'a {
        self.compatible_brands
            .chunks_exact(4)
            .map(|chunk| FourCC::new([chunk[0], chunk[1], chunk[2], chunk[3]]))
    }
}

impl BoxCodec for FtypBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::FTYP
    }
}

impl<'de> BoxDecode<'de> for FtypBoxView<'de> {
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
                BoxType::FTYP,
            ));
        }

        let compatible_brands = cur.take(remaining)?;

        Ok(FtypBoxView {
            major_brand,
            minor_version,
            compatible_brands,
        })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use super::*;
    use crate::BoxEncode;

    use crate::cursor::WriteCursor;

    /// An owned File Type Box (`ftyp`).
    ///
    /// This is the owned variant of [`FtypBoxView`] that stores compatible brands
    /// in a heap-allocated vector.
    ///
    /// # Example
    ///
    /// ```
    /// use mp4_bmff::boxes::bmff::FtypBox;
    /// use mp4_bmff::types::FourCC;
    ///
    /// let ftyp = FtypBox {
    ///     major_brand: FourCC::new(*b"isom"),
    ///     minor_version: 512,
    ///     compatible_brands: vec![
    ///         FourCC::new(*b"isom"),
    ///         FourCC::new(*b"iso2"),
    ///     ],
    /// };
    /// ```
    #[derive(Debug, Clone)]
    pub struct FtypBox {
        /// The brand identifier for the best use of the file.
        pub major_brand: FourCC,
        /// The minor version of the major brand.
        pub minor_version: u32,
        /// A list of brands with which the file is compatible.
        pub compatible_brands: Vec<FourCC>,
    }

    impl From<&FtypBoxView<'_>> for FtypBox {
        fn from(view: &FtypBoxView<'_>) -> Self {
            let compatible_brands = view.compatible_brands().collect::<Vec<FourCC>>();

            FtypBox {
                major_brand: view.major_brand,
                minor_version: view.minor_version,
                compatible_brands,
            }
        }
    }

    impl FtypBoxView<'_> {
        /// Converts this view into an owned `FtypBox`.
        pub fn to_owned(&self) -> FtypBox {
            FtypBox::from(self)
        }
    }

    impl BoxCodec for FtypBox {
        fn boxtype(&self) -> BoxType {
            BoxType::FTYP
        }
    }

    impl BoxDecode<'_> for FtypBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = FtypBoxView::decode(bytes)?;
            Ok(FtypBox::from(&view))
        }
    }

    impl BoxEncode for FtypBox {
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
pub use owned::FtypBox;

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
    fn test_ftyp_box_view_decode() {
        let data = raw_data();
        let ftyp_view = FtypBoxView::decode(&data).unwrap();

        assert_eq!(ftyp_view.major_brand, FourCC::new(*b"isom"));
        assert_eq!(ftyp_view.minor_version, 512);

        let compatible_brands: Vec<FourCC> = ftyp_view.compatible_brands().collect();
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
    fn test_ftyp_box_view_no_compatible_brands() {
        let data: [u8; 8] = [
            b'i', b's', b'o', b'm', // major_brand
            0x00, 0x00, 0x00, 0x01, // minor_version (1)
        ];

        let ftyp_view = FtypBoxView::decode(&data).unwrap();
        assert_eq!(ftyp_view.major_brand, FourCC::new(*b"isom"));
        assert_eq!(ftyp_view.minor_version, 1);
        assert_eq!(ftyp_view.compatible_brands().count(), 0);
    }

    #[test]
    fn test_ftyp_box_view_invalid_size() {
        // 9 bytes: major_brand(4) + minor_version(4) + 1 extra byte (not multiple of 4)
        let data: [u8; 9] = [
            b'i', b's', b'o', b'm', // major_brand
            0x00, 0x00, 0x00, 0x01, // minor_version
            0x00, // invalid extra byte
        ];

        let result = FtypBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_ftyp_box_view_truncated() {
        // Only 4 bytes, missing minor_version
        let data: [u8; 4] = [b'i', b's', b'o', b'm'];

        let result = FtypBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_ftyp_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_data();
        let ftyp_box = FtypBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; ftyp_box.encoded_len()];
        ftyp_box.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_ftyp_box_to_owned() {
        let data = raw_data();
        let view = FtypBoxView::decode(&data).unwrap();
        let owned = view.to_owned();

        assert_eq!(owned.major_brand, view.major_brand);
        assert_eq!(owned.minor_version, view.minor_version);
        assert_eq!(
            owned.compatible_brands.len(),
            view.compatible_brands().count()
        );
    }
}
