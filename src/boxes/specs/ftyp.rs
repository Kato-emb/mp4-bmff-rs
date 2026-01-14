//! File Type Box (`ftyp`) implementation.
//!
//! The File Type Box identifies the specifications to which the file complies.
//! It contains a major brand, minor version, and a list of compatible brands.
//!
//! Reference: ISO/IEC 14496-12:2022 Section 4.3

use crate::boxes::error::*;
use crate::cursor::ReadCursor;
use crate::types::FourCC;

/// A reference to a File Type Box (`ftyp`).
#[derive(Debug)]
pub struct FtypBoxRef<'a> {
    /// The major brand.
    pub major_brand: FourCC,
    /// The minor version.
    pub minor_version: u32,
    compatible_brands: &'a [u8],
}

impl<'a> FtypBoxRef<'a> {
    /// Returns an iterator over the compatible brands.
    pub fn compatible_brands(&self) -> impl Iterator<Item = FourCC> + 'a {
        self.compatible_brands
            .chunks_exact(4)
            .map(|chunk| FourCC::new([chunk[0], chunk[1], chunk[2], chunk[3]]))
    }

    /// Parses an `FtypBoxRef` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<FtypBoxRef<'a>> {
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

        Ok(FtypBoxRef {
            major_brand,
            minor_version,
            compatible_brands,
        })
    }

    /// Converts this `FtypBoxRef` into an owned `FtypBox`.
    #[cfg(feature = "alloc")]
    pub fn to_owned(&self) -> FtypBox {
        FtypBox::from_ref(self)
    }
}

#[cfg(feature = "alloc")]
pub use owned::FtypBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use super::*;
    use crate::cursor::WriteCursor;

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
        /// Creates an `FtypBox` from an `FtypBoxRef`.
        pub fn from_ref(ftyp_ref: &FtypBoxRef) -> Self {
            let compatible_brands = ftyp_ref.compatible_brands().collect::<Vec<FourCC>>();

            FtypBox {
                major_brand: ftyp_ref.major_brand,
                minor_version: ftyp_ref.minor_version,
                compatible_brands,
            }
        }

        /// Parses an `FtypBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<Self> {
            let ftyp_ref = FtypBoxRef::parse(payload)?;
            Ok(Self::from_ref(&ftyp_ref))
        }

        /// Writes the `FtypBox` to the given `WriteCursor`.
        pub fn write(&self, cur: &mut WriteCursor) -> Result<()> {
            cur.write_array(self.major_brand.as_bytes())
                .map_err(|e| Error::new(e.into()).at(cur.position() as u64))?;
            cur.write_u32_be(self.minor_version)
                .map_err(|e| Error::new(e.into()).at(cur.position() as u64))?;
            for brand in &self.compatible_brands {
                cur.write_array(brand.as_bytes())
                    .map_err(|e| Error::new(e.into()).at(cur.position() as u64))?;
            }

            Ok(())
        }
    }

    impl From<FtypBoxRef<'_>> for FtypBox {
        fn from(ftyp_ref: FtypBoxRef) -> Self {
            Self::from_ref(&ftyp_ref)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_ftyp_box_owned_write() {
            let ftyp = FtypBox {
                major_brand: FourCC::new(*b"isom"),
                minor_version: 512,
                compatible_brands: vec![
                    FourCC::new(*b"isom"),
                    FourCC::new(*b"iso2"),
                    FourCC::new(*b"avc1"),
                ],
            };

            let mut buffer = [0u8; 20];
            let mut cursor = WriteCursor::new(&mut buffer);

            ftyp.write(&mut cursor).unwrap();

            let expected: [u8; 20] = [
                b'i', b's', b'o', b'm', // major_brand
                0x00, 0x00, 0x02, 0x00, // minor_version (512)
                b'i', b's', b'o', b'm', // compatible_brand 1
                b'i', b's', b'o', b'2', // compatible_brand 2
                b'a', b'v', b'c', b'1', // compatible_brand 3
            ];

            assert_eq!(&buffer, &expected);
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

        let ftyp_ref = FtypBoxRef::parse(&data).unwrap();

        assert_eq!(ftyp_ref.major_brand, FourCC::new(*b"isom"));
        assert_eq!(ftyp_ref.minor_version, 512);

        let compatible_brands: Vec<FourCC> = ftyp_ref.compatible_brands().collect();
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
