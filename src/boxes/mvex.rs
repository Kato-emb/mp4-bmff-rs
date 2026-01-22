use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxIter;
use crate::BoxType;
use crate::error::*;

use crate::boxes::MehdBox;
use crate::boxes::TrexBox;

/// A reference to a Movie Extends Box (`mvex`).
///
/// This box warns readers that there might be Movie Fragment Boxes in this file.
/// It contains default values for all tracks.
#[derive(Debug)]
pub struct MvexBoxView<'a> {
    payload: &'a [u8],
}

impl<'a> MvexBoxView<'a> {
    /// Returns an iterator over the child boxes of this `MvexBoxView`.
    pub fn children(&self) -> BoxIter<'a> {
        BoxIter::new(self.payload)
    }

    /// Returns the Movie Extends Header Box (`mehd`) if present.
    pub fn mehd(&self) -> Result<Option<MehdBox>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::MEHD {
                let mehd = MehdBox::decode(child.payload())?;
                return Ok(Some(mehd));
            }
        }
        Ok(None)
    }

    /// Returns an iterator over the Track Extends Boxes (`trex`) contained in this `MvexBoxView`.
    pub fn trexs(&self) -> impl Iterator<Item = Result<TrexBox>> + 'a {
        self.children().filter_map(|child| match child {
            Ok(view) if view.boxtype() == BoxType::TREX => Some(TrexBox::decode(view.payload())),
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
    }

    /// Finds a `TrexBox` by track ID.
    pub fn find_trex(&self, track_id: u32) -> Result<Option<TrexBox>> {
        for result in self.trexs() {
            let trex = result?;
            if trex.track_id == track_id {
                return Ok(Some(trex));
            }
        }
        Ok(None)
    }
}

impl BoxCodec for MvexBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::MVEX
    }
}

impl<'de> BoxDecode<'de> for MvexBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        Ok(MvexBoxView { payload: bytes })
    }
}

impl<'a> TryFrom<&'a [u8]> for MvexBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        MvexBoxView::decode(value)
    }
}

