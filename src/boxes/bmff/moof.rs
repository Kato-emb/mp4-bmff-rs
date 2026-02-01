use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

use super::MfhdBox;
use super::MvexBoxView;
use super::TrafBoxView;

/// A reference to a Movie Fragment Box (`moof`).
#[derive(Debug)]
pub struct MoofBoxView<'a> {
    content: &'a [u8],
}

impl<'a> MoofBoxView<'a> {
    /// Returns an iterator over the child boxes of this `moof` box.
    pub fn boxes(&self) -> BoxIter<'a> {
        BoxIter::new(self.content)
    }

    /// Returns the Movie Fragment Header Box (`mfhd`) contained in this `moof` box.
    pub fn mfhd(&self) -> Result<MfhdBox> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::MFHD {
                let mfhd = MfhdBox::decode(b.payload())?;
                return Ok(mfhd);
            }
        }

        Err(Error::in_box(
            ErrorKind::BoxMissing {
                required: BoxType::MFHD,
            },
            BoxType::MOOF,
        ))
    }

    /// Returns an iterator over the Track Fragment Boxes (`traf`) contained in this `moof` box.
    pub fn trafs(&self) -> impl Iterator<Item = Result<TrafBoxView<'a>>> + 'a {
        self.boxes().filter_map(|result| match result {
            Ok(rawbox) if rawbox.boxtype() == BoxType::TRAF => {
                Some(TrafBoxView::decode(rawbox.into_payload()))
            }
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
    }

    /// Returns the Movie Extends Box (`mvex`) contained in this `moof` box if present.
    pub fn mvex(&self) -> Result<Option<MvexBoxView<'a>>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::MVEX {
                let mvex = MvexBoxView::decode(b.into_payload())?;
                return Ok(Some(mvex));
            }
        }

        Ok(None)
    }
}

impl BoxCodec for MoofBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::MOOF
    }
}

