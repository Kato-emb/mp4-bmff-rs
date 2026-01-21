use crate::types::FourCC;

use crate::BoxType;
use crate::RawBoxRef;
use crate::error::*;

use crate::base::codec::DecodeIn;
use crate::cursor::ReadCursor;

/// A reference to a File Type Box (`ftyp`).
#[derive(Debug)]
pub struct FtypBoxView<'a> {
    /// The major brand.
    pub major_brand: FourCC,
    /// The minor version.
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

impl BmffBox for FtypBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::FTYP
    }

    fn payload_size(&self) -> u64 {
        4 + 4 + self.compatible_brands.len() as u64
    }
}

impl<'de> DecodeIn<'de> for FtypBoxView<'de> {
    fn decode_in(cur: &mut ReadCursor<'de>) -> Result<Self> {
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

    /// Parses an `FtypBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<FtypBoxView<'a>> {
        let mut cursor = ReadCursor::new(payload);
        let this = FtypBoxView::parse_in(&mut cursor)?;

        Ok(this)
    }
}

impl<'a> TryFrom<&'a [u8]> for FtypBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        FtypBoxView::parse(value)
    }
}

impl<'a> TryFrom<RawBoxRef<'a>> for FtypBoxView<'a> {
    type Error = Error;

    fn try_from(value: RawBoxRef<'a>) -> Result<Self> {
        if value.boxtype() != BoxType::FTYP {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::FTYP,
                found: value.boxtype(),
            }));
        }

        FtypBoxView::parse(value.payload())
    }
}

#[cfg(feature = "alloc")]
pub use owned::FtypBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use crate::cursor::WriteCursor;

    use super::*;
    use crate::base::codec::EncodeIn;

    /// An owned File Type Box (`ftyp`).
    #[derive(Debug, Clone)]
    pub struct FtypBox {
        /// The major brand.
        pub major_brand: FourCC,
        /// The minor version.
        pub minor_version: u32,
        /// The compatible brands.
        pub compatible_brands: Vec<FourCC>,
    }

    impl BmffBox for FtypBox {
        fn boxtype(&self) -> BoxType {
            BoxType::FTYP
        }

        fn payload_size(&self) -> u64 {
            4 // major_brand
            + 4 // minor_version
            + self.compatible_brands.len() as u64 * 4 // compatible_brands
        }
    }

    impl From<&FtypBoxView<'_>> for FtypBox {
        fn from(view: &FtypBoxView) -> Self {
            let compatible_brands = view.compatible_brands().collect::<Vec<FourCC>>();

            FtypBox {
                major_brand: view.major_brand,
                minor_version: view.minor_version,
                compatible_brands,
            }
        }
    }

    impl DecodeIn<'_> for FtypBox {
        fn decode_in(cur: &mut ReadCursor<'_>) -> Result<Self> {
            let view = FtypBoxView::decode_in(cur)?;
            Ok(FtypBox::from(&view))
        }
    }

    impl EncodeIn for FtypBox {
        fn encode_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            // Write major_brand (4 bytes)
            cur.write_array(self.major_brand.as_bytes())?;

            // Write minor_version (4 bytes)
            cur.write_u32_be(self.minor_version)?;

            // Write compatible_brands
            for brand in &self.compatible_brands {
                cur.write_array(brand.as_bytes())?;
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ftyp_box_ref_parse() {
        use crate::BoxDecode;

        let data: [u8; 20] = [
            b'i', b's', b'o', b'm', // major_brand
            0x00, 0x00, 0x02, 0x00, // minor_version (512)
            b'i', b's', b'o', b'm', // compatible_brand 1
            b'i', b's', b'o', b'2', // compatible_brand 2
            b'a', b'v', b'c', b'1', // compatible_brand 3
        ];

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

    #[cfg(feature = "alloc")]
    #[test]
    fn test_ftyp_box_write() {
        use crate::BoxEncode;

        let ftyp_view = FtypBox {
            major_brand: FourCC::new(*b"isom"),
            minor_version: 512,
            compatible_brands: vec![
                FourCC::new(*b"isom"),
                FourCC::new(*b"iso2"),
                FourCC::new(*b"avc1"),
            ],
        };

        let mut buffer = vec![0u8; ftyp_view.payload_size() as usize];
        ftyp_view.encode(&mut buffer).unwrap();

        let expected: [u8; 20] = [
            b'i', b's', b'o', b'm', // major_brand
            0x00, 0x00, 0x02, 0x00, // minor_version (512)
            b'i', b's', b'o', b'm', // compatible_brand 1
            b'i', b's', b'o', b'2', // compatible_brand 2
            b'a', b'v', b'c', b'1', // compatible_brand 3
        ];

        assert_eq!(buffer, expected);
    }
}
