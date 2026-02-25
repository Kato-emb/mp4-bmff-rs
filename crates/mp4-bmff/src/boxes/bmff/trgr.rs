//! Track Group Box (`trgr`) implementation.
//!
//! The Track Group Box enables grouping of tracks that share a common
//! characteristic. Tracks with the same track group ID for a given track
//! group type belong to the same group. This is useful for indicating
//! tracks that are alternatives to each other or have some other logical
//! relationship.

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

/// A Track Group Type Box entry.
///
/// Contains a track group ID that, when combined with the track group type
/// (the FourCC of the containing box), identifies which group this track
/// belongs to.
///
/// # Common Track Group Types
///
/// - `msrc`: Multi-source presentation group. Tracks with the same ID
///   originated from the same source.
/// - `cstg`: CMAF switching track group. Tracks with the same ID can be
///   switched between during adaptive streaming.
/// - `alte`: Alternate group. Tracks with the same ID are alternatives
///   (e.g., different bitrates or languages).
///
/// # Structure
///
/// - `track_group_id`: 32-bit identifier for the group within this type.
#[derive(Debug, Clone, Copy)]
pub struct TrgrTypeBox {
    /// The track group ID.
    pub track_group_id: u32,
}

/// A reference to a Track Group Box (`trgr`).
///
/// The Track Group Box is a container for track group type boxes. Each child
/// box indicates that this track belongs to a particular group identified by
/// the combination of the child box's type (FourCC) and the track group ID
/// in its payload.
///
/// # Structure
///
/// Contains zero or more track group type boxes, each defining membership
/// in a specific track group.
#[derive(Debug)]
pub struct TrgrBoxView<'a> {
    content: &'a [u8],
}

impl<'a> TrgrBoxView<'a> {
    /// Returns an iterator over the child boxes of this `trgr` box.
    pub fn boxes(&self) -> BoxIter<'a> {
        BoxIter::new(self.content)
    }

    /// Returns an iterator over the track group type boxes contained in this `trgr` box.
    pub fn track_groups(&self) -> impl Iterator<Item = Result<(BoxType, TrgrTypeBox)>> + 'a {
        self.boxes().map(|result| {
            result.map(|rawbox| {
                let track_group_type = rawbox.boxtype();
                let track_group_id_bytes = rawbox.into_payload();
                let track_group_id = u32::from_be_bytes([
                    track_group_id_bytes[0],
                    track_group_id_bytes[1],
                    track_group_id_bytes[2],
                    track_group_id_bytes[3],
                ]);

                (track_group_type, TrgrTypeBox { track_group_id })
            })
        })
    }

    /// Finds a track group type box by its track group type.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed or too short.
    pub fn find_track_group(&self, track_group_type: BoxType) -> Result<Option<TrgrTypeBox>> {
        for result in self.track_groups() {
            let (boxtype, track_group_box) = result?;
            if boxtype == track_group_type {
                return Ok(Some(track_group_box));
            }
        }

        Ok(None)
    }
}

impl BoxCodec for TrgrBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::TRGR
    }
}

