use crate::types::FourCC;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::RawBoxRef;
use crate::error::*;
use crate::iter::BoxIter;

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

    /// Creates a `TrackReferenceTypeBoxView` from a `RawBoxRef`.
    pub fn from_rawbox(rawbox: RawBoxRef<'a>) -> Result<Self> {
        let reference_type = rawbox.boxtype().type_field();
        let track_ids = rawbox.into_payload();

        if !track_ids.len().is_multiple_of(Self::TRACK_ID_SIZE) {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Track reference payload is not a multiple of 4 bytes",
                    got: track_ids.len() as u64,
                },
                BoxType::TREF,
            ));
        }

        Ok(TrackReferenceTypeBoxView {
            reference_type,
            track_ids,
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
        self.children()
            .map(|result| result.and_then(TrackReferenceTypeBoxView::from_rawbox))
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
}

impl BoxCodec for TrefBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::TREF
    }
}

impl<'de> BoxDecode<'de> for TrefBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        Ok(TrefBoxView { payload: bytes })
    }
}

#[cfg(feature = "alloc")]
pub use owned::*;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use crate::cursor::WriteCursor;

    use crate::codec::boxed_len;
    use crate::codec::write_box_in;

    use super::*;
    use crate::BoxCodec;
    use crate::BoxDecode;
    use crate::BoxEncode;

    /// An owned Track Reference Type Box.
    #[derive(Debug, Clone)]
    pub struct TrackReferenceTypeBox {
        /// The reference type (e.g., 'hint', 'cdsc', 'font', 'hind', 'vdep', etc.).
        pub reference_type: FourCC,
        /// The track IDs referenced by this track.
        pub track_ids: Vec<u32>,
    }

    impl TryFrom<&TrackReferenceTypeBoxView<'_>> for TrackReferenceTypeBox {
        type Error = Error;

        fn try_from(view: &TrackReferenceTypeBoxView<'_>) -> Result<Self> {
            let track_ids: Vec<u32> = view.track_ids().collect();
            Ok(TrackReferenceTypeBox {
                reference_type: view.reference_type,
                track_ids,
            })
        }
    }

    impl BoxCodec for TrackReferenceTypeBox {
        fn boxtype(&self) -> BoxType {
            // Track reference types are always valid FourCC values
            BoxType::new(self.reference_type)
        }
    }

    impl BoxEncode for TrackReferenceTypeBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            self.track_ids.len() * 4
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);
            for &track_id in &self.track_ids {
                cur.write_u32_be(track_id)?;
            }
            Ok(cur.position())
        }
    }

    /// An owned Track Reference Box (`tref`).
    #[derive(Debug, Clone)]
    pub struct TrefBox {
        /// The track references contained in this box.
        pub references: Vec<TrackReferenceTypeBox>,
    }

    impl TryFrom<&TrefBoxView<'_>> for TrefBox {
        type Error = Error;

        fn try_from(view: &TrefBoxView<'_>) -> Result<Self> {
            let mut references = Vec::new();
            for result in view.references() {
                references.push(TrackReferenceTypeBox::try_from(&result?)?);
            }
            Ok(TrefBox { references })
        }
    }

    impl TrefBox {
        /// Finds a track reference by its reference type.
        pub fn find_reference(&self, reference_type: FourCC) -> Option<&TrackReferenceTypeBox> {
            self.references
                .iter()
                .find(|r| r.reference_type == reference_type)
        }
    }

    impl BoxCodec for TrefBox {
        fn boxtype(&self) -> BoxType {
            BoxType::TREF
        }
    }

    impl BoxDecode<'_> for TrefBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = TrefBoxView::decode(bytes)?;
            TrefBox::try_from(&view)
        }
    }

    impl BoxEncode for TrefBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            self.references.iter().map(boxed_len).sum()
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);
            for reference in &self.references {
                write_box_in(&mut cur, reference)?;
            }

            Ok(cur.position())
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
    fn parse_and_iterate_references() {
        // Empty case
        let payload = make_tref_payload(&[]);
        let tref = TrefBoxView::decode(&payload).unwrap();
        assert_eq!(tref.references().count(), 0);

        // Single reference
        let payload = make_tref_payload(&[(b"hint", &[2])]);
        let tref = TrefBoxView::decode(&payload).unwrap();
        let references: Vec<_> = tref.references().collect();
        assert_eq!(references.len(), 1);
        let hint_ref = references[0].as_ref().unwrap();
        assert_eq!(hint_ref.reference_type, FourCC::new(*b"hint"));
        assert_eq!(hint_ref.track_ids().collect::<Vec<_>>(), vec![2]);

        // Multiple references
        let payload =
            make_tref_payload(&[(b"hint", &[2, 3]), (b"cdsc", &[4]), (b"vdep", &[5, 6, 7])]);
        let tref = TrefBoxView::decode(&payload).unwrap();
        let references: Vec<_> = tref.references().map(|r| r.unwrap()).collect();
        assert_eq!(references.len(), 3);
        assert_eq!(references[0].track_ids().count(), 2);
        assert_eq!(references[1].track_ids().count(), 1);
        assert_eq!(references[2].track_ids().count(), 3);
    }

    #[test]
    fn find_reference() {
        let payload = make_tref_payload(&[(b"hint", &[2]), (b"cdsc", &[4, 5])]);
        let tref = TrefBoxView::decode(&payload).unwrap();

        // Found cases
        let hint_ref = tref.find_reference(FourCC::new(*b"hint")).unwrap().unwrap();
        assert_eq!(hint_ref.track_ids().collect::<Vec<_>>(), vec![2]);

        let cdsc_ref = tref.find_reference(FourCC::new(*b"cdsc")).unwrap().unwrap();
        assert_eq!(cdsc_ref.track_ids().collect::<Vec<_>>(), vec![4, 5]);

        // Not found case
        assert!(
            tref.find_reference(FourCC::new(*b"vdep"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn invalid_payload_size() {
        // Payload not a multiple of 4 should fail
        let mut box_data = Vec::new();
        box_data.extend_from_slice(&13u32.to_be_bytes()); // size: 13 (8 header + 5 payload)
        box_data.extend_from_slice(b"hint");
        box_data.extend_from_slice(&[1, 2, 3, 4, 5]); // 5 bytes - not multiple of 4

        let frame = RawBoxRef::parse(&box_data).unwrap();
        let result = TrackReferenceTypeBoxView::from_rawbox(frame);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidBoxSize { .. }
        ));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn owned_conversion() {
        let payload = make_tref_payload(&[(b"hint", &[2, 3]), (b"cdsc", &[4])]);
        let view = TrefBoxView::decode(&payload).unwrap();
        let owned = TrefBox::try_from(&view).unwrap();

        assert_eq!(owned.references.len(), 2);
        assert_eq!(owned.references[0].track_ids, vec![2, 3]);
        assert_eq!(owned.references[1].track_ids, vec![4]);

        // Test find on owned
        assert_eq!(
            owned
                .find_reference(FourCC::new(*b"hint"))
                .unwrap()
                .track_ids,
            vec![2, 3]
        );
        assert!(owned.find_reference(FourCC::new(*b"vdep")).is_none());
    }
}
