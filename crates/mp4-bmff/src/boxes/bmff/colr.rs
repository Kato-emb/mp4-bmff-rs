//! Colour Information Box (`colr`) implementation.
//!
//! The Colour Information Box provides colour characteristics of visual
//! samples, supporting NCLX colour parameters and ICC profiles.

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;

use crate::cursor::ReadCursor;

/// An indication of the colour information supplied.
///
/// The `colr` box can contain either NCLX colour information or ICC profiles,
/// and this enum captures the different types of colour information that can be present in a `colr` box.
#[derive(Debug, Clone)]
pub enum ColourType<T> {
    /// NCLX colour information, which includes colour primaries, transfer characteristics, matrix coefficients, and a full range flag.
    Nclx {
        /// Colour primaries, specified as an unsigned 16-bit integer.
        colour_primaries: u16,
        /// Transfer characteristics, specified as an unsigned 16-bit integer.
        transfer_characteristics: u16,
        /// Matrix coefficients, specified as an unsigned 16-bit integer.
        matrix_coefficients: u16,
        /// Full range flag, indicating whether the colour values are in full range (0-255) or limited range (16-235).
        full_range_flag: bool,
    },
    /// Restricted ICC profile, which contains a byte array representing the ICC profile data.
    RestrictedICC {
        /// ICC profile data for restricted colour information.
        icc_profile: T,
    },
    /// Unrestricted ICC profile, which contains a byte array representing the ICC profile data.
    UnrestrictedICC {
        /// ICC profile data for unrestricted colour information.
        icc_profile: T,
    },
}

/// A reference to a Colour Information Box (`colr`).
///
/// This box provides information about the colour characteristics of visual samples, such as the colour primaries, transfer characteristics, and matrix coefficients. It can also contain ICC profiles for more detailed colour information. The `colr` box is typically used within video sample entries to specify the colour properties of the video content.
///
/// # Structure
/// - `colour_type`: Indicates the type of colour information provided. It can be one of the following:
///   - `nclx`: Specifies colour information using the NCLX format, which includes colour primaries, transfer characteristics, matrix coefficients, and a full range flag.
///   - `rICC`: Contains a restricted ICC profile for colour information.
///   - `prof`: Contains an unrestricted ICC profile for colour information.
#[derive(Debug, Clone)]
pub struct ColrBoxView<'a> {
    /// The type of colour information provided in the `colr` box, which can be NCLX or ICC profile data.
    pub colour_type: ColourType<&'a [u8]>,
}

impl BoxCodec for ColrBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::COLR
    }
}

