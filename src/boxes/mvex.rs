use crate::cursor::ReadCursor;

use crate::RawBoxRef;
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
                let mehd = MehdBox::parse(child.payload())?;
                return Ok(Some(mehd));
            }
        }
        Ok(None)
    }

    /// Returns an iterator over the Track Extends Boxes (`trex`) contained in this `MvexBoxView`.
    pub fn trexs(&self) -> impl Iterator<Item = Result<TrexBox>> + 'a {
        self.children().filter_map(|child| match child {
            Ok(view) if view.boxtype() == BoxType::TREX => Some(TrexBox::parse(view.payload())),
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

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<MvexBoxView<'a>> {
        let payload = cur.take(cur.remaining())?;
        Ok(MvexBoxView { payload })
    }

    /// Parses a `MvexBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<MvexBoxView<'a>> {
        let mut cursor = ReadCursor::new(payload);
        MvexBoxView::parse_in(&mut cursor)
    }
}

impl<'a> TryFrom<RawBoxRef<'a>> for MvexBoxView<'a> {
    type Error = Error;

    fn try_from(value: RawBoxRef<'a>) -> Result<Self> {
        if value.boxtype() != BoxType::MVEX {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::MVEX,
                found: value.boxtype(),
            }));
        }

        MvexBoxView::parse(value.payload())
    }
}

#[cfg(feature = "alloc")]
pub use owned::MvexBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use crate::BoxFrameMut;
    use crate::base::frame::write_box_in;

    use super::*;
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
        /// Constructs a `MvexBox` from a `MvexBoxView`.
        pub fn from_view(view: &MvexBoxView<'_>) -> Result<MvexBox> {
            let mut mehd = None;
            let mut trexs = Vec::new();

            for child in view.children() {
                let child = child?;

                match child.boxtype() {
                    BoxType::MEHD if mehd.is_none() => {
                        mehd = Some(MehdBox::parse(child.payload())?);
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
                        trexs.push(TrexBox::parse(child.payload())?);
                    }
                    _ => continue,
                }
            }

            Ok(MvexBox { mehd, trexs })
        }

        /// Parses a `MvexBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<MvexBox> {
            let view = MvexBoxView::parse(payload)?;
            MvexBox::from_view(&view)
        }

        /// Finds a `TrexBox` by track ID.
        pub fn find_trex(&self, track_id: u32) -> Option<&TrexBox> {
            self.trexs.iter().find(|trex| trex.track_id == track_id)
        }

        /// Returns the size of the payload in bytes.
        pub fn size(&self) -> usize {
            let mut size = 0;
            if let Some(ref mehd) = self.mehd {
                size += BoxFrameMut::required_len(BoxType::MEHD, mehd.size());
            }
            for trex in &self.trexs {
                size += BoxFrameMut::required_len(BoxType::TREX, trex.size());
            }
            size
        }

        pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            if let Some(ref mehd) = self.mehd {
                write_box_in(cur, BoxType::MEHD, mehd.size(), |p| mehd.write(p))?;
            }

            for trex in &self.trexs {
                write_box_in(cur, BoxType::TREX, trex.size(), |p| trex.write(p))?;
            }

            if !cur.is_empty() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxSize {
                        reason: "Buffer larger than expected",
                        got: cur.remaining() as u64,
                    },
                    BoxType::MVEX,
                ));
            }

            Ok(())
        }

        /// Writes this `MvexBox` into the given payload.
        pub fn write(&self, payload: &mut [u8]) -> Result<()> {
            let mut cursor = WriteCursor::new(payload);
            self.write_in(&mut cursor)
        }
    }

    impl TryFrom<&MvexBoxView<'_>> for MvexBox {
        type Error = Error;

        fn try_from(value: &MvexBoxView<'_>) -> Result<Self> {
            MvexBox::from_view(value)
        }
    }

    impl TryFrom<RawBoxRef<'_>> for MvexBox {
        type Error = Error;

        fn try_from(value: RawBoxRef<'_>) -> Result<Self> {
            if value.boxtype() != BoxType::MVEX {
                return Err(Error::new(ErrorKind::MismatchedBoxType {
                    expected: BoxType::MVEX,
                    found: value.boxtype(),
                }));
            }

            let view = MvexBoxView::parse(value.payload())?;
            MvexBox::from_view(&view)
        }
    }
}

#[cfg(test)]
mod tests {
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
        let mvex = MvexBoxView::parse(&payload).unwrap();

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
        let mvex = MvexBoxView::parse(&payload).unwrap();

        assert!(mvex.mehd().unwrap().is_none());

        let trexs: Vec<_> = mvex.trexs().collect();
        assert_eq!(trexs.len(), 1);
    }

    #[test]
    fn parse_mvex_empty() {
        let payload = make_mvex_payload(false, &[]);
        let mvex = MvexBoxView::parse(&payload).unwrap();

        assert!(mvex.mehd().unwrap().is_none());
        assert_eq!(mvex.trexs().count(), 0);
    }

    #[test]
    fn parse_mvex_find_trex() {
        let payload = make_mvex_payload(false, &[1, 2, 3]);
        let mvex = MvexBoxView::parse(&payload).unwrap();

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

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = RawBoxRef::parse_in(&mut cursor).unwrap();
        let mvex = MvexBoxView::try_from(box_view).unwrap();

        assert!(mvex.mehd().unwrap().is_some());
    }

    #[test]
    fn try_from_box_view_wrong_type() {
        let payload = make_mvex_payload(false, &[1]);

        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"moov"); // Wrong type
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = RawBoxRef::parse_in(&mut cursor).unwrap();
        let result = MvexBoxView::try_from(box_view);

        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::MismatchedBoxType { .. }));
        }
    }

    #[cfg(feature = "alloc")]
    mod alloc_tests {
        use super::*;

        #[test]
        fn mvex_box_from_view() {
            let payload = make_mvex_payload(true, &[1, 2]);
            let view = MvexBoxView::parse(&payload).unwrap();
            let mvex_box = MvexBox::from_view(&view).unwrap();

            assert!(mvex_box.mehd.is_some());
            assert_eq!(mvex_box.mehd.unwrap().fragment_duration, 10000);
            assert_eq!(mvex_box.trexs.len(), 2);
            assert_eq!(mvex_box.trexs[0].track_id, 1);
            assert_eq!(mvex_box.trexs[1].track_id, 2);
        }

        #[test]
        fn mvex_box_parse() {
            let payload = make_mvex_payload(false, &[1, 2, 3]);
            let mvex_box = MvexBox::parse(&payload).unwrap();

            assert!(mvex_box.mehd.is_none());
            assert_eq!(mvex_box.trexs.len(), 3);
        }

        #[test]
        fn mvex_box_find_trex() {
            let payload = make_mvex_payload(false, &[1, 2]);
            let mvex_box = MvexBox::parse(&payload).unwrap();

            assert!(mvex_box.find_trex(1).is_some());
            assert!(mvex_box.find_trex(2).is_some());
            assert!(mvex_box.find_trex(3).is_none());
        }

        #[test]
        fn mvex_box_multiple_mehd_error() {
            let mut payload = Vec::new();
            payload.extend_from_slice(&make_box(b"mehd", &make_mehd_payload(1000)));
            payload.extend_from_slice(&make_box(b"mehd", &make_mehd_payload(2000)));

            let result = MvexBox::parse(&payload);
            assert!(result.is_err());
            if let Err(err) = result {
                assert!(matches!(err.kind(), ErrorKind::InvalidBoxField { .. }));
            }
        }
    }
}
