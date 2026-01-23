use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

use crate::boxes::MfroBox;
use crate::boxes::TfraBoxView;

/// A reference to a Movie Fragment Random Access Box (`mfra`).
///
/// This box contains a table that can assist in finding random access points
/// in a fragmented movie file.
#[derive(Debug)]
pub struct MfraBoxView<'a> {
    payload: &'a [u8],
}

impl<'a> MfraBoxView<'a> {
    /// Returns an iterator over the child boxes of this `MfraBoxView`.
    pub fn children(&self) -> BoxIter<'a> {
        BoxIter::new(self.payload)
    }

    /// Returns the Movie Fragment Random Access Offset Box (`mfro`).
    ///
    /// The `mfro` box is mandatory and is typically the last child box.
    pub fn mfro(&self) -> Result<MfroBox> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::MFRO {
                return MfroBox::try_from(child.payload());
            }
        }

        Err(Error::in_box(
            ErrorKind::BoxMissing {
                required: BoxType::MFRO,
            },
            BoxType::MFRA,
        ))
    }

    /// Returns an iterator over the Track Fragment Random Access Boxes (`tfra`).
    pub fn tfras(&self) -> impl Iterator<Item = Result<TfraBoxView<'a>>> + 'a {
        self.children().filter_map(|result| match result {
            Ok(box_view) if box_view.boxtype() == BoxType::TFRA => {
                Some(TfraBoxView::try_from(box_view.into_payload()))
            }
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
    }
}

impl BoxCodec for MfraBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::MFRA
    }
}

impl<'de> BoxDecode<'de> for MfraBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        Ok(MfraBoxView { payload: bytes })
    }
}

impl<'a> TryFrom<&'a [u8]> for MfraBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        MfraBoxView::decode(value)
    }
}

#[cfg(feature = "alloc")]
pub use owned::MfraBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use crate::cursor::WriteCursor;

    use super::*;
    use crate::codec::boxed_len;
    use crate::codec::write_box_in;

    use crate::BoxEncode;
    use crate::boxes::TfraBox;

    /// An owned Movie Fragment Random Access Box (`mfra`).
    #[derive(Debug, Clone)]
    pub struct MfraBox {
        /// The Track Fragment Random Access Boxes (`tfra`).
        pub tfras: Vec<TfraBox>,
        /// The Movie Fragment Random Access Offset Box (`mfro`).
        pub mfro: MfroBox,
    }

    impl TryFrom<&MfraBoxView<'_>> for MfraBox {
        type Error = Error;

        fn try_from(view: &MfraBoxView<'_>) -> Result<Self> {
            let mfro = view.mfro()?;

            let mut tfras = Vec::new();
            for tfra_result in view.tfras() {
                let tfra_view = tfra_result?;
                tfras.push(TfraBox::from(&tfra_view));
            }

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
            let mut size = 0;
            for tfra in &self.tfras {
                size += boxed_len(tfra);
            }
            size += boxed_len(&self.mfro);
            size
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

#[cfg(test)]
mod tests {
    use crate::RawBoxRef;

    use super::*;

    fn make_box(fourcc: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let size = (8 + payload.len()) as u32;
        let mut data = Vec::new();
        data.extend_from_slice(&size.to_be_bytes());
        data.extend_from_slice(fourcc);
        data.extend_from_slice(payload);
        data
    }

    fn make_full_box_header(version: u8, flags: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(version);
        data.extend_from_slice(&flags.to_be_bytes()[1..4]);
        data
    }

    fn make_mfro_payload(mfra_size: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&make_full_box_header(0, 0));
        data.extend_from_slice(&mfra_size.to_be_bytes());
        data
    }

    fn make_tfra_payload_minimal(track_id: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&make_full_box_header(0, 0));
        data.extend_from_slice(&track_id.to_be_bytes());
        // Reserved and length_size fields (all zeros)
        data.extend_from_slice(&0u32.to_be_bytes());
        // Number of entries (0)
        data.extend_from_slice(&0u32.to_be_bytes());
        data
    }

    #[test]
    fn parse_mfra_minimal() {
        let mfro = make_box(b"mfro", &make_mfro_payload(1024));

        let mfra = MfraBoxView::decode(&mfro).unwrap();

        let mfro_box = mfra.mfro().unwrap();
        assert_eq!(mfro_box.size, 1024);
        assert_eq!(mfra.tfras().count(), 0);
    }

    #[test]
    fn parse_mfra_with_tfras() {
        let tfra1 = make_box(b"tfra", &make_tfra_payload_minimal(1));
        let tfra2 = make_box(b"tfra", &make_tfra_payload_minimal(2));
        let mfro = make_box(b"mfro", &make_mfro_payload(2048));

        let mut payload = Vec::new();
        payload.extend_from_slice(&tfra1);
        payload.extend_from_slice(&tfra2);
        payload.extend_from_slice(&mfro);

        let mfra = MfraBoxView::decode(&payload).unwrap();

        assert_eq!(mfra.mfro().unwrap().size, 2048);

        let tfras: Vec<_> = mfra.tfras().collect();
        assert_eq!(tfras.len(), 2);
        assert_eq!(tfras[0].as_ref().unwrap().track_id, 1);
        assert_eq!(tfras[1].as_ref().unwrap().track_id, 2);
    }

    #[test]
    fn parse_mfra_missing_mfro() {
        let tfra = make_box(b"tfra", &make_tfra_payload_minimal(1));

        let mfra = MfraBoxView::decode(&tfra).unwrap();
        let result = mfra.mfro();

        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::BoxMissing { .. }));
        }
    }

    #[test]
    fn try_from_box_view_success() {
        let mfro = make_box(b"mfro", &make_mfro_payload(512));

        let mut box_data = Vec::new();
        let size = 8 + mfro.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"mfra");
        box_data.extend_from_slice(&mfro);

        let raw = RawBoxRef::parse(&box_data).unwrap();
        let mfra = MfraBoxView::try_from(raw.payload()).unwrap();

        assert_eq!(mfra.mfro().unwrap().size, 512);
    }

    #[cfg(feature = "alloc")]
    mod alloc_tests {
        use super::*;

        #[test]
        fn mfra_box_from_view() {
            let tfra = make_box(b"tfra", &make_tfra_payload_minimal(3));
            let mfro = make_box(b"mfro", &make_mfro_payload(4096));

            let mut payload = Vec::new();
            payload.extend_from_slice(&tfra);
            payload.extend_from_slice(&mfro);

            let view = MfraBoxView::decode(&payload).unwrap();
            let owned = MfraBox::try_from(&view).unwrap();

            assert_eq!(owned.mfro.size, 4096);
            assert_eq!(owned.tfras.len(), 1);
            assert_eq!(owned.tfras[0].track_id, 3);
        }

        #[test]
        fn mfra_box_parse() {
            let mfro = make_box(b"mfro", &make_mfro_payload(8192));

            let owned = MfraBox::decode(&mfro).unwrap();

            assert_eq!(owned.mfro.size, 8192);
            assert_eq!(owned.tfras.len(), 0);
        }
    }
}
