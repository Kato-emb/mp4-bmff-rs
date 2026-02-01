use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::FixedSizeEntry;
use crate::iter::FixedSizeEntryIter;

use crate::cursor::ReadCursor;

define_box_flags!(
    /// Flags for the Independent and Disposable Samples Box (`sdtp`).
    SdtpFlags {}
);

/// An entry in the Independent and Disposable Samples Box (`sdtp`).
#[derive(Debug, Clone, Copy)]
pub struct SdtpEntry {
    /// Whether the sample is leading.
    pub is_leading: u8,
    /// The sample depends on other samples.
    pub sample_depends_on: u8,
    /// The sample is depended on by other samples.
    pub sample_is_depended_on: u8,
    /// The sample has redundancy.
    pub sample_has_redundancy: u8,
}

impl FixedSizeEntry for SdtpEntry {
    const ENTRY_SIZE: usize = 1;

    fn from_bytes(bytes: &[u8]) -> Self {
        debug_assert_eq!(bytes.len(), Self::ENTRY_SIZE);
        let byte = bytes[0];

        SdtpEntry {
            is_leading: (byte & 0b1100_0000) >> 6,
            sample_depends_on: (byte & 0b0011_0000) >> 4,
            sample_is_depended_on: (byte & 0b0000_1100) >> 2,
            sample_has_redundancy: byte & 0b0000_0011,
        }
    }

    fn to_bytes(&self, bytes: &mut [u8]) {
        debug_assert_eq!(bytes.len(), Self::ENTRY_SIZE);
        let mut byte = 0u8;
        byte |= (self.is_leading & 0b11) << 6;
        byte |= (self.sample_depends_on & 0b11) << 4;
        byte |= (self.sample_is_depended_on & 0b11) << 2;
        byte |= self.sample_has_redundancy & 0b11;
        bytes[0] = byte;
    }
}

/// A reference to an Independent and Disposable Samples Box (`sdtp`).
#[derive(Debug)]
pub struct SdtpBoxView<'a> {
    /// Box version (0).
    pub version: u8,
    /// Box flags (should be 0).
    pub flags: SdtpFlags,
    /// Number of entries.
    pub entry_count: u32,
    entries: &'a [u8],
}

impl<'a> SdtpBoxView<'a> {
    /// Returns an iterator over the SDTP entries.
    pub fn entries(&self) -> FixedSizeEntryIter<'a, SdtpEntry> {
        FixedSizeEntryIter::new(self.entries)
    }
}

impl BoxCodec for SdtpBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::SDTP
    }
}

