use crate::cursor::ReadCursor;

use crate::BoxIter;
use crate::BoxType;
use crate::BoxView;
use crate::error::*;
use crate::types::FourCC;

/// A reference to a Track Reference Type Box.
///
/// Each track reference type box contains an array of track IDs that this track references.
#[derive(Debug)]
pub struct TrackReferenceTypeBoxView<'a> {
    /// The reference type (e.g., 'hint', 'cdsc', 'font', 'hind', 'vdep', etc.).
    pub reference_type: FourCC,
    track_ids: &'a [u8],
}

impl<'a> TrackReferenceTypeBoxView<'a> {
    const TRACK_ID_SIZE: usize = 4;

    /// Returns an iterator over the track IDs in this reference.
    pub fn track_ids(&self) -> impl Iterator<Item = u32> + 'a {
        let track_ids = self.track_ids;

        track_ids
            .chunks_exact(Self::TRACK_ID_SIZE)
            .map(|chunk| u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
    }

    /// Parses a `TrackReferenceTypeView` from the given box view.
    pub fn from_box_view(box_view: &BoxView<'a>) -> Result<Self> {
        let boxtype = box_view.header.boxtype();
        let reference_type = boxtype.type_field();
        let payload = box_view.payload;

        if !payload.len().is_multiple_of(Self::TRACK_ID_SIZE) {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Track reference payload is not a multiple of 4 bytes",
                    got: payload.len() as u64,
                },
                boxtype,
            ));
        }

        Ok(TrackReferenceTypeBoxView {
            reference_type,
            track_ids: payload,
        })
    }
}

/// A reference to a Track Reference Box (`tref`).
///
/// This box contains references to other tracks. The reference types indicate
/// the nature of the reference (e.g., 'hint' for hint tracks, 'cdsc' for
/// content description tracks, etc.).
#[derive(Debug)]
pub struct TrefBoxView<'a> {
    payload: &'a [u8],
}

impl<'a> TrefBoxView<'a> {
    /// Returns an iterator over the child boxes of this `TrefBoxView`.
    pub fn children(&self) -> BoxIter<'a> {
        BoxIter::new(self.payload)
    }

    /// Returns an iterator over the track reference types in this box.
    pub fn references(&self) -> impl Iterator<Item = Result<TrackReferenceTypeBoxView<'a>>> + 'a {
        self.children().map(|result| {
            result.and_then(|box_view| TrackReferenceTypeBoxView::from_box_view(&box_view))
        })
    }

    /// Finds a track reference by its reference type.
    pub fn find_reference(
        &self,
        reference_type: FourCC,
    ) -> Result<Option<TrackReferenceTypeBoxView<'a>>> {
        for result in self.references() {
            let reference = result?;
            if reference.reference_type == reference_type {
                return Ok(Some(reference));
            }
        }
        Ok(None)
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<TrefBoxView<'a>> {
        let payload = cur.take(cur.remaining())?;
        Ok(TrefBoxView { payload })
    }

    /// Parses a `TrefBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<TrefBoxView<'a>> {
        let mut cursor = ReadCursor::new(payload);
        TrefBoxView::parse_in(&mut cursor)
    }
}

impl<'a> TryFrom<&BoxView<'a>> for TrefBoxView<'a> {
    type Error = Error;

    fn try_from(value: &BoxView<'a>) -> Result<Self> {
        if value.header.boxtype() != BoxType::TREF {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::TREF,
                found: value.header.boxtype(),
            }));
        }

        TrefBoxView::parse(value.payload)
    }
}

#[cfg(feature = "alloc")]
pub use owned::*;

#[cfg(feature = "alloc")]
mod owned {
    use super::*;

    /// An owned Track Reference Type Box.
    #[derive(Debug, Clone)]
    pub struct TrackReferenceTypeBox {
        /// The reference type (e.g., 'hint', 'cdsc', 'font', 'hind', 'vdep', etc.).
        pub reference_type: FourCC,
        /// The track IDs referenced by this track.
        pub track_ids: Vec<u32>,
    }

    impl TrackReferenceTypeBox {
        /// Creates a `TrackReferenceTypeBox` from a `TrackReferenceTypeView`.
        pub fn from_view(view: &TrackReferenceTypeBoxView<'_>) -> Result<Self> {
            let track_ids: Vec<u32> = view.track_ids().collect();
            Ok(TrackReferenceTypeBox {
                reference_type: view.reference_type,
                track_ids,
            })
        }
    }

