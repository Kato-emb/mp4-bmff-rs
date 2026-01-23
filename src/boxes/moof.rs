use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

use crate::boxes::MfhdBox;
use crate::boxes::TrafBoxView;

/// A reference to a Movie Fragment Box (`moof`).
///
/// This box contains the metadata for a movie fragment.
#[derive(Debug)]
pub struct MoofBoxView<'a> {
    payload: &'a [u8],
}

impl<'a> MoofBoxView<'a> {
    /// Returns an iterator over the child boxes of this `MoofBoxView`.
    pub fn children(&self) -> BoxIter<'a> {
        BoxIter::new(self.payload)
    }

    /// Returns the Movie Fragment Header Box (`mfhd`).
    pub fn mfhd(&self) -> Result<MfhdBox> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::MFHD {
                return MfhdBox::decode(child.payload());
            }
        }

        Err(Error::in_box(
            ErrorKind::BoxMissing {
                required: BoxType::MFHD,
            },
            BoxType::MOOF,
        ))
    }

    /// Returns an iterator over the Track Fragment Boxes (`traf`).
    pub fn trafs(&self) -> impl Iterator<Item = Result<TrafBoxView<'a>>> + 'a {
        self.children().filter_map(|result| match result {
            Ok(view) if view.boxtype() == BoxType::TRAF => {
                Some(TrafBoxView::decode(view.into_payload()))
            }
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
    }
}

impl BoxCodec for MoofBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::MOOF
    }
}

impl<'de> BoxDecode<'de> for MoofBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        Ok(MoofBoxView { payload: bytes })
    }
}

impl<'a> TryFrom<&'a [u8]> for MoofBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        MoofBoxView::decode(value)
    }
}

#[cfg(feature = "alloc")]
pub use owned::MoofBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use crate::cursor::WriteCursor;

    use super::*;
    use crate::codec::boxed_len;
    use crate::codec::write_box_in;

    use crate::BoxCodec;
    use crate::BoxDecode;
    use crate::BoxEncode;
    use crate::boxes::TrafBox;

    /// An owned Movie Fragment Box (`moof`).
    #[derive(Debug, Clone)]
    pub struct MoofBox {
        /// The Movie Fragment Header Box (`mfhd`).
        pub mfhd: MfhdBox,
        /// The Track Fragment Boxes (`traf`).
        pub trafs: Vec<TrafBox>,
    }

    impl TryFrom<&MoofBoxView<'_>> for MoofBox {
        type Error = Error;

        fn try_from(view: &MoofBoxView<'_>) -> Result<Self> {
            let mfhd = view.mfhd()?;

            let mut trafs = Vec::new();
            for traf_result in view.trafs() {
                let traf_view = traf_result?;
                trafs.push(TrafBox::try_from(&traf_view)?);
            }

            Ok(MoofBox { mfhd, trafs })
        }
    }

    impl BoxCodec for MoofBox {
        fn boxtype(&self) -> BoxType {
            BoxType::MOOF
        }
    }

    impl BoxDecode<'_> for MoofBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = MoofBoxView::decode(bytes)?;
            MoofBox::try_from(&view)
        }
    }

    impl BoxEncode for MoofBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            boxed_len(&self.mfhd) // mfhd
            + self.trafs.iter().map(boxed_len).sum::<usize>() // trafs
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            write_box_in(&mut cur, &self.mfhd)?;

            for traf in &self.trafs {
                write_box_in(&mut cur, traf)?;
            }

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

    fn make_mfhd_payload(sequence_number: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(0); // version
        data.extend_from_slice(&[0, 0, 0]); // flags
        data.extend_from_slice(&sequence_number.to_be_bytes());
        data
    }

    fn make_tfhd_payload(track_id: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(0); // version
        data.extend_from_slice(&[0, 0, 0]); // flags
        data.extend_from_slice(&track_id.to_be_bytes());
        data
    }

    fn make_traf_payload(track_id: u32) -> Vec<u8> {
        make_box(b"tfhd", &make_tfhd_payload(track_id))
    }

    #[test]
    fn parse_moof_minimal() {
        let mfhd = make_box(b"mfhd", &make_mfhd_payload(1));

        let moof = MoofBoxView::decode(&mfhd).unwrap();

        let mfhd_box = moof.mfhd().unwrap();
        assert_eq!(mfhd_box.sequence_number, 1);
        assert_eq!(moof.trafs().count(), 0);
    }

    #[test]
    fn parse_moof_with_trafs() {
        let mfhd = make_box(b"mfhd", &make_mfhd_payload(5));
        let traf1 = make_box(b"traf", &make_traf_payload(1));
        let traf2 = make_box(b"traf", &make_traf_payload(2));

        let mut payload = Vec::new();
        payload.extend_from_slice(&mfhd);
        payload.extend_from_slice(&traf1);
        payload.extend_from_slice(&traf2);

        let moof = MoofBoxView::decode(&payload).unwrap();

        assert_eq!(moof.mfhd().unwrap().sequence_number, 5);

        let trafs: Vec<_> = moof.trafs().collect();
        assert_eq!(trafs.len(), 2);
        assert_eq!(trafs[0].as_ref().unwrap().tfhd().unwrap().track_id, 1);
        assert_eq!(trafs[1].as_ref().unwrap().tfhd().unwrap().track_id, 2);
    }

    #[test]
    fn parse_moof_missing_mfhd() {
        let traf = make_box(b"traf", &make_traf_payload(1));

        let moof = MoofBoxView::decode(&traf).unwrap();
        let result = moof.mfhd();

        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::BoxMissing { .. }));
        }
    }

    #[test]
    fn try_from_box_view_success() {
        let mfhd = make_box(b"mfhd", &make_mfhd_payload(1));

        let mut box_data = Vec::new();
        let size = 8 + mfhd.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"moof");
        box_data.extend_from_slice(&mfhd);

        let raw = RawBoxRef::parse(&box_data).unwrap();
        let moof = MoofBoxView::try_from(raw.payload()).unwrap();

        assert_eq!(moof.mfhd().unwrap().sequence_number, 1);
    }

    #[cfg(feature = "alloc")]
    mod alloc_tests {
        use super::*;

        #[test]
        fn moof_box_from_view() {
            let mfhd = make_box(b"mfhd", &make_mfhd_payload(10));
            let traf = make_box(b"traf", &make_traf_payload(3));

            let mut payload = Vec::new();
            payload.extend_from_slice(&mfhd);
            payload.extend_from_slice(&traf);

            let moof_view = MoofBoxView::decode(&payload).unwrap();
            let owned = MoofBox::try_from(&moof_view).unwrap();

            assert_eq!(owned.mfhd.sequence_number, 10);
            assert_eq!(owned.trafs.len(), 1);
            assert_eq!(owned.trafs[0].tfhd.track_id, 3);
        }
    }
}
