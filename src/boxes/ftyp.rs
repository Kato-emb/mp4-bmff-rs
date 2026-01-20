use crate::cursor::ReadCursor;
use crate::types::FourCC;

use crate::BoxFrame;
use crate::BoxType;
use crate::error::*;

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

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<FtypBoxView<'a>> {
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

impl<'a> TryFrom<BoxFrame<'a>> for FtypBoxView<'a> {
    type Error = Error;

    fn try_from(value: BoxFrame<'a>) -> Result<Self> {
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

    impl FtypBox {
        /// Creates an `FtypBox` from an `FtypBoxView`.
        pub fn from_view(view: &FtypBoxView) -> Self {
            let compatible_brands = view.compatible_brands().collect::<Vec<FourCC>>();

            FtypBox {
                major_brand: view.major_brand,
                minor_version: view.minor_version,
                compatible_brands,
            }
        }

        /// Parses an `FtypBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<Self> {
            let ftyp_view = FtypBoxView::parse(payload)?;
            Ok(Self::from_view(&ftyp_view))
        }

        /// Writes this `FtypBox` into the given buffer.
        pub fn size(&self) -> usize {
            4 + 4 + self.compatible_brands.len() * 4
        }

        pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            // Write major_brand (4 bytes)
            cur.write_array(self.major_brand.as_bytes())?;

            // Write minor_version (4 bytes)
            cur.write_u32_be(self.minor_version)?;

            // Write compatible_brands
            for brand in &self.compatible_brands {
                cur.write_array(brand.as_bytes())?;
            }

            if !cur.is_empty() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxSize {
                        reason: "Buffer larger than expected",
                        got: cur.remaining() as u64,
                    },
                    BoxType::FTYP,
                ));
            }

            Ok(())
        }

        /// Writes this `FtypBox` into the given payload.
        pub fn write(&self, payload: &mut [u8]) -> Result<()> {
            let mut cursor = WriteCursor::new(payload);
            self.write_in(&mut cursor)
        }
    }

    impl FtypBoxView<'_> {
        /// Converts this `FtypBoxView` into an owned `FtypBox`.
        pub fn to_owned(&self) -> FtypBox {
            FtypBox::from_view(self)
        }
    }

    impl From<FtypBoxView<'_>> for FtypBox {
        fn from(view: FtypBoxView) -> Self {
            Self::from_view(&view)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ftyp_box_ref_parse() {
        let data: [u8; 20] = [
            b'i', b's', b'o', b'm', // major_brand
            0x00, 0x00, 0x02, 0x00, // minor_version (512)
            b'i', b's', b'o', b'm', // compatible_brand 1
            b'i', b's', b'o', b'2', // compatible_brand 2
            b'a', b'v', b'c', b'1', // compatible_brand 3
        ];

        let ftyp_view = FtypBoxView::parse(&data).unwrap();

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
        let ftyp_view = FtypBox {
            major_brand: FourCC::new(*b"isom"),
            minor_version: 512,
            compatible_brands: vec![
                FourCC::new(*b"isom"),
                FourCC::new(*b"iso2"),
                FourCC::new(*b"avc1"),
            ],
        };

        let mut buffer = vec![0u8; ftyp_view.size()];
        ftyp_view.write(&mut buffer).unwrap();

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
