use crate::cursor::ReadCursor;

use crate::BoxFrame;
use crate::BoxIter;
use crate::BoxType;
use crate::error::*;

use crate::boxes::SbgpBoxView;
use crate::boxes::TfdtBox;
use crate::boxes::TfhdBox;
use crate::boxes::TrunBoxView;

/// A reference to a Track Fragment Box (`traf`).
///
/// This box contains information for a single track fragment.
#[derive(Debug)]
pub struct TrafBoxView<'a> {
    payload: &'a [u8],
}

impl<'a> TrafBoxView<'a> {
    /// Returns an iterator over the child boxes of this `TrafBoxView`.
    pub fn children(&self) -> BoxIter<'a> {
        BoxIter::new(self.payload)
    }

    /// Returns the Track Fragment Header Box (`tfhd`).
    pub fn tfhd(&self) -> Result<TfhdBox> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::TFHD {
                return TfhdBox::parse(child.payload());
            }
        }

        Err(Error::in_box(
            ErrorKind::BoxMissing {
                required: BoxType::TFHD,
            },
            BoxType::TRAF,
        ))
    }

    /// Returns the Track Fragment Decode Time Box (`tfdt`) if present.
    pub fn tfdt(&self) -> Result<Option<TfdtBox>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::TFDT {
                return Ok(Some(TfdtBox::parse(child.payload())?));
            }
        }

        Ok(None)
    }

    /// Returns an iterator over the Track Run Boxes (`trun`).
    pub fn truns(&self) -> impl Iterator<Item = Result<TrunBoxView<'a>>> + 'a {
        self.children().filter_map(|result| match result {
            Ok(view) if view.boxtype() == BoxType::TRUN => Some(TrunBoxView::parse(view.payload())),
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
    }

    /// Returns an iterator over the Sample to Group Boxes (`sbgp`).
    pub fn sbgps(&self) -> impl Iterator<Item = Result<SbgpBoxView<'a>>> + 'a {
        self.children().filter_map(|result| match result {
            Ok(view) if view.boxtype() == BoxType::SBGP => Some(SbgpBoxView::parse(view.payload())),
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<TrafBoxView<'a>> {
        let payload = cur.take(cur.remaining())?;
        Ok(TrafBoxView { payload })
    }

    /// Parses a `TrafBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<TrafBoxView<'a>> {
        let mut cursor = ReadCursor::new(payload);
        TrafBoxView::parse_in(&mut cursor)
    }
}

impl<'a> TryFrom<BoxFrame<'a>> for TrafBoxView<'a> {
    type Error = Error;

    fn try_from(value: BoxFrame<'a>) -> Result<Self> {
        if value.boxtype() != BoxType::TRAF {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::TRAF,
                found: value.boxtype(),
            }));
        }

        TrafBoxView::parse(value.payload())
    }
}

#[cfg(feature = "alloc")]
pub use owned::TrafBox;

#[cfg(feature = "alloc")]
mod owned {
    extern crate alloc;
    use alloc::vec::Vec;

    use crate::BoxFrameMut;
    use crate::cursor::WriteCursor;

    use super::*;
    use crate::boxes::SbgpBox;
    use crate::boxes::TrunBox;

    /// An owned Track Fragment Box (`traf`).
    #[derive(Debug, Clone)]
    pub struct TrafBox {
        /// The Track Fragment Header Box (`tfhd`).
        pub tfhd: TfhdBox,
        /// The Track Fragment Decode Time Box (`tfdt`), if present.
        pub tfdt: Option<TfdtBox>,
        /// The Track Run Boxes (`trun`).
        pub truns: Vec<TrunBox>,
        /// The Sample to Group Boxes (`sbgp`).
        pub sbgps: Vec<SbgpBox>,
    }