impl<'de> BoxDecode<'de> for SdtpBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = SdtpFlags::from_be_bytes(cur.read_array::<3>()?);

        let entry_count = cur.read_u32_be()?;

        let expected_size = entry_count as usize * SdtpEntry::ENTRY_SIZE;

        if cur.remaining() != expected_size {
            return Err(Error::at_in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Entries length does not match entry count",
                    got: cur.remaining() as u64,
                },
                cur.position() as u64,
                BoxType::SDTP,
            ));
        }

        let entries = cur.take(expected_size)?;

        Ok(SdtpBoxView {
            version,
            flags,
            entry_count,
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

    /// An owned Independent and Disposable Samples Box (`sdtp`).
    #[derive(Debug, Clone)]
    pub struct SdtpBox {
        /// Box version (0).
        pub version: u8,
        /// Box flags (should be 0).
        pub flags: SdtpFlags,
        /// Entries.
        pub entries: Vec<SdtpEntry>,
    }

    impl From<&SdtpBoxView<'_>> for SdtpBox {
        fn from(view: &SdtpBoxView<'_>) -> Self {
            let entries = view.entries().collect();
            SdtpBox {
                version: view.version,
                flags: view.flags,
                entries,
            }
        }
    }

    impl SdtpBoxView<'_> {
        /// Converts this box view into an owned box.
        pub fn to_owned(&self) -> SdtpBox {
            SdtpBox::from(self)
        }
    }

    impl BoxCodec for SdtpBox {
        fn boxtype(&self) -> BoxType {
            BoxType::SDTP
        }
    }

    impl BoxDecode<'_> for SdtpBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = SdtpBoxView::decode(bytes)?;
            Ok(SdtpBox::from(&view))
        }
    }

    impl BoxEncode for SdtpBox {
        fn encoded_len(&self) -> usize {
            4 // version + flags
            + 4 // entry count
            + self.entries.len() * SdtpEntry::ENTRY_SIZE // entries
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_be_bytes())?;

            cur.write_u32_be(self.entries.len() as u32)?;

            for entry in &self.entries {
                let buf = cur.take_mut(SdtpEntry::ENTRY_SIZE)?;
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

    fn raw_data() -> [u8; 10] {
        [
            0x00,                   // version = 0
            0x00, 0x00, 0x00,       // flags (3 bytes)
            0x00, 0x00, 0x00, 0x02, // entry_count = 2
            // entry 1: is_leading=3, sample_depends_on=2, sample_is_depended_on=1, sample_has_redundancy=0
            0b11_10_01_00, // 0xE4
            // entry 2: is_leading=0, sample_depends_on=1, sample_is_depended_on=2, sample_has_redundancy=3
            0b00_01_10_11, // 0x1B
        ]
    }

    #[test]
    fn test_sdtp_box_view_decode() {
        let data = raw_data();
        let sdtp = SdtpBoxView::decode(&data).unwrap();

        assert_eq!(sdtp.version, 0);
        assert_eq!(sdtp.flags.bits(), 0);
        assert_eq!(sdtp.entry_count, 2);

        let mut entries = sdtp.entries();
        let first = entries.next().unwrap();
        assert_eq!(first.is_leading, 3);
        assert_eq!(first.sample_depends_on, 2);
        assert_eq!(first.sample_is_depended_on, 1);
        assert_eq!(first.sample_has_redundancy, 0);

        let second = entries.next().unwrap();
        assert_eq!(second.is_leading, 0);
        assert_eq!(second.sample_depends_on, 1);
        assert_eq!(second.sample_is_depended_on, 2);
        assert_eq!(second.sample_has_redundancy, 3);

        assert!(entries.next().is_none());
    }

    #[test]
    fn test_sdtp_box_view_empty_entries() {
        let data: [u8; 8] = [
            0x00,                   // version = 0
            0x00, 0x00, 0x00,       // flags (3 bytes)
            0x00, 0x00, 0x00, 0x00, // entry_count = 0
        ];

        let sdtp = SdtpBoxView::decode(&data).unwrap();
        assert_eq!(sdtp.version, 0);
        assert_eq!(sdtp.entry_count, 0);
        assert_eq!(sdtp.entries().count(), 0);
    }

    #[test]
    fn test_sdtp_box_view_invalid_size() {
        // entry_count = 3 but only 2 entries provided
        let data: [u8; 10] = [
            0x00,                   // version = 0
            0x00, 0x00, 0x00,       // flags (3 bytes)
            0x00, 0x00, 0x00, 0x03, // entry_count = 3
            0xE4, 0x1B,             // only 2 entries
        ];

        let result = SdtpBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_sdtp_box_view_truncated() {
        let data: [u8; 4] = [0x00; 4]; // too short

        let result = SdtpBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_sdtp_entry_round_trip() {
        let entry = SdtpEntry {
            is_leading: 2,
            sample_depends_on: 1,
            sample_is_depended_on: 3,
            sample_has_redundancy: 0,
        };

        let mut bytes = [0u8; 1];
        entry.to_bytes(&mut bytes);

        let decoded = SdtpEntry::from_bytes(&bytes);
        assert_eq!(decoded.is_leading, entry.is_leading);
        assert_eq!(decoded.sample_depends_on, entry.sample_depends_on);
        assert_eq!(decoded.sample_is_depended_on, entry.sample_is_depended_on);
        assert_eq!(decoded.sample_has_redundancy, entry.sample_has_redundancy);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_sdtp_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_data();
        let sdtp = SdtpBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; sdtp.encoded_len()];
        sdtp.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_sdtp_box_to_owned() {
        let data = raw_data();
        let view = SdtpBoxView::decode(&data).unwrap();
        let owned = view.to_owned();

        assert_eq!(owned.version, view.version);
        assert_eq!(owned.flags.bits(), view.flags.bits());
        assert_eq!(owned.entries.len(), view.entry_count as usize);
    }
}
