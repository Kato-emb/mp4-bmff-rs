use crate::BoxCodec;
use crate::BoxDecode;
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
                return TfhdBox::decode(child.into_payload());
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
                return Ok(Some(TfdtBox::decode(child.into_payload())?));
            }
        }

        Ok(None)
    }

    /// Returns an iterator over the Track Run Boxes (`trun`).
    pub fn truns(&self) -> impl Iterator<Item = Result<TrunBoxView<'a>>> + 'a {
        self.children().filter_map(|result| match result {
            Ok(view) if view.boxtype() == BoxType::TRUN => {
                Some(TrunBoxView::decode(view.into_payload()))
            }
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
    }

    /// Returns an iterator over the Sample to Group Boxes (`sbgp`).
    pub fn sbgps(&self) -> impl Iterator<Item = Result<SbgpBoxView<'a>>> + 'a {
        self.children().filter_map(|result| match result {
            Ok(view) if view.boxtype() == BoxType::SBGP => {
                Some(SbgpBoxView::decode(view.into_payload()))
            }
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
    }
}

impl BoxCodec for TrafBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::TRAF
    }
}

impl<'de> BoxDecode<'de> for TrafBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        Ok(TrafBoxView { payload: bytes })
    }
}

impl<'a> TryFrom<&'a [u8]> for TrafBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        TrafBoxView::decode(value)
    }
}

#[cfg(feature = "alloc")]
pub use owned::TrafBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use crate::cursor::WriteCursor;

    use crate::base::writer::write_box_in;

    use super::*;
    use crate::BoxCodec;
    use crate::BoxDecode;
    use crate::BoxEncode;
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

    impl TryFrom<&TrafBoxView<'_>> for TrafBox {
        type Error = Error;

        fn try_from(view: &TrafBoxView<'_>) -> Result<Self> {
            let tfhd = view.tfhd()?;
            let tfdt = view.tfdt()?;

            let mut truns = Vec::new();
            for trun_result in view.truns() {
                let trun_view = trun_result?;
                truns.push(TrunBox::from(&trun_view));
            }

            let mut sbgps = Vec::new();
            for sbgp_result in view.sbgps() {
                let sbgp_view = sbgp_result?;
                sbgps.push(SbgpBox::try_from(&sbgp_view)?);
            }

            Ok(TrafBox {
                tfhd,
                tfdt,
                truns,
                sbgps,
            })
        }
    }

    impl BoxCodec for TrafBox {
        fn boxtype(&self) -> BoxType {
            BoxType::TRAF
        }
    }

    impl BoxDecode<'_> for TrafBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = TrafBoxView::decode(bytes)?;
            TrafBox::try_from(&view)
        }
    }

    impl BoxEncode for TrafBox {
        fn encode(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            write_box_in(&mut cur, &self.tfhd)?;

            if let Some(ref tfdt) = self.tfdt {
                write_box_in(&mut cur, tfdt)?;
            }

            for trun in &self.truns {
                write_box_in(&mut cur, trun)?;
            }

            for sbgp in &self.sbgps {
                write_box_in(&mut cur, sbgp)?;
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

        let traf = TrafBoxView::decode(&payload).unwrap();

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

        let traf = TrafBoxView::decode(&payload).unwrap();

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

        let traf = TrafBoxView::decode(&payload).unwrap();

        let truns: Vec<_> = traf.truns().collect();
        assert_eq!(truns.len(), 2);
        assert_eq!(truns[0].as_ref().unwrap().sample_count, 10);
        assert_eq!(truns[1].as_ref().unwrap().sample_count, 20);
    }

    #[test]
    fn parse_traf_missing_tfhd() {
        let tfdt = make_box(b"tfdt", &make_tfdt_payload_v0(0));

        let traf = TrafBoxView::decode(&tfdt).unwrap();
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

        let raw = RawBoxRef::parse(&box_data).unwrap();
        let traf = TrafBoxView::try_from(raw.payload()).unwrap();

        assert_eq!(traf.tfhd().unwrap().track_id, 1);
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

            let view = TrafBoxView::decode(&payload).unwrap();
            let owned = TrafBox::try_from(&view).unwrap();

            assert_eq!(owned.tfhd.track_id, 5);
            assert_eq!(owned.tfdt.unwrap().base_media_decode_time, 2000);
            assert_eq!(owned.truns.len(), 1);
            assert_eq!(owned.truns[0].sample_count, 15); // 15 samples with no per-sample fields
        }

        #[test]
        fn traf_box_parse() {
            let tfhd = make_box(b"tfhd", &make_tfhd_payload(10));

            let view = TrafBoxView::decode(&tfhd).unwrap();
            let owned = TrafBox::try_from(&view).unwrap();

            assert_eq!(owned.tfhd.track_id, 10);
        }
    }
}
