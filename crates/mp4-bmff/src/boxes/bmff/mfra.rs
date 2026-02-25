//! Movie Fragment Random Access Box (`mfra`) implementation.
//!
//! The Movie Fragment Random Access Box provides a table that allows random
//! access to movie fragments without parsing the entire file. It maps times
//! to byte offsets of movie fragments, enabling efficient seeking.
//!
//! This box is typically located at the end of a fragmented MP4 file and
//! is optional. The `mfro` child box allows finding `mfra` by reading from
//! the file end.

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

use super::MfroBox;
use super::TfraBoxView;

/// A reference to a Movie Fragment Random Access Box (`mfra`).
///
/// Provides random access index for fragmented movies. Located at file end
/// to enable seeking by reading backwards from EOF.
///
/// # Structure
///
/// Optional child boxes:
/// - `tfra`: Track Fragment Random Access Box - per-track seek table.
///
/// Required child boxes:
/// - `mfro`: Movie Fragment Random Access Offset Box - size of `mfra`.
#[derive(Debug)]
pub struct MfraBoxView<'a> {
    content: &'a [u8],
}

impl<'a> MfraBoxView<'a> {
    /// Returns an iterator over the child boxes of this `mfra` box.
    pub fn boxes(&self) -> BoxIter<'a> {
        BoxIter::new(self.content)
    }

    /// Returns an iterator over the Track Fragment Random Access Boxes (`tfra`) contained in this `mfra` box.
    pub fn tfras(&self) -> impl Iterator<Item = Result<TfraBoxView<'a>>> + 'a {
        self.boxes().filter_map(|result| match result {
            Ok(rawbox) if rawbox.boxtype() == BoxType::TFRA => {
                Some(TfraBoxView::decode(rawbox.into_payload()))
            }
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
    }

    /// Returns the Movie Fragment Random Access Offset Box (`mfro`) contained in this `mfra` box.
    ///
    /// # Errors
    ///
    /// Returns an error if the `mfro` box is missing or if there are multiple `mfro` boxes. The `mfro` box is required in a valid `mfra` box.
    pub fn mfro(&self) -> Result<MfroBox> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::MFRO {
                let mfro = MfroBox::decode(b.payload())?;
                return Ok(mfro);
            }
        }

        Err(Error::in_box(
            ErrorKind::BoxMissing {
                required: BoxType::MFRO,
            },
            BoxType::MFRA,
        ))
    }
}

impl BoxCodec for MfraBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::MFRA
    }
}

impl<'de> BoxDecode<'de> for MfraBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        Ok(MfraBoxView { content: bytes })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use alloc::vec::Vec;

    use super::*;
    use crate::BoxEncode;

    use crate::codec::boxed_len;
    use crate::codec::write_box_in;
    use crate::cursor::WriteCursor;

    use crate::boxes::bmff::TfraBox;

    /// An owned Movie Fragment Random Access Box (`mfra`).
    ///
    /// This is the owned variant of [`MfraBoxView`] that stores child boxes
    /// in heap-allocated memory.
    ///
    /// # Structure
    ///
    /// - `tfras`: Track Fragment Random Access boxes (one per track).
    /// - `mfro`: Movie Fragment Random Access Offset with `mfra` size.
    #[derive(Debug, Clone)]
    pub struct MfraBox {
        /// Track Fragment Random Access Boxes (one per track with seek points).
        pub tfras: Vec<TfraBox>,
        /// Movie Fragment Random Access Offset Box (contains `mfra` size).
        pub mfro: MfroBox,
    }

    impl TryFrom<&MfraBoxView<'_>> for MfraBox {
        type Error = Error;

        fn try_from(view: &MfraBoxView<'_>) -> Result<Self> {
            let mut tfras = Vec::new();
            let mut mfro = None;

            for result in view.boxes() {
                let rawbox = result?;

                match rawbox.boxtype() {
                    BoxType::TFRA => {
                        let tfra_box = TfraBox::try_from(&TfraBoxView::decode(rawbox.payload())?)?;
                        tfras.push(tfra_box);
                    }
                    BoxType::MFRO => {
                        if mfro.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::MFRO,
                                },
                                BoxType::MFRA,
                            ));
                        }
                        let mfro_box = MfroBox::decode(rawbox.payload())?;
                        mfro = Some(mfro_box);
                    }
                    _ => continue,
                }
            }

            let mfro = mfro.ok_or_else(|| {
                Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::MFRO,
                    },
                    BoxType::MFRA,
                )
            })?;

            Ok(MfraBox { tfras, mfro })
        }
    }

    impl BoxCodec for MfraBox {
        fn boxtype(&self) -> BoxType {
            BoxType::MFRA
        }
    }

    impl BoxDecode<'_> for MfraBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = MfraBoxView::decode(bytes)?;
            MfraBox::try_from(&view)
        }
    }

    impl BoxEncode for MfraBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            let mut len = 0;
            for tfra in &self.tfras {
                len += boxed_len(tfra);
            }
            len += boxed_len(&self.mfro);
            len
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            for tfra in &self.tfras {
                write_box_in(&mut cur, tfra)?;
            }

            write_box_in(&mut cur, &self.mfro)?;

            Ok(cur.position())
        }
    }
}

