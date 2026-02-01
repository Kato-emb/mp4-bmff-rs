use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

use super::TfdtBox;
use super::TfhdBox;
use super::TrunBoxView;

/// A reference to a Track Fragment Box (`traf`).
#[derive(Debug)]
pub struct TrafBoxView<'a> {
    content: &'a [u8],
}

impl<'a> TrafBoxView<'a> {
    /// Returns an iterator over the child boxes of this `traf` box.
    pub fn boxes(&self) -> BoxIter<'a> {
        BoxIter::new(self.content)
    }

    /// Returns the Track Fragment Header Box (`tfhd`) contained in this `traf` box.
    pub fn tfhd(&self) -> Result<TfhdBox> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::TFHD {
                let tfhd = TfhdBox::decode(b.payload())?;
                return Ok(tfhd);
            }
        }

        Err(Error::in_box(
            ErrorKind::BoxMissing {
                required: BoxType::TFHD,
            },
            BoxType::TRAF,
        ))
    }

    /// Returns an iterator over the Track Fragment Run Boxes (`trun`) contained in this `traf` box.
    pub fn truns(&self) -> impl Iterator<Item = Result<TrunBoxView<'a>>> + 'a {
        self.boxes().filter_map(|result| match result {
            Ok(rawbox) if rawbox.boxtype() == BoxType::TRUN => {
                Some(TrunBoxView::decode(rawbox.into_payload()))
            }
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
    }

    /// Returns the Track Fragment Decode Time Box (`tfdt`) contained in this `traf` box.
    pub fn tfdt(&self) -> Result<Option<TfdtBox>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::TFDT {
                let tfdt = TfdtBox::decode(b.payload())?;
                return Ok(Some(tfdt));
            }
        }

        Ok(None)
    }
}

impl BoxCodec for TrafBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::TRAF
    }
}