impl<'de> BoxDecode<'de> for TrgrBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        Ok(TrgrBoxView { content: bytes })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use alloc::vec::Vec;

    use super::*;
    use crate::BoxEncode;
    use crate::BoxHeader;

    use crate::cursor::WriteCursor;

    /// An owned Track Group Box (`trgr`).
    ///
    /// This is the owned variant of [`TrgrBoxView`] that stores track group
    /// memberships in a heap-allocated vector.
    ///
    /// # Structure
    ///
    /// - `track_groups`: List of track group memberships, each containing
    ///   the group type (BoxType/FourCC) and the track group ID.
    ///
    /// # Example
    ///
    /// ```
    /// use mp4_bmff::BoxDecode;
    /// use mp4_bmff::boxes::bmff::TrgrBox;
    ///
    /// // A trgr box with an "msrc" group membership
    /// let data: [u8; 12] = [
    ///     0x00, 0x00, 0x00, 0x0C, // size = 12
    ///     b'm', b's', b'r', b'c', // type = "msrc"
    ///     0x00, 0x00, 0x00, 0x01, // track_group_id = 1
    /// ];
    ///
    /// let trgr = TrgrBox::decode(&data).unwrap();
    /// assert_eq!(trgr.track_groups.len(), 1);
    /// assert_eq!(trgr.track_groups[0].1.track_group_id, 1);
    /// ```
    #[derive(Debug, Clone)]
    pub struct TrgrBox {
        /// Track group memberships organized by group type.
        /// Each tuple contains (group_type, track_group_id).
        pub track_groups: Vec<(BoxType, TrgrTypeBox)>,
    }

    impl TryFrom<&TrgrBoxView<'_>> for TrgrBox {
        type Error = Error;

        fn try_from(view: &TrgrBoxView<'_>) -> Result<Self> {
            let mut track_groups = Vec::new();
            for result in view.track_groups() {
                let (boxtype, track_group_box) = result?;
                track_groups.push((boxtype, track_group_box));
            }

            Ok(TrgrBox { track_groups })
        }
    }

    impl BoxCodec for TrgrBox {
        fn boxtype(&self) -> BoxType {
            BoxType::TRGR
        }
    }

    impl BoxDecode<'_> for TrgrBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = TrgrBoxView::decode(bytes)?;
            TrgrBox::try_from(&view)
        }
    }

    impl BoxEncode for TrgrBox {
        fn encoded_len(&self) -> usize {
            self.track_groups.len() * 12 // Each track group box: 8 bytes header + 4 bytes track_group_id
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            for (track_group_type, track_group_box) in &self.track_groups {
                let header = BoxHeader::new(*track_group_type, 4);
                let header_len = header.header_len();
                header.write(cur.take_mut(header_len)?)?;
                cur.write_u32_be(track_group_box.track_group_id)?;
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

    fn raw_data() -> [u8; 24] {
        [
            // First child box: "msrc" track group type
            0x00, 0x00, 0x00, 0x0C, // size = 12 (8 header + 4 payload)
            b'm', b's', b'r', b'c', // type = "msrc"
            0x00, 0x00, 0x00, 0x01, // track_group_id = 1
            // Second child box: "cstg" track group type
            0x00, 0x00, 0x00, 0x0C, // size = 12 (8 header + 4 payload)
            b'c', b's', b't', b'g', // type = "cstg"
            0x00, 0x00, 0x00, 0x02, // track_group_id = 2
        ]
    }

    #[test]
    fn test_trgr_box_view_decode() {
        let data = raw_data();
        let trgr_view = TrgrBoxView::decode(&data).unwrap();

        let mut track_groups = trgr_view.track_groups();

        // First track group: msrc
        let (boxtype, track_group_box) = track_groups.next().unwrap().unwrap();
        assert_eq!(boxtype, BoxType::new(FourCC::new(*b"msrc")));
        assert_eq!(track_group_box.track_group_id, 1);

        // Second track group: cstg
        let (boxtype, track_group_box) = track_groups.next().unwrap().unwrap();
        assert_eq!(boxtype, BoxType::new(FourCC::new(*b"cstg")));
        assert_eq!(track_group_box.track_group_id, 2);

        assert!(track_groups.next().is_none());
    }

    #[test]
    fn test_trgr_box_view_empty_track_groups() {
        let data: [u8; 0] = [];
        let trgr_view = TrgrBoxView::decode(&data).unwrap();

        assert!(trgr_view.track_groups().next().is_none());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_trgr_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_data();
        let trgr_box = TrgrBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; trgr_box.encoded_len()];
        trgr_box.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_trgr_box_to_owned() {
        let data = raw_data();
        let view = TrgrBoxView::decode(&data).unwrap();
        let owned = TrgrBox::try_from(&view).unwrap();

        assert_eq!(owned.track_groups.len(), 2);
        assert_eq!(owned.track_groups[0].0, BoxType::new(FourCC::new(*b"msrc")));
        assert_eq!(owned.track_groups[0].1.track_group_id, 1);
        assert_eq!(owned.track_groups[1].0, BoxType::new(FourCC::new(*b"cstg")));
        assert_eq!(owned.track_groups[1].1.track_group_id, 2);
    }
}