    impl TrafBox {
        /// Creates a `TrafBox` from a `TrafBoxView`.
        pub fn from_view(view: &TrafBoxView<'_>) -> Result<TrafBox> {
            let tfhd = view.tfhd()?;
            let tfdt = view.tfdt()?;

            let mut truns = Vec::new();
            for trun_result in view.truns() {
                let trun_view = trun_result?;
                truns.push(TrunBox::from_view(&trun_view)?);
            }

            let mut sbgps = Vec::new();
            for sbgp_result in view.sbgps() {
                let sbgp_view = sbgp_result?;
                sbgps.push(SbgpBox::from_view(&sbgp_view)?);
            }

            Ok(TrafBox {
                tfhd,
                tfdt,
                truns,
                sbgps,
            })
        }

        /// Parses a `TrafBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<TrafBox> {
            let view = TrafBoxView::parse(payload)?;
            TrafBox::from_view(&view)
        }

        /// Returns the size of the payload in bytes.
        pub fn size(&self) -> usize {
            let mut size = 0;
            size += BoxFrameMut::required_len(BoxType::TFHD, self.tfhd.size());
            if let Some(ref tfdt) = self.tfdt {
                size += BoxFrameMut::required_len(BoxType::TFDT, tfdt.size());
            }
            for trun in &self.truns {
                size += BoxFrameMut::required_len(BoxType::TRUN, trun.size());
            }
            for sbgp in &self.sbgps {
                size += BoxFrameMut::required_len(BoxType::SBGP, sbgp.size());
            }
            size
        }

        fn write_box<F>(
            cur: &mut WriteCursor<'_>,
            boxtype: BoxType,
            payload_size: usize,
            write_payload: F,
        ) -> Result<()>
        where
            F: FnOnce(&mut [u8]) -> Result<()>,
        {
            let frame_size = BoxFrameMut::required_len(boxtype, payload_size);
            let buf = cur.take_mut(frame_size)?;
            let mut frame = BoxFrameMut::new(buf, boxtype, payload_size)?;
            write_payload(frame.payload_mut())?;
            Ok(())
        }

        pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            Self::write_box(cur, BoxType::TFHD, self.tfhd.size(), |p| self.tfhd.write(p))?;

            if let Some(ref tfdt) = self.tfdt {
                Self::write_box(cur, BoxType::TFDT, tfdt.size(), |p| tfdt.write(p))?;
            }

            for trun in &self.truns {
                Self::write_box(cur, BoxType::TRUN, trun.size(), |p| trun.write(p))?;
            }

            for sbgp in &self.sbgps {
                Self::write_box(cur, BoxType::SBGP, sbgp.size(), |p| sbgp.write(p))?;
            }

