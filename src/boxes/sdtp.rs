use crate::cursor::ReadCursor;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::FixedSizeEntry;
use crate::iter::FixedSizeEntryIter;

use super::FullBoxFlags;

/// An entry in the independen and Disposable Samples Box (`sdtp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SdtpEntry {
    /// Sample is a leading sample.
    pub is_leading: u8,
    /// Sample depends on other samples.
    pub sample_depends_on: u8,
    /// Sample is depended on by other samples.
    pub sample_is_depended_on: u8,
    /// Sample has redundancy.
    pub sample_has_redundancy: u8,
}

impl FixedSizeEntry for SdtpEntry {
    const ENTRY_SIZE: usize = 1;

    fn from_bytes(bytes: &[u8]) -> Self {
        debug_assert_eq!(bytes.len(), Self::ENTRY_SIZE);
        let byte = bytes[0];
        SdtpEntry {
            is_leading: (byte >> 6) & 0x03,
            sample_depends_on: (byte >> 4) & 0x03,
            sample_is_depended_on: (byte >> 2) & 0x03,
            sample_has_redundancy: byte & 0x03,
        }
    }

    fn to_bytes(&self, bytes: &mut [u8]) {
        debug_assert_eq!(bytes.len(), Self::ENTRY_SIZE);
        bytes[0] = (self.is_leading << 6)
            | (self.sample_depends_on << 4)
            | (self.sample_is_depended_on << 2)
            | self.sample_has_redundancy;
    }
}

/// A reference to the independen and Disposable Samples Box (`sdtp`).
#[derive(Debug)]
pub struct SdtpBoxView<'a> {
    /// The version of the box (0).
    pub version: u8,
    /// The flags of the box.
    pub flags: SdtpFlags,
    entries: &'a [u8],
}

impl<'a> SdtpBoxView<'a> {
    /// Returns an iterator over the entries in the Disposable Samples Box.
    pub fn entries(&self) -> FixedSizeEntryIter<'a, SdtpEntry> {
        FixedSizeEntryIter::new(self.entries)
    }
}

/// Specification for the independen and Disposable Samples Box (`sdtp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SdtpSpec;

/// Flags for the independen and Disposable Samples Box (`sdtp`).
pub type SdtpFlags = FullBoxFlags<SdtpSpec>;

impl BoxCodec for SdtpBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::SDTP
    }
}

impl<'de> BoxDecode<'de> for SdtpBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = SdtpFlags::from_bytes(cur.read_array()?);

        if version != 0 {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxVersion {
                    reason: "sdtp version must be 0",
                    got: version,
                },
                BoxType::SDTP,
            ));
        }

        let entries = cur.take(cur.remaining())?;

        Ok(SdtpBoxView {
            version,
            flags,
            entries,
        })
    }
}

impl<'a> TryFrom<&'a [u8]> for SdtpBoxView<'a> {
    type Error = Error;

    fn try_from(bytes: &'a [u8]) -> Result<Self> {
        SdtpBoxView::decode(bytes)
    }
}

