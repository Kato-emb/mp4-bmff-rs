//! Sample Description Box (`stsd`) implementation.
//!
//! The Sample Description Box gives detailed information about the coding type
//! used, and any initialization information needed for that coding. Each sample
//! entry describes the format of a coded sample; multiple entries allow
//! different formats within a single track.

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

use crate::cursor::ReadCursor;

define_box_flags!(
    /// Flags for Sample Description Box (`stsd`).
    ///
    /// Reserved (should be 0).
    StsdFlags {}
);

/// A reference to a Sample Description Box (`stsd`).
///
/// The Sample Description Box contains codec-specific configuration for the
/// samples in this track. Each sample entry (identified by sample_description_index
/// in other tables) describes how to decode a subset of samples.
///
/// # Common Sample Entry Types
///
/// Video: `avc1`, `avc3`, `hvc1`, `hev1`, `av01`, `vp09`
/// Audio: `mp4a`, `ac-3`, `ec-3`, `Opus`, `fLaC`
/// Text: `tx3g`, `wvtt`
///
/// # Structure
///
/// - `version`: Box version (should be 0).
/// - `flags`: Reserved (should be 0).
/// - `entry_count`: Number of sample entries.
/// - `entries`: Array of codec-specific sample entry boxes.
#[derive(Debug)]
pub struct StsdBoxView<'a> {
    /// Box version.
    pub version: u8,
    /// Box flags (should be 0).
    pub flags: StsdFlags,
    /// Number of sample entries.
    pub entry_count: u32,
    entries: &'a [u8],
}

impl<'a> StsdBoxView<'a> {
    /// Returns the raw bytes of the sample entries contained in this `stsd` box.
    pub fn sample_entries(&self) -> BoxIter<'a> {
        BoxIter::new(self.entries)
    }
}

impl BoxCodec for StsdBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::STSD
    }
}

impl<'de> BoxDecode<'de> for StsdBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = StsdFlags::from_be_bytes(cur.read_array::<3>()?);

        // Read entry_count (4 bytes)
        let entry_count = cur.read_u32_be()?;

        let entries = cur.take(cur.remaining())?;

        Ok(StsdBoxView {
            version,
            flags,
            entry_count,
            entries,
        })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use alloc::vec::Vec;

    use super::*;
    use crate::BoxEncode;
    use crate::RawBoxOwned;

    use crate::cursor::WriteCursor;

    /// An owned Sample Description Box (`stsd`).
    ///
    /// This is the owned variant of [`StsdBoxView`] that stores sample entries
    /// in a heap-allocated vector of raw boxes.
    ///
    /// # Structure
    ///
    /// - `version`: Box version (should be 0).
    /// - `flags`: Reserved (should be 0).
    /// - `entries`: Codec-specific sample entry boxes (stored as raw boxes).
    ///
    /// Note: Sample entries are stored as [`RawBoxOwned`] because the specific
    /// entry format varies by codec. Use codec-specific parsers to decode
    /// individual entries (e.g., `Mp4aBox` for MPEG-4 audio, `Avc1Box` for H.264).
    #[derive(Debug, Clone, Default)]
    pub struct StsdBox {
        /// Box version (should be 0).
        pub version: u8,
        /// Reserved flags (should be 0).
        pub flags: StsdFlags,
        /// Sample entries describing the formats used in this track.
        pub entries: Vec<RawBoxOwned>,
    }

    impl TryFrom<&StsdBoxView<'_>> for StsdBox {
        type Error = Error;

        fn try_from(view: &StsdBoxView<'_>) -> Result<Self> {
            let mut entries = Vec::with_capacity(view.entry_count as usize);

            for b in view.sample_entries() {
                let b = b?;
                entries.push(b.to_owned());
            }

            Ok(StsdBox {
                version: view.version,
                flags: view.flags,
                entries,
            })
        }
    }

    impl BoxCodec for StsdBox {
        fn boxtype(&self) -> BoxType {
            BoxType::STSD
        }
    }

    impl BoxDecode<'_> for StsdBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = StsdBoxView::decode(bytes)?;
            StsdBox::try_from(&view)
        }
    }

    impl BoxEncode for StsdBox {
        fn encoded_len(&self) -> usize {
            let mut len = 1 + 3; // version + flags
            len += 4; // entry_count

            for entry in &self.entries {
                len +=
                    usize::try_from(entry.header().total_size()).expect("Box size exceeds usize");
            }

            len
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_be_bytes())?;

            cur.write_u32_be(u32::try_from(self.entries.len())?)?;

            for entry in &self.entries {
                let buf = cur.take_mut(entry.len())?;
                entry.write(buf)?;
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
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x00, 0x00, 0x01, // entry_count = 1
            // Sample entry (box format):
            0x00, 0x00, 0x00, 0x10, // size = 16
            b't', b'e', b's', b't', // type = "test"
            0x01, 0x02, 0x03, 0x04, // payload (8 bytes)
            0x05, 0x06, 0x07, 0x08,
        ]
    }

    #[test]
    fn test_stsd_box_view_decode() {
        let data = raw_data();
        let stsd = StsdBoxView::decode(&data).unwrap();

        assert_eq!(stsd.version, 0);
        assert_eq!(stsd.flags.bits(), 0);
        assert_eq!(stsd.entry_count, 1);

        let mut entries = stsd.sample_entries();
        let entry = entries.next().unwrap().unwrap();
        assert_eq!(
            entry.header().boxtype(),
            BoxType::from(FourCC::new(*b"test"))
        );
        assert!(entries.next().is_none());
    }

    #[test]
    fn test_stsd_box_view_empty_entries() {
        let data: [u8; 8] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x00, 0x00, 0x00, // entry_count = 0
        ];

        let stsd = StsdBoxView::decode(&data).unwrap();
        assert_eq!(stsd.version, 0);
        assert_eq!(stsd.entry_count, 0);
        assert!(stsd.sample_entries().next().is_none());
    }

    #[test]
    fn test_stsd_box_view_truncated() {
        let data: [u8; 4] = [0x00; 4]; // too short

        let result = StsdBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_stsd_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_data();
        let stsd = StsdBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; stsd.encoded_len()];
        stsd.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_stsd_box_try_from() {
        let data = raw_data();
        let view = StsdBoxView::decode(&data).unwrap();
        let owned = StsdBox::try_from(&view).unwrap();

        assert_eq!(owned.version, view.version);
        assert_eq!(owned.flags.bits(), view.flags.bits());
        assert_eq!(owned.entries.len(), view.entry_count as usize);
    }
}
