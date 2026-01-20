use crate::cursor::ReadCursor;
use crate::types::FourCC;

use crate::BoxFrame;
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

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<StypBoxView<'a>> {
        // Read major_brand (4 bytes)
        let major_brand = cur
            .read_array::<4>()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let major_brand = FourCC::new(major_brand);

        // Read minor_version (4 bytes)
        let minor_version = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

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

    /// Parses a `StypBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<StypBoxView<'a>> {
        let mut cursor = ReadCursor::new(payload);
        let this = StypBoxView::parse_in(&mut cursor)?;

        Ok(this)
    }
}

impl<'a> TryFrom<&'a [u8]> for StypBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        StypBoxView::parse(value)
    }
}

impl<'a> TryFrom<BoxFrame<'a>> for StypBoxView<'a> {
    type Error = Error;

    fn try_from(value: BoxFrame<'a>) -> Result<Self> {
        if value.boxtype() != BoxType::STYP {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::STYP,
                found: value.boxtype(),
            }));
        }

        StypBoxView::parse(value.payload())
    }
}

#[cfg(feature = "alloc")]
pub use owned::StypBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

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

    impl StypBox {
        /// Creates a `StypBox` from a `StypBoxView`.
        pub fn from_view(view: &StypBoxView) -> Self {
            let compatible_brands = view.compatible_brands().collect::<Vec<FourCC>>();

            StypBox {
                major_brand: view.major_brand,
                minor_version: view.minor_version,
                compatible_brands,
            }
        }

        /// Parses a `StypBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<Self> {
            let styp_view = StypBoxView::parse(payload)?;
            Ok(Self::from_view(&styp_view))
        }

        /// Returns the size of the `StypBox` data.
        pub fn size(&self) -> usize {
            4 + 4 + self.compatible_brands.len() * 4
        }

        pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            // Write major_brand (4 bytes)
            cur.write_array(self.major_brand.as_bytes())
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

            // Write minor_version (4 bytes)
            cur.write_u32_be(self.minor_version)
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

            // Write compatible_brands
            for brand in &self.compatible_brands {
                cur.write_array(brand.as_bytes())
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
            }

            if !cur.is_empty() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxSize {
                        reason: "Buffer larger than expected",
                        got: cur.remaining() as u64,
                    },
                    BoxType::STYP,
                ));
            }

            Ok(())
        }

        /// Writes this `StypBox` into the given payload.
        pub fn write(&self, payload: &mut [u8]) -> Result<()> {
            let mut cursor = WriteCursor::new(payload);
            self.write_in(&mut cursor)
        }
    }

    impl StypBoxView<'_> {
        /// Converts this `StypBoxView` into an owned `StypBox`.
        pub fn to_owned(&self) -> StypBox {
            StypBox::from_view(self)
        }
    }

    impl From<StypBoxView<'_>> for StypBox {
        fn from(view: StypBoxView) -> Self {
            Self::from_view(&view)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "alloc")]
    #[test]
    fn styp_box_round_trip() {
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
        let mut buf = vec![0u8; original.size()];
        original.write(&mut buf).unwrap();

        // Parse
        let reparsed = StypBox::parse(&buf).unwrap();

        assert_eq!(reparsed.major_brand, original.major_brand);
        assert_eq!(reparsed.minor_version, original.minor_version);
        assert_eq!(reparsed.compatible_brands, original.compatible_brands);
    }
}