#[cfg(feature = "alloc")]
pub use owned::SdtpBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use crate::BoxEncode;
    use crate::cursor::WriteCursor;

    use super::*;

    /// Owned version of the independen and Disposable Samples Box (`sdtp`).
    #[derive(Debug, Clone)]
    pub struct SdtpBox {
        /// The version of the box (0).
        pub version: u8,
        /// The flags of the box.
        pub flags: SdtpFlags,
        /// The entries in the box.
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
        #[inline]
        fn encoded_len(&self) -> usize {
            1 // version
                + 3 // flags
                + self.entries.len() * SdtpEntry::ENTRY_SIZE // entries
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_bytes())?;

            for entry in &self.entries {
                let mut entry_bytes = [0u8; SdtpEntry::ENTRY_SIZE];
                entry.to_bytes(&mut entry_bytes);
                cur.write_array(&entry_bytes)?;
            }

            Ok(cur.position())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sdtp_payload(version: u8, flags: u32, entries: &[SdtpEntry]) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(version);
        data.extend_from_slice(&flags.to_be_bytes()[1..4]);
        for entry in entries {
            let mut entry_bytes = [0u8; SdtpEntry::ENTRY_SIZE];
            entry.to_bytes(&mut entry_bytes);
            data.extend_from_slice(&entry_bytes);
        }
        data
    }

    #[test]
    fn entry_from_bytes_and_to_bytes() {
        // Test bit packing: is_leading=2, sample_depends_on=1, sample_is_depended_on=3, sample_has_redundancy=0
        // Binary: 10 01 11 00 = 0x9C
        let bytes = [0x9C];
        let entry = SdtpEntry::from_bytes(&bytes);
        assert_eq!(entry.is_leading, 2);
        assert_eq!(entry.sample_depends_on, 1);
        assert_eq!(entry.sample_is_depended_on, 3);
        assert_eq!(entry.sample_has_redundancy, 0);

        // Round-trip
        let mut out = [0u8; 1];
        entry.to_bytes(&mut out);
        assert_eq!(out[0], 0x9C);

        // All zeros
        let zeros = SdtpEntry::from_bytes(&[0x00]);
        assert_eq!(zeros.is_leading, 0);
        assert_eq!(zeros.sample_depends_on, 0);
        assert_eq!(zeros.sample_is_depended_on, 0);
        assert_eq!(zeros.sample_has_redundancy, 0);

        // All max (0xFF = 11 11 11 11)
        let max = SdtpEntry::from_bytes(&[0xFF]);
        assert_eq!(max.is_leading, 3);
        assert_eq!(max.sample_depends_on, 3);
        assert_eq!(max.sample_is_depended_on, 3);
        assert_eq!(max.sample_has_redundancy, 3);
    }

    #[test]
    fn parse_empty_entries() {
        let payload = make_sdtp_payload(0, 0, &[]);
        let view = SdtpBoxView::decode(&payload).unwrap();
        assert_eq!(view.version, 0);
        assert_eq!(view.entries().count(), 0);
    }

    #[test]
    fn parse_and_iterate_entries() {
        let entries = [
            SdtpEntry {
                is_leading: 0,
                sample_depends_on: 2,
                sample_is_depended_on: 1,
                sample_has_redundancy: 0,
            },
            SdtpEntry {
                is_leading: 1,
                sample_depends_on: 1,
                sample_is_depended_on: 2,
                sample_has_redundancy: 1,
            },
            SdtpEntry {
                is_leading: 2,
                sample_depends_on: 0,
                sample_is_depended_on: 0,
                sample_has_redundancy: 2,
            },
        ];
        let payload = make_sdtp_payload(0, 0, &entries);
        let view = SdtpBoxView::decode(&payload).unwrap();

        let parsed: Vec<_> = view.entries().collect();
        assert_eq!(parsed.len(), 3);

        assert_eq!(parsed[0].is_leading, 0);
        assert_eq!(parsed[0].sample_depends_on, 2);
        assert_eq!(parsed[0].sample_is_depended_on, 1);
        assert_eq!(parsed[0].sample_has_redundancy, 0);

        assert_eq!(parsed[1].is_leading, 1);
        assert_eq!(parsed[1].sample_depends_on, 1);
        assert_eq!(parsed[1].sample_is_depended_on, 2);
        assert_eq!(parsed[1].sample_has_redundancy, 1);

        assert_eq!(parsed[2].is_leading, 2);
        assert_eq!(parsed[2].sample_depends_on, 0);
        assert_eq!(parsed[2].sample_is_depended_on, 0);
        assert_eq!(parsed[2].sample_has_redundancy, 2);
    }

    #[test]
    fn invalid_version() {
        let payload = make_sdtp_payload(1, 0, &[]);
        assert!(SdtpBoxView::decode(&payload).is_err());

        let payload = make_sdtp_payload(2, 0, &[]);
        assert!(SdtpBoxView::decode(&payload).is_err());
    }

    #[test]
    fn try_from_slice() {
        let payload = make_sdtp_payload(0, 0, &[]);
        let view: SdtpBoxView = (&payload[..]).try_into().unwrap();
        assert_eq!(view.version, 0);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn owned_conversion() {
        let entries = [
            SdtpEntry {
                is_leading: 1,
                sample_depends_on: 2,
                sample_is_depended_on: 1,
                sample_has_redundancy: 0,
            },
            SdtpEntry {
                is_leading: 0,
                sample_depends_on: 1,
                sample_is_depended_on: 2,
                sample_has_redundancy: 3,
            },
        ];
        let payload = make_sdtp_payload(0, 0, &entries);
        let view = SdtpBoxView::decode(&payload).unwrap();
        let owned = SdtpBox::from(&view);

        assert_eq!(owned.version, 0);
        assert_eq!(owned.entries.len(), 2);
        assert_eq!(owned.entries[0], entries[0]);
        assert_eq!(owned.entries[1], entries[1]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn round_trip_encode_decode() {
        use crate::BoxEncode;

        let entries = vec![
            SdtpEntry {
                is_leading: 2,
                sample_depends_on: 1,
                sample_is_depended_on: 0,
                sample_has_redundancy: 3,
            },
            SdtpEntry {
                is_leading: 0,
                sample_depends_on: 2,
                sample_is_depended_on: 2,
                sample_has_redundancy: 0,
            },
            SdtpEntry {
                is_leading: 3,
                sample_depends_on: 3,
                sample_is_depended_on: 3,
                sample_has_redundancy: 3,
            },
        ];

        let original = SdtpBox {
            version: 0,
            flags: SdtpFlags::default(),
            entries: entries.clone(),
        };

        // Encode
        let mut buf = vec![0u8; original.encoded_len()];
        let written = original.encode_into(&mut buf).unwrap();
        assert_eq!(written, original.encoded_len());

        // Decode and compare
        let decoded = SdtpBox::decode(&buf[..written]).unwrap();
        assert_eq!(decoded.version, original.version);
        assert_eq!(decoded.entries.len(), original.entries.len());
        for (i, entry) in decoded.entries.iter().enumerate() {
            assert_eq!(*entry, entries[i]);
        }
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn encode_buffer_too_small() {
        use crate::BoxEncode;

        let sdtp = SdtpBox {
            version: 0,
            flags: SdtpFlags::default(),
            entries: vec![SdtpEntry {
                is_leading: 0,
                sample_depends_on: 0,
                sample_is_depended_on: 0,
                sample_has_redundancy: 0,
            }],
        };

        let mut small_buf = vec![0u8; 2];
        assert!(sdtp.encode_into(&mut small_buf).is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn boxtype_returns_sdtp() {
        use crate::BoxCodec;

        let view_payload = make_sdtp_payload(0, 0, &[]);
        let view = SdtpBoxView::decode(&view_payload).unwrap();
        assert_eq!(view.boxtype(), BoxType::SDTP);

        let owned = SdtpBox {
            version: 0,
            flags: SdtpFlags::default(),
            entries: vec![],
        };
        assert_eq!(owned.boxtype(), BoxType::SDTP);
    }
}
