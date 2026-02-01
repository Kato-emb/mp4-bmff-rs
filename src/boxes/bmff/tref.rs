use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

/// A reference to a Track Reference Type Box.
#[derive(Debug)]
pub struct TrefTypeBoxView<'a> {
    track_ids: &'a [u8],
}

impl<'a> TrefTypeBoxView<'a> {
    /// Returns an iterator over the track IDs in this box.
    pub fn track_ids(&self) -> impl Iterator<Item = u32> + 'a {
        self.track_ids
            .chunks_exact(4)
            .map(|chunk| u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
    }
}

/// A reference to a Track Reference Box (`tref`).
pub struct TrefBoxView<'a> {
    content: &'a [u8],
}

impl<'a> TrefBoxView<'a> {
    /// Returns an iterator over the child boxes of this `tref` box.
    pub fn boxes(&self) -> BoxIter<'a> {
        BoxIter::new(self.content)
    }

    /// Returns an iterator over the track reference type boxes contained in this `tref` box.
    pub fn references(&self) -> impl Iterator<Item = Result<(BoxType, TrefTypeBoxView<'a>)>> + 'a {
        self.boxes().map(|result| {
            result.map(|rawbox| {
                let reference_type = rawbox.boxtype();
                let tref_type_box = TrefTypeBoxView {
                    track_ids: rawbox.into_payload(),
                };

                (reference_type, tref_type_box)
            })
        })
    }

    /// Finds a track reference type box by its reference type.
    pub fn find_reference(&self, reference_type: BoxType) -> Result<Option<TrefTypeBoxView<'a>>> {
        for result in self.references() {
            let (boxtype, tref_type_box) = result?;
            if boxtype == reference_type {
                return Ok(Some(tref_type_box));
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
        Ok(TrefBoxView { content: bytes })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use super::*;
    use crate::BoxEncode;
    use crate::BoxHeader;

    use crate::cursor::WriteCursor;

    /// An owned Track Reference Type Box.
    #[derive(Debug, Clone)]
    pub struct TrefTypeBox {
        /// The track IDs contained in this box.
        pub track_ids: Vec<u32>,
    }

    impl From<&TrefTypeBoxView<'_>> for TrefTypeBox {
        fn from(view: &TrefTypeBoxView) -> Self {
            let track_ids = view.track_ids().collect();
            TrefTypeBox { track_ids }
        }
    }

    impl TrefTypeBoxView<'_> {
        /// Converts this view into an owned `TrefTypeBox`.
        pub fn to_owned(&self) -> TrefTypeBox {
            TrefTypeBox::from(self)
        }
    }

    /// An owned Track Reference Box (`tref`).
    #[derive(Debug, Clone)]
    pub struct TrefBox {
        /// The references contained in this box.
        pub references: Vec<(BoxType, TrefTypeBox)>,
    }

    impl TryFrom<&TrefBoxView<'_>> for TrefBox {
        type Error = Error;

        fn try_from(value: &TrefBoxView<'_>) -> Result<Self> {
            let mut references = Vec::new();
            for result in value.references() {
                let (boxtype, tref_type_box_view) = result?;
                let tref_type_box = TrefTypeBox::from(&tref_type_box_view);
                references.push((boxtype, tref_type_box));
            }
            Ok(TrefBox { references })
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
            let mut len = 0;
            for (_, tref_type_box) in &self.references {
                len += 8; // Box header (size + type)
                len += tref_type_box.track_ids.len() * 4;
            }
            len
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            for (reference_type, tref_type_box) in &self.references {
                let header =
                    BoxHeader::new(*reference_type, (tref_type_box.track_ids.len() * 4) as u64);
                let header_len = header.header_len();
                header.write(cur.take_mut(header_len)?)?;
                for track_id in &tref_type_box.track_ids {
                    cur.write_u32_be(*track_id)?;
                }
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
    use crate::types::FourCC;

    fn raw_data() -> [u8; 28] {
        [
            // First child box: "hint" reference type
            0x00, 0x00, 0x00, 0x10, // size = 16 (8 header + 8 payload)
            b'h', b'i', b'n', b't', // type = "hint"
            0x00, 0x00, 0x00, 0x01, // track_id = 1
            0x00, 0x00, 0x00, 0x02, // track_id = 2
            // Second child box: "cdsc" reference type
            0x00, 0x00, 0x00, 0x0C, // size = 12 (8 header + 4 payload)
            b'c', b'd', b's', b'c', // type = "cdsc"
            0x00, 0x00, 0x00, 0x03, // track_id = 3
        ]
    }

    #[test]
    fn test_tref_box_view_decode() {
        let data = raw_data();
        let tref_view = TrefBoxView::decode(&data).unwrap();

        let mut references = tref_view.references();

        // First reference: hint
        let (boxtype, tref_type_box) = references.next().unwrap().unwrap();
        assert_eq!(boxtype, BoxType::new(FourCC::new(*b"hint")));
        let mut track_ids = tref_type_box.track_ids();
        assert_eq!(track_ids.next(), Some(1));
        assert_eq!(track_ids.next(), Some(2));
        assert!(track_ids.next().is_none());

        // Second reference: cdsc
        let (boxtype, tref_type_box) = references.next().unwrap().unwrap();
        assert_eq!(boxtype, BoxType::new(FourCC::new(*b"cdsc")));
        let mut track_ids = tref_type_box.track_ids();
        assert_eq!(track_ids.next(), Some(3));
        assert!(track_ids.next().is_none());

        assert!(references.next().is_none());
    }

    #[test]
    fn test_tref_box_view_empty_references() {
        let data: [u8; 0] = [];
        let tref_view = TrefBoxView::decode(&data).unwrap();

        assert!(tref_view.references().next().is_none());
    }

    #[test]
    fn test_tref_type_box_view_empty_track_ids() {
        let data: [u8; 8] = [
            0x00, 0x00, 0x00, 0x08, // size = 8 (header only, no track IDs)
            b'h', b'i', b'n', b't', // type = "hint"
        ];

        let tref_view = TrefBoxView::decode(&data).unwrap();
        let mut references = tref_view.references();
        let (_, tref_type_box) = references.next().unwrap().unwrap();
        assert!(tref_type_box.track_ids().next().is_none());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_tref_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_data();
        let tref_box = TrefBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; tref_box.encoded_len()];
        tref_box.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_tref_box_to_owned() {
        let data = raw_data();
        let view = TrefBoxView::decode(&data).unwrap();
        let owned = TrefBox::try_from(&view).unwrap();

        assert_eq!(owned.references.len(), 2);
        assert_eq!(owned.references[0].0, BoxType::new(FourCC::new(*b"hint")));
        assert_eq!(owned.references[0].1.track_ids, vec![1, 2]);
        assert_eq!(owned.references[1].0, BoxType::new(FourCC::new(*b"cdsc")));
        assert_eq!(owned.references[1].1.track_ids, vec![3]);
    }
}
