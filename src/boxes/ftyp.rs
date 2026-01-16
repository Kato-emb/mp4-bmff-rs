use crate::cursor::ReadCursor;
use crate::types::FourCC;

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

    /// Parses an `FtypBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<FtypBoxView<'a>> {
        let mut cursor = ReadCursor::new(payload);

        // Read major_brand (4 bytes)
        let major_brand = cursor
            .read_array::<4>()
            .map_err(|e| Error::new(e.into()).at(cursor.position() as u64))?;
        let major_brand = FourCC::new(major_brand);

        // Read minor_version (4 bytes)
        let minor_version = cursor
            .read_u32_be()
            .map_err(|e| Error::new(e.into()).at(cursor.position() as u64))?;

        // The remaining bytes are compatible_brands
        let remaining = cursor.remaining();
        if !remaining.is_multiple_of(4) {
            return Err(Error::new(ErrorKind::InvalidBoxSize {
                reason: "Compatible brands length is not a multiple of 4",
                got: remaining as u64,
            })
            .at(cursor.position() as u64));
        }
        let compatible_brands = cursor.remaining_slice();

        Ok(FtypBoxView {
            major_brand,
            minor_version,
            compatible_brands,
        })
    }
}

#[cfg(feature = "alloc")]
pub use owned::FtypBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

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
}