impl<'de> BoxDecode<'de> for TrafBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        Ok(TrafBoxView { content: bytes })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use super::*;
    use crate::BoxEncode;

    use crate::codec::boxed_len;
    use crate::codec::write_box_in;
    use crate::cursor::WriteCursor;

    use crate::boxes::bmff::TrunBox;

    /// An owned Track Fragment Box (`traf`).
    #[derive(Debug, Clone)]
    pub struct TrafBox {
        /// The Track Fragment Header Box (`tfhd`).
        pub tfhd: TfhdBox,
        /// The Track Fragment Run Boxes (`trun`).
        pub truns: Vec<TrunBox>,
        /// The Track Fragment Decode Time Box (`tfdt`), if present.
        pub tfdt: Option<TfdtBox>,
    }

    impl TryFrom<&TrafBoxView<'_>> for TrafBox {
        type Error = Error;

        fn try_from(view: &TrafBoxView<'_>) -> Result<Self> {
            let mut tfhd = None;
            let mut truns = Vec::new();
            let mut tfdt = None;

            for result in view.boxes() {
                let rawbox = result?;
                match rawbox.boxtype() {
                    BoxType::TFHD => {
                        if tfhd.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::TFHD,
                                },
                                BoxType::TRAF,
                            ));
                        }
                        tfhd = Some(TfhdBox::decode(rawbox.payload())?);
                    }
                    BoxType::TRUN => {
                        let trun = TrunBox::decode(rawbox.into_payload())?;
                        truns.push(trun);
                    }
                    BoxType::TFDT => {
                        if tfdt.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::TFDT,
                                },
                                BoxType::TRAF,
                            ));
                        }
                        tfdt = Some(TfdtBox::decode(rawbox.payload())?);
                    }
                    _ => continue,
                }
            }

            let tfhd = tfhd.ok_or_else(|| {
                Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::TFHD,
                    },
                    BoxType::TRAF,
                )
            })?;

            Ok(TrafBox { tfhd, truns, tfdt })
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
        #[inline]
        fn encoded_len(&self) -> usize {
            boxed_len(&self.tfhd)
                + self.truns.iter().map(|trun| boxed_len(trun)).sum::<usize>()
                + self.tfdt.as_ref().map_or(0, |tfdt| boxed_len(tfdt))
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            write_box_in(&mut cur, &self.tfhd)?;
            for trun in &self.truns {
                write_box_in(&mut cur, trun)?;
            }
            if let Some(tfdt) = &self.tfdt {
                write_box_in(&mut cur, tfdt)?;
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

    fn raw_data_tfhd_only() -> [u8; 16] {
        [
            // tfhd box (minimal, no optional fields)
            0x00, 0x00, 0x00, 0x10, // size = 16
            b't', b'f', b'h', b'd', // type = "tfhd"
            0x00,             // version = 0
            0x00, 0x00, 0x00, // flags = 0 (no optional fields)
            0x00, 0x00, 0x00, 0x01, // track_id = 1
        ]
    }

    fn raw_data_tfhd_and_trun() -> [u8; 32] {
        [
            // tfhd box (minimal)
            0x00, 0x00, 0x00, 0x10, // size = 16
            b't', b'f', b'h', b'd', // type = "tfhd"
            0x00,             // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x02, // track_id = 2
            // trun box (minimal, no samples)
            0x00, 0x00, 0x00, 0x10, // size = 16
            b't', b'r', b'u', b'n', // type = "trun"
            0x00,             // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x00, // sample_count = 0
        ]
    }

    fn raw_data_tfhd_and_tfdt() -> [u8; 32] {
        [
            // tfhd box (minimal)
            0x00, 0x00, 0x00, 0x10, // size = 16
            b't', b'f', b'h', b'd', // type = "tfhd"
            0x00,             // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x01, // track_id = 1
            // tfdt box (version 0)
            0x00, 0x00, 0x00, 0x10, // size = 16
            b't', b'f', b'd', b't', // type = "tfdt"
            0x00,             // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x10, 0x00, // base_media_decode_time = 4096
        ]
    }

    #[test]
    fn test_traf_box_view_decode_empty() {
        let data = raw_data_empty();
        let traf = TrafBoxView::decode(&data).unwrap();

        assert_eq!(traf.boxes().count(), 0);
        assert_eq!(traf.truns().count(), 0);
    }

    #[test]
    fn test_traf_box_view_missing_required_tfhd() {
        let data = raw_data_empty();
        let traf = TrafBoxView::decode(&data).unwrap();

        let result = traf.tfhd();
        assert!(result.is_err());
    }

    #[test]
    fn test_traf_box_view_tfhd_only() {
        let data = raw_data_tfhd_only();
        let traf = TrafBoxView::decode(&data).unwrap();

        let tfhd = traf.tfhd().unwrap();
        assert_eq!(tfhd.track_id, 1);

        assert_eq!(traf.truns().count(), 0);
        assert!(traf.tfdt().unwrap().is_none());
    }

    #[test]
    fn test_traf_box_view_tfhd_and_trun() {
        let data = raw_data_tfhd_and_trun();
        let traf = TrafBoxView::decode(&data).unwrap();

        let tfhd = traf.tfhd().unwrap();
        assert_eq!(tfhd.track_id, 2);

        let truns: Vec<_> = traf.truns().collect();
        assert_eq!(truns.len(), 1);

        let trun = truns[0].as_ref().unwrap();
        assert_eq!(trun.sample_count, 0);
    }

    #[test]
    fn test_traf_box_view_tfhd_and_tfdt() {
        let data = raw_data_tfhd_and_tfdt();
        let traf = TrafBoxView::decode(&data).unwrap();

        let tfhd = traf.tfhd().unwrap();
        assert_eq!(tfhd.track_id, 1);

        let tfdt = traf.tfdt().unwrap();
        assert!(tfdt.is_some());
        assert_eq!(tfdt.unwrap().base_media_decode_time, 4096);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_traf_box_try_from() {
        let data = raw_data_tfhd_only();
        let view = TrafBoxView::decode(&data).unwrap();
        let owned = TrafBox::try_from(&view).unwrap();

        assert_eq!(owned.tfhd.track_id, 1);
        assert!(owned.truns.is_empty());
        assert!(owned.tfdt.is_none());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_traf_box_try_from_missing_tfhd() {
        let data = raw_data_empty();
        let view = TrafBoxView::decode(&data).unwrap();
        let result = TrafBox::try_from(&view);

        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_traf_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_data_tfhd_only();
        let traf = TrafBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; traf.encoded_len()];
        let len = traf.encode_into(&mut encoded).unwrap();

        assert_eq!(len, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }
}