impl<'de> BoxDecode<'de> for ColrBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let colour_type = cur.read_array::<4>()?;
        let colour_type = match &colour_type {
            b"nclx" => {
                let colour_primaries = cur.read_u16_be()?;
                let transfer_characteristics = cur.read_u16_be()?;
                let matrix_coefficients = cur.read_u16_be()?;
                let full_range_flag = cur.read_u8()? & 0x80 != 0;
                ColourType::Nclx {
                    colour_primaries,
                    transfer_characteristics,
                    matrix_coefficients,
                    full_range_flag,
                }
            }
            b"rICC" => {
                let icc_profile = cur.take(cur.remaining())?;
                ColourType::RestrictedICC { icc_profile }
            }
            b"prof" => {
                let icc_profile = cur.take(cur.remaining())?;
                ColourType::UnrestrictedICC { icc_profile }
            }
            _ => {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxField {
                        field: "colour_type",
                        reason: "The defined colour types are 'nclx', 'rICC', and 'prof'",
                    },
                    BoxType::COLR,
                ));
            }
        };

        Ok(ColrBoxView { colour_type })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use alloc::vec::Vec;

    use super::*;
    use crate::BoxEncode;

    use crate::cursor::WriteCursor;

    /// An owned Colour Information Box (`colr`).
    #[derive(Debug, Clone)]
    pub struct ColrBox {
        /// The type of colour information provided in the `colr` box, which can be NCLX or ICC profile data. This is the owned variant of `ColourType` that stores ICC profile data in a `Vec<u8>`.
        pub colour_type: ColourType<Vec<u8>>,
    }

    impl TryFrom<&ColrBoxView<'_>> for ColrBox {
        type Error = Error;

        fn try_from(view: &ColrBoxView<'_>) -> Result<Self> {
            let colour_type = match &view.colour_type {
                ColourType::Nclx {
                    colour_primaries,
                    transfer_characteristics,
                    matrix_coefficients,
                    full_range_flag,
                } => ColourType::Nclx {
                    colour_primaries: *colour_primaries,
                    transfer_characteristics: *transfer_characteristics,
                    matrix_coefficients: *matrix_coefficients,
                    full_range_flag: *full_range_flag,
                },
                ColourType::RestrictedICC { icc_profile } => ColourType::RestrictedICC {
                    icc_profile: icc_profile.to_vec(),
                },
                ColourType::UnrestrictedICC { icc_profile } => ColourType::UnrestrictedICC {
                    icc_profile: icc_profile.to_vec(),
                },
            };
            Ok(ColrBox { colour_type })
        }
    }

    impl BoxCodec for ColrBox {
        fn boxtype(&self) -> BoxType {
            BoxType::COLR
        }
    }

    impl BoxDecode<'_> for ColrBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = ColrBoxView::decode(bytes)?;
            Self::try_from(&view)
        }
    }

    impl BoxEncode for ColrBox {
        fn encoded_len(&self) -> usize {
            4 + match &self.colour_type {
                ColourType::Nclx { .. } => 7, // 3 u16 fields (6 bytes) + 1 u8 full_range_flag (1 byte)
                ColourType::RestrictedICC { icc_profile } => icc_profile.len(),
                ColourType::UnrestrictedICC { icc_profile } => icc_profile.len(),
            }
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            match &self.colour_type {
                ColourType::Nclx {
                    colour_primaries,
                    transfer_characteristics,
                    matrix_coefficients,
                    full_range_flag,
                } => {
                    cur.write_array(b"nclx")?;
                    cur.write_u16_be(*colour_primaries)?;
                    cur.write_u16_be(*transfer_characteristics)?;
                    cur.write_u16_be(*matrix_coefficients)?;
                    let full_range_byte = if *full_range_flag { 0x80 } else { 0x00 };
                    cur.write_u8(full_range_byte)?;
                }
                ColourType::RestrictedICC { icc_profile } => {
                    cur.write_array(b"rICC")?;
                    cur.write_slice(icc_profile)?;
                }
                ColourType::UnrestrictedICC { icc_profile } => {
                    cur.write_array(b"prof")?;
                    cur.write_slice(icc_profile)?;
                }
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

    fn raw_data_nclx() -> [u8; 11] {
        [
            b'n', b'c', b'l', b'x', // colour_type = "nclx"
            0x00, 0x01, // colour_primaries = 1 (BT.709)
            0x00, 0x0D, // transfer_characteristics = 13 (sRGB)
            0x00, 0x01, // matrix_coefficients = 1 (BT.709)
            0x80, // full_range_flag = true
        ]
    }

    fn raw_data_ricc() -> [u8; 8] {
        [
            b'r', b'I', b'C', b'C', // colour_type = "rICC"
            0xDE, 0xAD, 0xBE, 0xEF, // icc_profile (4 bytes)
        ]
    }

    #[test]
    fn test_colr_box_view_decode_nclx() {
        let data = raw_data_nclx();
        let colr = ColrBoxView::decode(&data).unwrap();

        match colr.colour_type {
            ColourType::Nclx {
                colour_primaries,
                transfer_characteristics,
                matrix_coefficients,
                full_range_flag,
            } => {
                assert_eq!(colour_primaries, 1);
                assert_eq!(transfer_characteristics, 13);
                assert_eq!(matrix_coefficients, 1);
                assert!(full_range_flag);
            }
            _ => panic!("expected Nclx"),
        }
    }

    #[test]
    fn test_colr_box_view_decode_ricc() {
        let data = raw_data_ricc();
        let colr = ColrBoxView::decode(&data).unwrap();

        match colr.colour_type {
            ColourType::RestrictedICC { icc_profile } => {
                assert_eq!(icc_profile, &[0xDE, 0xAD, 0xBE, 0xEF]);
            }
            _ => panic!("expected RestrictedICC"),
        }
    }

    #[test]
    fn test_colr_box_view_decode_prof() {
        let data = [
            b'p', b'r', b'o', b'f', // colour_type = "prof"
            0x01, 0x02, // icc_profile (2 bytes)
        ];
        let colr = ColrBoxView::decode(&data).unwrap();

        match colr.colour_type {
            ColourType::UnrestrictedICC { icc_profile } => {
                assert_eq!(icc_profile, &[0x01, 0x02]);
            }
            _ => panic!("expected UnrestrictedICC"),
        }
    }

    #[test]
    fn test_colr_box_view_decode_invalid_type() {
        let data = [b'x', b'y', b'z', b'w'];
        let result = ColrBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_colr_box_view_decode_truncated() {
        let data: [u8; 2] = [0x00; 2];
        let result = ColrBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_colr_box_round_trip_nclx() {
        use crate::BoxEncode;

        let original = raw_data_nclx();
        let colr = ColrBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; colr.encoded_len()];
        colr.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_colr_box_round_trip_ricc() {
        use crate::BoxEncode;

        let original = raw_data_ricc();
        let colr = ColrBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; colr.encoded_len()];
        colr.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_colr_box_try_from() {
        let data = raw_data_nclx();
        let view = ColrBoxView::decode(&data).unwrap();
        let owned = ColrBox::try_from(&view).unwrap();

        match owned.colour_type {
            ColourType::Nclx {
                colour_primaries,
                transfer_characteristics,
                matrix_coefficients,
                full_range_flag,
            } => {
                assert_eq!(colour_primaries, 1);
                assert_eq!(transfer_characteristics, 13);
                assert_eq!(matrix_coefficients, 1);
                assert!(full_range_flag);
            }
            _ => panic!("expected Nclx"),
        }
    }
}