            if !cur.is_empty() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxSize {
                        reason: "Buffer larger than expected",
                        got: cur.remaining() as u64,
                    },
                    BoxType::TRAF,
                ));
            }

            Ok(())
        }

        /// Writes this `TrafBox` into the given payload.
        pub fn write(&self, payload: &mut [u8]) -> Result<()> {
            let mut cursor = WriteCursor::new(payload);
            self.write_in(&mut cursor)
        }
    }

    impl TryFrom<&TrafBoxView<'_>> for TrafBox {
        type Error = Error;

        fn try_from(value: &TrafBoxView<'_>) -> Result<Self> {
            TrafBox::from_view(value)
        }
    }

    impl TryFrom<BoxFrame<'_>> for TrafBox {
        type Error = Error;

        fn try_from(value: BoxFrame<'_>) -> Result<Self> {
            let view = TrafBoxView::try_from(value)?;
            TrafBox::from_view(&view)
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

    fn make_tfhd_payload(track_id: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(0); // version
        data.extend_from_slice(&[0, 0, 0]); // flags
        data.extend_from_slice(&track_id.to_be_bytes());
        data
    }

    fn make_tfdt_payload_v0(base_media_decode_time: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(0); // version
        data.extend_from_slice(&[0, 0, 0]); // flags
        data.extend_from_slice(&base_media_decode_time.to_be_bytes());
        data
    }

    fn make_trun_payload(sample_count: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(0); // version
        data.extend_from_slice(&[0, 0, 0]); // flags (no optional fields)
        data.extend_from_slice(&sample_count.to_be_bytes());
        data
    }

    #[test]
    fn parse_traf_minimal() {
        let tfhd = make_box(b"tfhd", &make_tfhd_payload(1));

        let mut payload = Vec::new();
        payload.extend_from_slice(&tfhd);

        let traf = TrafBoxView::parse(&payload).unwrap();

        let tfhd_box = traf.tfhd().unwrap();
        assert_eq!(tfhd_box.track_id, 1);

        assert!(traf.tfdt().unwrap().is_none());
        assert_eq!(traf.truns().count(), 0);
    }

    #[test]
    fn parse_traf_with_tfdt() {
        let tfhd = make_box(b"tfhd", &make_tfhd_payload(2));
        let tfdt = make_box(b"tfdt", &make_tfdt_payload_v0(1000));

        let mut payload = Vec::new();
        payload.extend_from_slice(&tfhd);
        payload.extend_from_slice(&tfdt);

        let traf = TrafBoxView::parse(&payload).unwrap();

        let tfdt_box = traf.tfdt().unwrap().unwrap();
        assert_eq!(tfdt_box.base_media_decode_time, 1000);
    }

    #[test]
    fn parse_traf_with_truns() {
        let tfhd = make_box(b"tfhd", &make_tfhd_payload(3));
        let trun1 = make_box(b"trun", &make_trun_payload(10));
        let trun2 = make_box(b"trun", &make_trun_payload(20));

        let mut payload = Vec::new();
        payload.extend_from_slice(&tfhd);
        payload.extend_from_slice(&trun1);
        payload.extend_from_slice(&trun2);

        let traf = TrafBoxView::parse(&payload).unwrap();

        let truns: Vec<_> = traf.truns().collect();
        assert_eq!(truns.len(), 2);
        assert_eq!(truns[0].as_ref().unwrap().sample_count, 10);
        assert_eq!(truns[1].as_ref().unwrap().sample_count, 20);
    }

    #[test]
    fn parse_traf_missing_tfhd() {
        let tfdt = make_box(b"tfdt", &make_tfdt_payload_v0(0));

        let traf = TrafBoxView::parse(&tfdt).unwrap();
        let result = traf.tfhd();

        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::BoxMissing { .. }));
        }
    }

    #[test]
    fn try_from_box_view_success() {
        let tfhd = make_box(b"tfhd", &make_tfhd_payload(1));

        let mut box_data = Vec::new();
        let size = 8 + tfhd.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"traf");
        box_data.extend_from_slice(&tfhd);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxFrame::parse_in(&mut cursor).unwrap();
        let traf = TrafBoxView::try_from(box_view).unwrap();

        assert_eq!(traf.tfhd().unwrap().track_id, 1);
    }

    #[test]
    fn try_from_box_view_wrong_type() {
        let tfhd = make_box(b"tfhd", &make_tfhd_payload(1));

        let mut box_data = Vec::new();
        let size = 8 + tfhd.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"moof");
        box_data.extend_from_slice(&tfhd);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxFrame::parse_in(&mut cursor).unwrap();
        let result = TrafBoxView::try_from(box_view);

        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    mod alloc_tests {
        use super::*;

        #[test]
        fn traf_box_from_view() {
            let tfhd = make_box(b"tfhd", &make_tfhd_payload(5));
            let tfdt = make_box(b"tfdt", &make_tfdt_payload_v0(2000));
            let trun = make_box(b"trun", &make_trun_payload(15));

            let mut payload = Vec::new();
            payload.extend_from_slice(&tfhd);
            payload.extend_from_slice(&tfdt);
            payload.extend_from_slice(&trun);

            let view = TrafBoxView::parse(&payload).unwrap();
            let owned = TrafBox::from_view(&view).unwrap();

            assert_eq!(owned.tfhd.track_id, 5);
            assert_eq!(owned.tfdt.unwrap().base_media_decode_time, 2000);
            assert_eq!(owned.truns.len(), 1);
            assert_eq!(owned.truns[0].sample_count, 15); // 15 samples with no per-sample fields
        }

        #[test]
        fn traf_box_parse() {
            let tfhd = make_box(b"tfhd", &make_tfhd_payload(10));

            let view = TrafBoxView::parse(&tfhd).unwrap();
            let owned = TrafBox::from_view(&view).unwrap();

            assert_eq!(owned.tfhd.track_id, 10);
        }
    }
}
