use crate::cursor::ReadCursor;
use crate::types::FourCC;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;

/// A reference to a Segment Type Box (`styp`).
#[derive(Debug)]
pub struct StypBoxView<'a> {
    /// The major brand.
    pub major_brand: FourCC,
    /// The minor version.
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

impl<'a> TryFrom<&'a [u8]> for StypBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        StypBoxView::decode(value)
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
pub use owned::StypBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use crate::BoxEncode;
    use crate::cursor::WriteCursor;

    use super::*;

    /// An owned Segment Type Box (`styp`).
    #[derive(Debug, Clone)]
    pub struct StypBox {
        /// The major brand.
        pub major_brand: FourCC,
        /// The minor version.
        pub minor_version: u32,
        /// The compatible brands.
        pub compatible_brands: Vec<FourCC>,
    }

    impl From<StypBoxView<'_>> for StypBox {
        fn from(view: StypBoxView) -> Self {
            let compatible_brands = view.compatible_brands().collect::<Vec<FourCC>>();

            StypBox {
                major_brand: view.major_brand,
                minor_version: view.minor_version,
                compatible_brands,
            }
        }
    }

    impl BoxCodec for StypBox {
        fn boxtype(&self) -> BoxType {
            BoxType::STYP
        }
    }

    impl BoxDecode<'_> for StypBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = StypBoxView::decode(bytes)?;
            Ok(StypBox::from(view))
        }
    }

    impl BoxEncode for StypBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            4 // major_brand
                + 4 // minor_version
                + self.compatible_brands.len() * 4 // compatible_brands
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "alloc")]
    #[test]
    fn styp_box_round_trip() {
        use crate::BoxEncode;

        let original = StypBox {
            major_brand: FourCC::new(*b"isom"),
            minor_version: 512,
            compatible_brands: vec![
                FourCC::new(*b"isom"),
                FourCC::new(*b"iso6"),
                FourCC::new(*b"msdh"),
            ],
        };

        // Write
        let mut buf = vec![0u8; 256];
        let written = original.encode_into(&mut buf).unwrap();

        // Parse
        let reparsed = StypBox::decode(&buf[..written]).unwrap();

        assert_eq!(reparsed.major_brand, original.major_brand);
        assert_eq!(reparsed.minor_version, original.minor_version);
        assert_eq!(reparsed.compatible_brands, original.compatible_brands);
    }
}
