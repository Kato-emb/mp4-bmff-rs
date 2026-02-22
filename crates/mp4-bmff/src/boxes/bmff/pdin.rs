//! Progressive Download Information Box (`pdin`) implementation.
//!
//! The Progressive Download Information Box provides information about
//! the rate at which data will be received and the initial playback delay
//! that will be needed. This allows players to estimate when enough data
//! will be available to begin playback.

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::FixedEntry;

define_box_flags!(
    /// Flags for the Progressive Download Information Box (`pdin`).
    PdinFlags {}
);

/// An entry in the Progressive Download Information Box (`pdin`).
///
/// Each entry describes the playback characteristics at a specific download rate,
/// allowing clients to determine the required buffering time before playback
/// can begin without interruption.
///
/// # Structure
///
/// - `rate`: Download rate in bytes per second.
/// - `initial_delay`: Required buffering delay in milliseconds at this rate.
#[derive(Debug, Clone, Copy)]
pub struct PdinEntry {
    /// The download rate in bytes per second.
    pub rate: u32,
    /// The initial playback delay in milliseconds required to buffer
    /// enough data for uninterrupted playback at the given rate.
    pub initial_delay: u32,
}

impl FixedEntry<8> for PdinEntry {
    fn from_bytes(bytes: &[u8; 8]) -> Self {
        let rate = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let initial_delay = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);

        PdinEntry {
            rate,
            initial_delay,
        }
    }

    #[cfg(feature = "alloc")]
    fn to_bytes(&self) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        bytes[0..4].copy_from_slice(&self.rate.to_be_bytes());
        bytes[4..8].copy_from_slice(&self.initial_delay.to_be_bytes());
        bytes
    }
}

define_entry_iter!(
    /// An iterator over entries in the Progressive Download Information Box (`pdin`).
    pub struct PdinEntryIter(PdinEntry, 8);
);

/// A reference to a Progressive Download Information Box (`pdin`).
///
/// This optional box provides information needed for progressive download
/// playback. It contains pairs of download rate and initial delay values
/// that allow clients to estimate when playback can begin based on the
/// current network conditions.
///
/// This box should be placed as early as possible in the file, ideally
/// before the Movie Box (`moov`), so clients can read it quickly.
///
/// # Structure
///
/// - `version`: Box version, should be 0.
/// - `flags`: Box flags, should be 0.
/// - `entries`: List of rate/delay pairs for different download scenarios.
#[derive(Debug)]
pub struct PdinBoxView<'a> {
    /// Box version (should be 0).
    pub version: u8,
    /// Box flags (should be 0).
    pub flags: PdinFlags,
    entries: &'a [u8],
}

impl<'a> PdinBoxView<'a> {
    /// Returns an iterator over the PDIN entries.
    pub fn entries(&self) -> PdinEntryIter<'a> {
        PdinEntryIter::new(self.entries)
    }
}

impl BoxCodec for PdinBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::PDIN
    }
}

