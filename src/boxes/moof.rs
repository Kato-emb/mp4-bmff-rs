use crate::cursor::ReadCursor;

use crate::BoxFrame;
use crate::BoxIter;
use crate::BoxType;
use crate::error::*;

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
                return MfhdBox::parse(child.payload());
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
            Ok(view) if view.boxtype() == BoxType::TRAF => Some(TrafBoxView::parse(view.payload())),
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<MoofBoxView<'a>> {
        let payload = cur.take(cur.remaining())?;
        Ok(MoofBoxView { payload })
    }

    /// Parses a `MoofBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<MoofBoxView<'a>> {
        let mut cursor = ReadCursor::new(payload);
        MoofBoxView::parse_in(&mut cursor)
    }
}

impl<'a> TryFrom<BoxFrame<'a>> for MoofBoxView<'a> {
    type Error = Error;

    fn try_from(value: BoxFrame<'a>) -> Result<Self> {
        if value.boxtype() != BoxType::MOOF {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::MOOF,
                found: value.boxtype(),
            }));
        }

        MoofBoxView::parse(value.payload())
    }
}

#[cfg(feature = "alloc")]
pub use owned::MoofBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use crate::cursor::WriteCursor;

    use super::*;
    use crate::BoxFrameMut;
    use crate::base::frame::write_box_in;

    use crate::boxes::TrafBox;

    /// An owned Movie Fragment Box (`moof`).
    #[derive(Debug, Clone)]
    pub struct MoofBox {
        /// The Movie Fragment Header Box (`mfhd`).
        pub mfhd: MfhdBox,
        /// The Track Fragment Boxes (`traf`).
        pub trafs: Vec<TrafBox>,
    }

    impl MoofBox {
        /// Creates a `MoofBox` from a `MoofBoxView`.
        pub fn from_view(view: &MoofBoxView<'_>) -> Result<MoofBox> {
            let mfhd = view.mfhd()?;

            let mut trafs = Vec::new();
            for traf_result in view.trafs() {
                let traf_view = traf_result?;
                trafs.push(TrafBox::from_view(&traf_view)?);
            }

            Ok(MoofBox { mfhd, trafs })
        }

        /// Parses a `MoofBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<MoofBox> {
            let view = MoofBoxView::parse(payload)?;
            MoofBox::from_view(&view)
        }

        /// Returns the size of the payload in bytes.
        pub fn size(&self) -> usize {
            let mut size = 0;
            size += BoxFrameMut::required_len(BoxType::MFHD, self.mfhd.size());
            for traf in &self.trafs {
                size += BoxFrameMut::required_len(BoxType::TRAF, traf.size());
            }
            size
        }

        pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            write_box_in(cur, BoxType::MFHD, self.mfhd.size(), |p| self.mfhd.write(p))?;

            for traf in &self.trafs {
                write_box_in(cur, BoxType::TRAF, traf.size(), |p| traf.write(p))?;
            }

            if !cur.is_empty() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxSize {
                        reason: "Buffer larger than expected",
                        got: cur.remaining() as u64,
                    },
                    BoxType::MOOF,
                ));
            }

            Ok(())
        }

        /// Writes this `MoofBox` into the given payload.
        pub fn write(&self, payload: &mut [u8]) -> Result<()> {
            let mut cursor = WriteCursor::new(payload);
            self.write_in(&mut cursor)
        }
    }

    impl TryFrom<&MoofBoxView<'_>> for MoofBox {
        type Error = Error;

        fn try_from(value: &MoofBoxView<'_>) -> Result<Self> {
            MoofBox::from_view(value)
        }
    }

    impl TryFrom<BoxFrame<'_>> for MoofBox {
        type Error = Error;

        fn try_from(value: BoxFrame<'_>) -> Result<Self> {
            let view = MoofBoxView::try_from(value)?;
            MoofBox::from_view(&view)
        }
    }
}

#[cfg(test)]
mod tests {
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

        let moof = MoofBoxView::parse(&mfhd).unwrap();

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

        let moof = MoofBoxView::parse(&payload).unwrap();

        assert_eq!(moof.mfhd().unwrap().sequence_number, 5);

        let trafs: Vec<_> = moof.trafs().collect();
        assert_eq!(trafs.len(), 2);
        assert_eq!(trafs[0].as_ref().unwrap().tfhd().unwrap().track_id, 1);
        assert_eq!(trafs[1].as_ref().unwrap().tfhd().unwrap().track_id, 2);
    }

    #[test]
    fn parse_moof_missing_mfhd() {
        let traf = make_box(b"traf", &make_traf_payload(1));

        let moof = MoofBoxView::parse(&traf).unwrap();
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

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxFrame::parse_in(&mut cursor).unwrap();
        let moof = MoofBoxView::try_from(box_view).unwrap();

        assert_eq!(moof.mfhd().unwrap().sequence_number, 1);
    }

    #[test]
    fn try_from_box_view_wrong_type() {
        let mfhd = make_box(b"mfhd", &make_mfhd_payload(1));

        let mut box_data = Vec::new();
        let size = 8 + mfhd.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"moov");
        box_data.extend_from_slice(&mfhd);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxFrame::parse_in(&mut cursor).unwrap();
        let result = MoofBoxView::try_from(box_view);

        assert!(result.is_err());
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

            let view = MoofBoxView::parse(&payload).unwrap();
            let owned = MoofBox::from_view(&view).unwrap();

            assert_eq!(owned.mfhd.sequence_number, 10);
            assert_eq!(owned.trafs.len(), 1);
            assert_eq!(owned.trafs[0].tfhd.track_id, 3);
        }

        #[test]
        fn moof_box_parse() {
            let mfhd = make_box(b"mfhd", &make_mfhd_payload(100));

            let owned = MoofBox::parse(&mfhd).unwrap();

            assert_eq!(owned.mfhd.sequence_number, 100);
            assert_eq!(owned.trafs.len(), 0);
        }
    }
}