#[cfg(feature = "alloc")]
pub use owned::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_data_empty() -> [u8; 0] {
        []
    }

    fn raw_data_mfro_only() -> [u8; 16] {
        [
            // mfro box
            0x00, 0x00, 0x00, 0x10, // size = 16
            b'm', b'f', b'r', b'o', // type = "mfro"
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x10, 0x00, // size = 4096
        ]
    }

    fn raw_data_with_tfra_and_mfro() -> [u8; 40] {
        [
            // tfra box (version 0, no entries) - 24 bytes
            0x00, 0x00, 0x00, 0x18, // size = 24
            b't', b'f', b'r', b'a', // type = "tfra"
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x01, // track_id = 1
            0x00, 0x00, 0x00, 0x00, // reserved (26 bits) + length_size fields (6 bits)
            0x00, 0x00, 0x00, 0x00, // number_of_entry = 0
            // mfro box - 16 bytes
            0x00, 0x00, 0x00, 0x10, // size = 16
            b'm', b'f', b'r', b'o', // type = "mfro"
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x20, 0x00, // size = 8192
        ]
    }

    #[test]
    fn test_mfra_box_view_decode_empty() {
        let data = raw_data_empty();
        let mfra = MfraBoxView::decode(&data).unwrap();

        assert_eq!(mfra.boxes().count(), 0);
        assert_eq!(mfra.tfras().count(), 0);
    }

    #[test]
    fn test_mfra_box_view_missing_required_mfro() {
        let data = raw_data_empty();
        let mfra = MfraBoxView::decode(&data).unwrap();

        let result = mfra.mfro();
        assert!(result.is_err());
    }

    #[test]
    fn test_mfra_box_view_mfro_only() {
        let data = raw_data_mfro_only();
        let mfra = MfraBoxView::decode(&data).unwrap();

        assert_eq!(mfra.tfras().count(), 0);

        let mfro = mfra.mfro().unwrap();
        assert_eq!(mfro.size, 4096);
    }

    #[test]
    fn test_mfra_box_view_with_tfra_and_mfro() {
        let data = raw_data_with_tfra_and_mfro();
        let mfra = MfraBoxView::decode(&data).unwrap();

        let tfras: Vec<_> = mfra.tfras().collect();
        assert_eq!(tfras.len(), 1);

        let tfra = tfras[0].as_ref().unwrap();
        assert_eq!(tfra.track_id, 1);

        let mfro = mfra.mfro().unwrap();
        assert_eq!(mfro.size, 8192);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_mfra_box_try_from() {
        let data = raw_data_mfro_only();
        let view = MfraBoxView::decode(&data).unwrap();
        let owned = MfraBox::try_from(&view).unwrap();

        assert!(owned.tfras.is_empty());
        assert_eq!(owned.mfro.size, 4096);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_mfra_box_try_from_missing_mfro() {
        let data = raw_data_empty();
        let view = MfraBoxView::decode(&data).unwrap();
        let result = MfraBox::try_from(&view);

        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_mfra_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_data_mfro_only();
        let mfra = MfraBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; mfra.encoded_len()];
        let len = mfra.encode_into(&mut encoded).unwrap();

        assert_eq!(len, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }
}