impl<'de> BoxDecode<'de> for PdinBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = crate::cursor::ReadCursor::new(bytes);

        // Read version (1 byte)
        let version = cur.read_u8()?;

        // Read flags (3 bytes)
        let flags = PdinFlags::from_be_bytes(cur.read_array::<3>()?);

        if !cur.remaining().is_multiple_of(PdinEntry::ENTRY_SIZE) {
            return Err(Error::at_in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "PDIN entries length is not a multiple of entry size",
                    got: cur.remaining() as u64,
                },
                cur.position() as u64,
                BoxType::PDIN,
            ));
        }

        // The remaining bytes are entries
        let entries = cur.take(cur.remaining())?;

        Ok(PdinBoxView {
            version,
            flags,
            entries,
        })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use super::*;
    use crate::BoxEncode;

    use crate::cursor::WriteCursor;

    /// An owned Progressive Download Information Box (`pdin`).
    ///
    /// This is the owned variant of [`PdinBoxView`] that stores entries
    /// in a heap-allocated vector.
    ///
    /// # Example
    ///
    /// ```
    /// use mp4_bmff::boxes::bmff::{PdinBox, PdinEntry};
    /// use mp4_bmff::BoxDecode;
    ///
    /// // Decode from raw bytes
    /// let data = [
    ///     0x00,                   // version
    ///     0x00, 0x00, 0x00,       // flags
    ///     0x00, 0x01, 0x86, 0xA0, // rate = 100,000 bytes/sec
    ///     0x00, 0x00, 0x07, 0xD0, // initial_delay = 2000 ms
    /// ];
    /// let pdin = PdinBox::decode(&data).unwrap();
    /// assert_eq!(pdin.entries.len(), 1);
    /// ```
    #[derive(Debug, Clone)]
    pub struct PdinBox {
        /// Box version (should be 0).
        pub version: u8,
        /// Box flags (should be 0).
        pub flags: PdinFlags,
        /// List of rate/delay pairs for progressive download estimation.
        pub entries: Vec<PdinEntry>,
    }

    impl From<&PdinBoxView<'_>> for PdinBox {
        fn from(view: &PdinBoxView<'_>) -> Self {
            let entries = view.entries().collect();
            PdinBox {
                version: view.version,
                flags: view.flags,
                entries,
            }
        }
    }

    impl PdinBoxView<'_> {
        /// Converts this view into an owned `PdinBox`.
        pub fn to_owned(&self) -> PdinBox {
            PdinBox::from(self)
        }
    }

    impl BoxCodec for PdinBox {
        fn boxtype(&self) -> BoxType {
            BoxType::PDIN
        }
    }

    impl BoxDecode<'_> for PdinBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = PdinBoxView::decode(bytes)?;
            Ok(PdinBox::from(&view))
        }
    }

    impl BoxEncode for PdinBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            4 + self.entries.len() * PdinEntry::ENTRY_SIZE // 4 bytes for version + flags, 8 bytes per entry
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_be_bytes())?;

            for entry in &self.entries {
                let bytes = entry.to_bytes();
                cur.write_array(&bytes)?;
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

    fn raw_data() -> [u8; 20] {
        [
            0x00, // version
            0x00, 0x00, 0x00, // flags
            0x00, 0x00, 0x03, 0xE8, // entry 1: rate = 1000
            0x00, 0x00, 0x01, 0xF4, // entry 1: initial_delay = 500
            0x00, 0x00, 0x07, 0xD0, // entry 2: rate = 2000
            0x00, 0x00, 0x03, 0xE8, // entry 2: initial_delay = 1000
        ]
    }

    #[test]
    fn test_pdin_box_view_decode() {
        let data = raw_data();
        let pdin_box_view = PdinBoxView::decode(&data).unwrap();
        assert_eq!(pdin_box_view.version, 0);
        assert_eq!(pdin_box_view.flags.bits(), 0);

        let mut entries = pdin_box_view.entries();
        let first = entries.next().unwrap();
        assert_eq!(first.rate, 1000);
        assert_eq!(first.initial_delay, 500);
        let second = entries.next().unwrap();
        assert_eq!(second.rate, 2000);
        assert_eq!(second.initial_delay, 1000);
        assert!(entries.next().is_none());
    }

    #[test]
    fn test_pdin_box_view_empty_entries() {
        let data: [u8; 4] = [
            0x00, // version
            0x00, 0x00, 0x00, // flags
        ];

        let pdin_box_view = PdinBoxView::decode(&data).unwrap();
        assert_eq!(pdin_box_view.version, 0);
        assert_eq!(pdin_box_view.entries().count(), 0);
    }

    #[test]
    fn test_pdin_box_view_invalid_size() {
        // 7 bytes after header: not a multiple of 8 (entry size)
        let data: [u8; 11] = [
            0x00, // version
            0x00, 0x00, 0x00, // flags
            0x00, 0x00, 0x03, 0xE8, // partial entry
            0x00, 0x00, 0x01, // incomplete
        ];

        let result = PdinBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_pdin_box_view_truncated() {
        // Only 2 bytes, missing flags
        let data: [u8; 2] = [0x00, 0x00];

        let result = PdinBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_pdin_entry_round_trip() {
        let entry = PdinEntry {
            rate: 1000,
            initial_delay: 500,
        };

        let bytes = entry.to_bytes();

        let decoded = PdinEntry::from_bytes(&bytes);
        assert_eq!(decoded.rate, entry.rate);
        assert_eq!(decoded.initial_delay, entry.initial_delay);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_pdin_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_data();
        let pdin_box = PdinBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; pdin_box.encoded_len()];
        pdin_box.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_pdin_box_to_owned() {
        let data = raw_data();
        let view = PdinBoxView::decode(&data).unwrap();
        let owned = view.to_owned();

        assert_eq!(owned.version, view.version);
        assert_eq!(owned.flags.bits(), view.flags.bits());
        assert_eq!(owned.entries.len(), view.entries().count());
    }
}