impl<'de> BoxDecode<'de> for MoofBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        Ok(MoofBoxView { content: bytes })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use super::*;
    use crate::BoxEncode;

    use crate::codec::boxed_len;
    use crate::codec::write_box_in;
    use crate::cursor::WriteCursor;

    use crate::boxes::bmff::MvexBox;
    use crate::boxes::bmff::TrafBox;

    /// An owned Movie Fragment Box (`moof`).
    #[derive(Debug, Clone)]
    pub struct MoofBox {
        /// Movie Fragment Header Box (`mfhd`).
        pub mfhd: MfhdBox,
        /// Track Fragment Boxes (`traf`).
        pub trafs: Vec<TrafBox>,
        /// Movie Extends Box (`mvex`), if present.
        pub mvex: Option<MvexBox>,
    }

    impl TryFrom<&MoofBoxView<'_>> for MoofBox {
        type Error = Error;

        fn try_from(view: &MoofBoxView<'_>) -> Result<Self> {
            let mut mfhd = None;
            let mut trafs = Vec::new();
            let mut mvex = None;

            for result in view.boxes() {
                let rawbox = result?;

                match rawbox.boxtype() {
                    BoxType::MFHD => {
                        if mfhd.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::MFHD,
                                },
                                BoxType::MOOF,
                            ));
                        }
                        let mfhd_box = MfhdBox::decode(rawbox.payload())?;
                        mfhd = Some(mfhd_box);
                    }
                    BoxType::TRAF => {
                        let traf_box = TrafBox::try_from(&TrafBoxView::decode(rawbox.payload())?)?;
                        trafs.push(traf_box);
                    }
                    BoxType::MVEX => {
                        if mvex.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::MVEX,
                                },
                                BoxType::MOOF,
                            ));
                        }
                        let mvex_box = MvexBox::try_from(&MvexBoxView::decode(rawbox.payload())?)?;
                        mvex = Some(mvex_box);
                    }
                    _ => {}
                }
            }

            let mfhd = mfhd.ok_or_else(|| {
                Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::MFHD,
                    },
                    BoxType::MOOF,
                )
            })?;

            Ok(MoofBox { mfhd, trafs, mvex })
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
            let mut len = boxed_len(&self.mfhd);
            for traf in &self.trafs {
                len += boxed_len(traf);
            }
            if let Some(mvex) = &self.mvex {
                len += boxed_len(mvex);
            }
            len
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            write_box_in(&mut cur, &self.mfhd)?;
            for traf in &self.trafs {
                write_box_in(&mut cur, traf)?;
            }
            if let Some(mvex) = &self.mvex {
                write_box_in(&mut cur, mvex)?;
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

    fn raw_data_empty() -> [u8; 0] {
        []
    }

    fn raw_data_mfhd_only() -> [u8; 16] {
        [
            // mfhd box
            0x00, 0x00, 0x00, 0x10, // size = 16
            b'm', b'f', b'h', b'd', // type = "mfhd"
            0x00,             // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x01, // sequence_number = 1
        ]
    }

    fn raw_data_mfhd_and_traf() -> [u8; 40] {
        [
            // mfhd box
            0x00, 0x00, 0x00, 0x10, // size = 16
            b'm', b'f', b'h', b'd', // type = "mfhd"
            0x00,             // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x02, // sequence_number = 2
            // traf box (contains tfhd)
            0x00, 0x00, 0x00, 0x18, // size = 24
            b't', b'r', b'a', b'f', // type = "traf"
            // tfhd box inside traf
            0x00, 0x00, 0x00, 0x10, // size = 16
            b't', b'f', b'h', b'd', // type = "tfhd"
            0x00,             // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x01, // track_id = 1
        ]
    }

    #[test]
    fn test_moof_box_view_decode_empty() {
        let data = raw_data_empty();
        let moof = MoofBoxView::decode(&data).unwrap();

        assert_eq!(moof.boxes().count(), 0);
        assert_eq!(moof.trafs().count(), 0);
    }

    #[test]
    fn test_moof_box_view_missing_required_mfhd() {
        let data = raw_data_empty();
        let moof = MoofBoxView::decode(&data).unwrap();

        let result = moof.mfhd();
        assert!(result.is_err());
    }

    #[test]
    fn test_moof_box_view_mfhd_only() {
        let data = raw_data_mfhd_only();
        let moof = MoofBoxView::decode(&data).unwrap();

        let mfhd = moof.mfhd().unwrap();
        assert_eq!(mfhd.sequence_number, 1);

        assert_eq!(moof.trafs().count(), 0);
        assert!(moof.mvex().unwrap().is_none());
    }

    #[test]
    fn test_moof_box_view_mfhd_and_traf() {
        let data = raw_data_mfhd_and_traf();
        let moof = MoofBoxView::decode(&data).unwrap();

        let mfhd = moof.mfhd().unwrap();
        assert_eq!(mfhd.sequence_number, 2);

        let trafs: Vec<_> = moof.trafs().collect();
        assert_eq!(trafs.len(), 1);

        let traf = trafs[0].as_ref().unwrap();
        let tfhd = traf.tfhd().unwrap();
        assert_eq!(tfhd.track_id, 1);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_moof_box_try_from() {
        let data = raw_data_mfhd_only();
        let view = MoofBoxView::decode(&data).unwrap();
        let owned = MoofBox::try_from(&view).unwrap();

        assert_eq!(owned.mfhd.sequence_number, 1);
        assert!(owned.trafs.is_empty());
        assert!(owned.mvex.is_none());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_moof_box_try_from_missing_mfhd() {
        let data = raw_data_empty();
        let view = MoofBoxView::decode(&data).unwrap();
        let result = MoofBox::try_from(&view);

        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_moof_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_data_mfhd_only();
        let moof = MoofBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; moof.encoded_len()];
        let len = moof.encode_into(&mut encoded).unwrap();

        assert_eq!(len, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }
}