#[cfg(feature = "alloc")]
pub use owned::MvexBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use crate::base::writer::write_box_in;

    use super::*;
    use crate::BoxEncode;
    use crate::cursor::WriteCursor;

    /// An owned Movie Extends Box (`mvex`).
    #[derive(Debug, Clone)]
    pub struct MvexBox {
        /// The Movie Extends Header Box (`mehd`), if present.
        pub mehd: Option<MehdBox>,
        /// The Track Extends Boxes (`trex`).
        pub trexs: Vec<TrexBox>,
    }

    impl MvexBox {
        /// Finds a `TrexBox` by track ID.
        pub fn find_trex(&self, track_id: u32) -> Option<&TrexBox> {
            self.trexs.iter().find(|trex| trex.track_id == track_id)
        }
    }

    impl TryFrom<&MvexBoxView<'_>> for MvexBox {
        type Error = Error;

        fn try_from(view: &MvexBoxView<'_>) -> Result<Self> {
            let mut mehd = None;
            let mut trexs = Vec::new();

            for child in view.children() {
                let child = child?;

                match child.boxtype() {
                    BoxType::MEHD if mehd.is_none() => {
                        mehd = Some(MehdBox::decode(child.payload())?);
                    }
                    BoxType::MEHD => {
                        return Err(Error::in_box(
                            ErrorKind::InvalidBoxField {
                                field: "Movie Extends Header Box",
                                reason: "multiple mehd boxes found",
                            },
                            BoxType::MVEX,
                        ));
                    }
                    BoxType::TREX => {
                        trexs.push(TrexBox::decode(child.payload())?);
                    }
                    _ => continue,
                }
            }

            Ok(MvexBox { mehd, trexs })
        }
    }

    impl BoxCodec for MvexBox {
        fn boxtype(&self) -> BoxType {
            BoxType::MVEX
        }
    }

    impl BoxDecode<'_> for MvexBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = MvexBoxView::decode(bytes)?;
            MvexBox::try_from(&view)
        }
    }

    impl BoxEncode for MvexBox {
        fn encode(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            if let Some(ref mehd) = self.mehd {
                write_box_in(&mut cur, mehd)?;
            }

            for trex in &self.trexs {
                write_box_in(&mut cur, trex)?;
            }

            Ok(cur.position())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::RawBoxRef;

    use super::*;

    fn make_box(boxtype: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let size = (8 + payload.len()) as u32;
        let mut data = Vec::new();
        data.extend_from_slice(&size.to_be_bytes());
        data.extend_from_slice(boxtype);
        data.extend_from_slice(payload);
        data
    }

    fn make_full_box_header(version: u8, flags: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(version);
        data.extend_from_slice(&flags.to_be_bytes()[1..4]);
        data
    }

    fn make_mehd_payload(fragment_duration: u32) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(0, 0));
        payload.extend_from_slice(&fragment_duration.to_be_bytes());
        payload
    }

    fn make_trex_payload(track_id: u32) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(0, 0));
        payload.extend_from_slice(&track_id.to_be_bytes());
        payload.extend_from_slice(&1u32.to_be_bytes()); // default_sample_description_index
        payload.extend_from_slice(&1024u32.to_be_bytes()); // default_sample_duration
        payload.extend_from_slice(&512u32.to_be_bytes()); // default_sample_size
        payload.extend_from_slice(&0u32.to_be_bytes()); // default_sample_flags
        payload
    }

    fn make_mvex_payload(with_mehd: bool, track_ids: &[u32]) -> Vec<u8> {
        let mut data = Vec::new();
        if with_mehd {
            data.extend_from_slice(&make_box(b"mehd", &make_mehd_payload(10000)));
        }
        for &track_id in track_ids {
            data.extend_from_slice(&make_box(b"trex", &make_trex_payload(track_id)));
        }
        data
    }

    #[test]
    fn parse_mvex_with_mehd() {
        let payload = make_mvex_payload(true, &[1, 2]);
        let mvex = MvexBoxView::decode(&payload).unwrap();

        let mehd = mvex.mehd().unwrap().unwrap();
        assert_eq!(mehd.fragment_duration, 10000);

        let trexs: Vec<_> = mvex.trexs().collect();
        assert_eq!(trexs.len(), 2);
        assert_eq!(trexs[0].as_ref().unwrap().track_id, 1);
        assert_eq!(trexs[1].as_ref().unwrap().track_id, 2);
    }

    #[test]
    fn parse_mvex_without_mehd() {
        let payload = make_mvex_payload(false, &[1]);
        let mvex = MvexBoxView::decode(&payload).unwrap();

        assert!(mvex.mehd().unwrap().is_none());

        let trexs: Vec<_> = mvex.trexs().collect();
        assert_eq!(trexs.len(), 1);
    }

    #[test]
    fn parse_mvex_empty() {
        let payload = make_mvex_payload(false, &[]);
        let mvex = MvexBoxView::decode(&payload).unwrap();

        assert!(mvex.mehd().unwrap().is_none());
        assert_eq!(mvex.trexs().count(), 0);
    }

    #[test]
    fn parse_mvex_find_trex() {
        let payload = make_mvex_payload(false, &[1, 2, 3]);
        let mvex = MvexBoxView::decode(&payload).unwrap();

        let trex1 = mvex.find_trex(1).unwrap().unwrap();
        assert_eq!(trex1.track_id, 1);

        let trex2 = mvex.find_trex(2).unwrap().unwrap();
        assert_eq!(trex2.track_id, 2);

        let trex3 = mvex.find_trex(3).unwrap().unwrap();
        assert_eq!(trex3.track_id, 3);

        assert!(mvex.find_trex(4).unwrap().is_none());
    }

    #[test]
    fn try_from_box_view_success() {
        let payload = make_mvex_payload(true, &[1]);

        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"mvex");
        box_data.extend_from_slice(&payload);

        let raw = RawBoxRef::parse(&box_data).unwrap();
        let mvex = MvexBoxView::try_from(raw.payload()).unwrap();

        assert!(mvex.mehd().unwrap().is_some());
    }

    #[cfg(feature = "alloc")]
    mod alloc_tests {
        use super::*;

        #[test]
        fn mvex_box_parse() {
            let payload = make_mvex_payload(false, &[1, 2, 3]);
            let mvex_box = MvexBox::decode(&payload).unwrap();

            assert!(mvex_box.mehd.is_none());
            assert_eq!(mvex_box.trexs.len(), 3);
        }

        #[test]
        fn mvex_box_find_trex() {
            let payload = make_mvex_payload(false, &[1, 2]);
            let mvex_box = MvexBox::decode(&payload).unwrap();

            assert!(mvex_box.find_trex(1).is_some());
            assert!(mvex_box.find_trex(2).is_some());
            assert!(mvex_box.find_trex(3).is_none());
        }

        #[test]
        fn mvex_box_multiple_mehd_error() {
            let mut payload = Vec::new();
            payload.extend_from_slice(&make_box(b"mehd", &make_mehd_payload(1000)));
            payload.extend_from_slice(&make_box(b"mehd", &make_mehd_payload(2000)));

            let result = MvexBox::decode(&payload);
            assert!(result.is_err());
            if let Err(err) = result {
                assert!(matches!(err.kind(), ErrorKind::InvalidBoxField { .. }));
            }
        }
    }
}