    impl TryFrom<&TrackReferenceTypeBoxView<'_>> for TrackReferenceTypeBox {
        type Error = Error;

        fn try_from(value: &TrackReferenceTypeBoxView<'_>) -> Result<Self> {
            TrackReferenceTypeBox::from_view(value)
        }
    }

    /// An owned Track Reference Box (`tref`).
    #[derive(Debug, Clone)]
    pub struct TrefBox {
        /// The track references contained in this box.
        pub references: Vec<TrackReferenceTypeBox>,
    }

    impl TrefBox {
        /// Creates a `TrefBox` from a `TrefBoxView`.
        pub fn from_view(view: &TrefBoxView<'_>) -> Result<Self> {
            let mut references = Vec::new();
            for result in view.references() {
                references.push(TrackReferenceTypeBox::from_view(&result?)?);
            }
            Ok(TrefBox { references })
        }

        /// Parses a `TrefBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<Self> {
            let view = TrefBoxView::parse(payload)?;
            TrefBox::from_view(&view)
        }

        /// Finds a track reference by its reference type.
        pub fn find_reference(&self, reference_type: FourCC) -> Option<&TrackReferenceTypeBox> {
            self.references
                .iter()
                .find(|r| r.reference_type == reference_type)
        }
    }

    impl TryFrom<&TrefBoxView<'_>> for TrefBox {
        type Error = Error;

        fn try_from(value: &TrefBoxView<'_>) -> Result<Self> {
            TrefBox::from_view(value)
        }
    }

    impl TryFrom<&BoxView<'_>> for TrefBox {
        type Error = Error;

        fn try_from(value: &BoxView<'_>) -> Result<Self> {
            if value.header.boxtype() != BoxType::TREF {
                return Err(Error::new(ErrorKind::MismatchedBoxType {
                    expected: BoxType::TREF,
                    found: value.header.boxtype(),
                }));
            }

            let view = TrefBoxView::parse(value.payload)?;
            TrefBox::from_view(&view)
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

    fn make_track_ref_payload(track_ids: &[u32]) -> Vec<u8> {
        let mut payload = Vec::new();
        for &id in track_ids {
            payload.extend_from_slice(&id.to_be_bytes());
        }
        payload
    }

    fn make_tref_payload(references: &[(&[u8; 4], &[u32])]) -> Vec<u8> {
        let mut payload = Vec::new();
        for (ref_type, track_ids) in references {
            let ref_payload = make_track_ref_payload(track_ids);
            payload.extend_from_slice(&make_box(ref_type, &ref_payload));
        }
        payload
    }

    #[test]
    fn parse_tref_empty() {
        let payload = make_tref_payload(&[]);
        let tref = TrefBoxView::parse(&payload).unwrap();

        assert_eq!(tref.references().count(), 0);
    }

    #[test]
    fn parse_tref_single_reference() {
        let payload = make_tref_payload(&[(b"hint", &[2])]);
        let tref = TrefBoxView::parse(&payload).unwrap();

        let references: Vec<_> = tref.references().collect();
        assert_eq!(references.len(), 1);

        let hint_ref = references[0].as_ref().unwrap();
        assert_eq!(hint_ref.reference_type, FourCC::new(*b"hint"));

        let track_ids: Vec<u32> = hint_ref.track_ids().collect();
        assert_eq!(track_ids, vec![2]);
    }

    #[test]
    fn parse_tref_multiple_references() {
        let payload =
            make_tref_payload(&[(b"hint", &[2, 3]), (b"cdsc", &[4]), (b"vdep", &[5, 6, 7])]);
        let tref = TrefBoxView::parse(&payload).unwrap();

        let references: Vec<_> = tref.references().map(|r| r.unwrap()).collect();
        assert_eq!(references.len(), 3);

        assert_eq!(references[0].reference_type, FourCC::new(*b"hint"));
        let track_ids_0: Vec<u32> = references[0].track_ids().collect();
        assert_eq!(track_ids_0.len(), 2);

        assert_eq!(references[1].reference_type, FourCC::new(*b"cdsc"));
        let track_ids_1: Vec<u32> = references[1].track_ids().collect();
        assert_eq!(track_ids_1.len(), 1);

        assert_eq!(references[2].reference_type, FourCC::new(*b"vdep"));
        let track_ids_2: Vec<u32> = references[2].track_ids().collect();
        assert_eq!(track_ids_2.len(), 3);
    }

    #[test]
    fn parse_tref_find_reference() {
        let payload = make_tref_payload(&[(b"hint", &[2]), (b"cdsc", &[4, 5])]);
        let tref = TrefBoxView::parse(&payload).unwrap();

        let hint_ref = tref.find_reference(FourCC::new(*b"hint")).unwrap().unwrap();
        let hint_track_ids: Vec<u32> = hint_ref.track_ids().collect();
        assert_eq!(hint_track_ids.len(), 1);

        let cdsc_ref = tref.find_reference(FourCC::new(*b"cdsc")).unwrap().unwrap();
        let cdsc_track_ids: Vec<u32> = cdsc_ref.track_ids().collect();
        assert_eq!(cdsc_track_ids.len(), 2);

        let missing = tref.find_reference(FourCC::new(*b"vdep")).unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn try_from_box_view_success() {
        let payload = make_tref_payload(&[(b"hint", &[2])]);

        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"tref");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxView::parse_in(&mut cursor).unwrap();
        let tref = TrefBoxView::try_from(&box_view).unwrap();

        assert_eq!(tref.references().count(), 1);
    }

    #[test]
    fn try_from_box_view_wrong_type() {
        let payload = make_tref_payload(&[]);

        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"trak"); // Wrong type
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxView::parse_in(&mut cursor).unwrap();
        let result = TrefBoxView::try_from(&box_view);

        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::MismatchedBoxType { .. }));
        }
    }

    #[cfg(feature = "alloc")]
    mod alloc_tests {
        use super::*;

        #[test]
        fn tref_box_from_view() {
            let payload = make_tref_payload(&[(b"hint", &[2, 3]), (b"cdsc", &[4])]);
            let view = TrefBoxView::parse(&payload).unwrap();
            let tref_box = TrefBox::from_view(&view).unwrap();

            assert_eq!(tref_box.references.len(), 2);
            assert_eq!(tref_box.references[0].reference_type, FourCC::new(*b"hint"));
            assert_eq!(tref_box.references[0].track_ids, vec![2, 3]);
            assert_eq!(tref_box.references[1].reference_type, FourCC::new(*b"cdsc"));
            assert_eq!(tref_box.references[1].track_ids, vec![4]);
        }

        #[test]
        fn tref_box_parse() {
            let payload = make_tref_payload(&[(b"vdep", &[5, 6, 7])]);
            let tref_box = TrefBox::parse(&payload).unwrap();

            assert_eq!(tref_box.references.len(), 1);
            assert_eq!(tref_box.references[0].track_ids, vec![5, 6, 7]);
        }

        #[test]
        fn tref_box_find_reference() {
            let payload = make_tref_payload(&[(b"hint", &[2]), (b"cdsc", &[4, 5])]);
            let tref_box = TrefBox::parse(&payload).unwrap();

            let hint_ref = tref_box.find_reference(FourCC::new(*b"hint")).unwrap();
            assert_eq!(hint_ref.track_ids, vec![2]);

            let cdsc_ref = tref_box.find_reference(FourCC::new(*b"cdsc")).unwrap();
            assert_eq!(cdsc_ref.track_ids, vec![4, 5]);

            let missing = tref_box.find_reference(FourCC::new(*b"vdep"));
            assert!(missing.is_none());
        }

        #[test]
        fn tref_box_empty() {
            let payload = make_tref_payload(&[]);
            let tref_box = TrefBox::parse(&payload).unwrap();

            assert!(tref_box.references.is_empty());
        }

        #[test]
        fn track_reference_type_box_try_from() {
            let payload = make_tref_payload(&[(b"hint", &[10, 20])]);
            let view = TrefBoxView::parse(&payload).unwrap();
            let ref_view = view.references().next().unwrap().unwrap();
            let track_ref: TrackReferenceTypeBox = (&ref_view).try_into().unwrap();

            assert_eq!(track_ref.reference_type, FourCC::new(*b"hint"));
            assert_eq!(track_ref.track_ids, vec![10, 20]);
        }
    }
}
