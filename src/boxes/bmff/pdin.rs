use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::FixedSizeEntry;
use crate::iter::FixedSizeEntryIter;

define_box_flags!(
    /// Flags for the Progressive Download Information Box (`pdin`).
    PdinFlags {}
);

/// An entry in the Progressive Download Information Box (`pdin`).
#[derive(Debug, Clone, Copy)]
pub struct PdinEntry {
    /// The rate at which data is downloaded.
    pub rate: u32,
    /// The initial playback delay.
    pub initial_delay: u32,
}

impl FixedSizeEntry for PdinEntry {
    const ENTRY_SIZE: usize = 8;

    fn from_bytes(bytes: &[u8]) -> Self {
        debug_assert_eq!(bytes.len(), Self::ENTRY_SIZE);

        PdinEntry {
            rate: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            initial_delay: u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        }
    }

    fn to_bytes(&self, bytes: &mut [u8]) {
        debug_assert_eq!(bytes.len(), Self::ENTRY_SIZE);
        bytes[0..4].copy_from_slice(&self.rate.to_be_bytes());
        bytes[4..8].copy_from_slice(&self.initial_delay.to_be_bytes());
    }
}

/// A reference to a Progressive Download Information Box (`pdin`).
#[derive(Debug)]
pub struct PdinBoxView<'a> {
    /// Box version (0).
    pub version: u8,
    /// Box flags (should be 0).
    pub flags: PdinFlags,
    entries: &'a [u8],
}

impl<'a> PdinBoxView<'a> {
    /// Returns an iterator over the PDIN entries.
    pub fn entries(&self) -> FixedSizeEntryIter<'a, PdinEntry> {
        FixedSizeEntryIter::new(self.entries)
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
    #[derive(Debug, Clone)]
    pub struct PdinBox {
        /// Box version (0).
        pub version: u8,
        /// Box flags (should be 0).
        pub flags: PdinFlags,
        /// The PDIN entries.
        pub entries: Vec<PdinEntry>,
    }

    impl From<&PdinBoxView<'_>> for PdinBox {
        fn from(view: &PdinBoxView) -> Self {
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
            4 + self.entries.len() * PdinEntry::ENTRY_SIZE
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_be_bytes())?;

            for entry in &self.entries {
                let buf = cur.take_mut(PdinEntry::ENTRY_SIZE)?;
                entry.to_bytes(buf);
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

        let mut bytes = [0u8; 8];
        entry.to_bytes(&mut bytes);

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
