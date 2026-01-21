use crate::cursor::ReadCursor;

use crate::BoxFrame;
use crate::BoxIter;
use crate::BoxType;
use crate::error::*;

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
                Some(TfraBoxView::try_from(box_view.payload()))
            }
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<MfraBoxView<'a>> {
        let payload = cur.take(cur.remaining())?;
        Ok(MfraBoxView { payload })
    }

    /// Parses a `MfraBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<MfraBoxView<'a>> {
        let mut cursor = ReadCursor::new(payload);
        MfraBoxView::parse_in(&mut cursor)
    }
}

impl<'a> TryFrom<BoxFrame<'a>> for MfraBoxView<'a> {
    type Error = Error;

    fn try_from(value: BoxFrame<'a>) -> Result<Self> {
        if value.boxtype() != BoxType::MFRA {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::MFRA,
                found: value.boxtype(),
            }));
        }

        MfraBoxView::parse(value.payload())
    }
}

#[cfg(feature = "alloc")]
pub use owned::MfraBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use crate::cursor::WriteCursor;

    use super::*;
    use crate::BoxFrameMut;
    use crate::base::frame::write_box_in;

    use crate::boxes::TfraBox;

    /// An owned Movie Fragment Random Access Box (`mfra`).
    #[derive(Debug, Clone)]
    pub struct MfraBox {
        /// The Track Fragment Random Access Boxes (`tfra`).
        pub tfras: Vec<TfraBox>,
        /// The Movie Fragment Random Access Offset Box (`mfro`).
        pub mfro: MfroBox,
    }

    impl MfraBox {
        /// Creates a `MfraBox` from a `MfraBoxView`.
        pub fn from_view(view: &MfraBoxView<'_>) -> Result<MfraBox> {
            let mfro = view.mfro()?;

            let mut tfras = Vec::new();
            for tfra_result in view.tfras() {
                let tfra_view = tfra_result?;
                tfras.push(TfraBox::from_view(&tfra_view)?);
            }

            Ok(MfraBox { tfras, mfro })
        }

        /// Parses a `MfraBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<MfraBox> {
            let view = MfraBoxView::parse(payload)?;
            MfraBox::from_view(&view)
        }

        /// Returns the size of the payload in bytes.
        pub fn size(&self) -> usize {
            let mut size = 0;
            for tfra in &self.tfras {
                size += BoxFrameMut::required_len(BoxType::TFRA, tfra.size());
            }
            size += BoxFrameMut::required_len(BoxType::MFRO, self.mfro.payload_size());
            size
        }

        pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            for tfra in &self.tfras {
                write_box_in(cur, BoxType::TFRA, tfra.size(), |p| tfra.write(p))?;
            }

            write_box_in(cur, BoxType::MFRO, self.mfro.payload_size(), |p| {
                self.mfro.write(p)
            })?;

            if !cur.is_empty() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxSize {
                        reason: "Buffer larger than expected",
                        got: cur.remaining() as u64,
                    },
                    BoxType::MFRA,
                ));
            }

            Ok(())
        }

        /// Writes this `MfraBox` into the given payload.
        pub fn write(&self, payload: &mut [u8]) -> Result<()> {
            let mut cursor = WriteCursor::new(payload);
            self.write_in(&mut cursor)
        }
    }

    impl TryFrom<&MfraBoxView<'_>> for MfraBox {
        type Error = Error;

        fn try_from(value: &MfraBoxView<'_>) -> Result<Self> {
            MfraBox::from_view(value)
        }
    }

    impl TryFrom<BoxFrame<'_>> for MfraBox {
        type Error = Error;

        fn try_from(value: BoxFrame<'_>) -> Result<Self> {
            let view = MfraBoxView::try_from(value)?;
            MfraBox::from_view(&view)
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

        let mfra = MfraBoxView::parse(&mfro).unwrap();

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

        let mfra = MfraBoxView::parse(&payload).unwrap();

        assert_eq!(mfra.mfro().unwrap().size, 2048);

        let tfras: Vec<_> = mfra.tfras().collect();
        assert_eq!(tfras.len(), 2);
        assert_eq!(tfras[0].as_ref().unwrap().track_id, 1);
        assert_eq!(tfras[1].as_ref().unwrap().track_id, 2);
    }

    #[test]
    fn parse_mfra_missing_mfro() {
        let tfra = make_box(b"tfra", &make_tfra_payload_minimal(1));

        let mfra = MfraBoxView::parse(&tfra).unwrap();
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

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxFrame::parse_in(&mut cursor).unwrap();
        let mfra = MfraBoxView::try_from(box_view).unwrap();

        assert_eq!(mfra.mfro().unwrap().size, 512);
    }

    #[test]
    fn try_from_box_view_wrong_type() {
        let mfro = make_box(b"mfro", &make_mfro_payload(512));

        let mut box_data = Vec::new();
        let size = 8 + mfro.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"moof");
        box_data.extend_from_slice(&mfro);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxFrame::parse_in(&mut cursor).unwrap();
        let result = MfraBoxView::try_from(box_view);

        assert!(result.is_err());
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

            let view = MfraBoxView::parse(&payload).unwrap();
            let owned = MfraBox::from_view(&view).unwrap();

            assert_eq!(owned.mfro.size, 4096);
            assert_eq!(owned.tfras.len(), 1);
            assert_eq!(owned.tfras[0].track_id, 3);
        }

        #[test]
        fn mfra_box_parse() {
            let mfro = make_box(b"mfro", &make_mfro_payload(8192));

            let owned = MfraBox::parse(&mfro).unwrap();

            assert_eq!(owned.mfro.size, 8192);
            assert_eq!(owned.tfras.len(), 0);
        }
    }
}
