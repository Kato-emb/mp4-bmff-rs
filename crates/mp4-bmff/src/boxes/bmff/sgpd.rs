use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::types::FourCC;

use crate::cursor::ReadCursor;

/// An iterator over entries in the Sample Group Description Box (`sgpd`).
///
/// Yields each `SampleGroupEntry` as an uninterpreted `&[u8]`.
/// Handles both fixed-length (`default_length > 0`) and
/// variable-length (`default_length == 0`) framing for version >= 1.
#[derive(Debug)]
pub struct SgpdEntryIter<'a> {
    entries: &'a [u8],
    entry_count: u32,
    default_length: u32,
}

impl<'a> Iterator for SgpdEntryIter<'a> {
    type Item = Result<&'a [u8]>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.entry_count == 0 {
            return None;
        }

        if self.default_length > 0 {
            // Fixed-length: take default_length bytes
            let len = self.default_length as usize;
            if self.entries.len() < len {
                self.entry_count = 0;
                return Some(Err(Error::in_box(
                    ErrorKind::NotEnoughBytes {
                        expected: len,
                        remaining: self.entries.len(),
                    },
                    BoxType::SGPD,
                )));
            }

            let (entry, rest) = self.entries.split_at(len);
            self.entries = rest;
            self.entry_count -= 1;
            Some(Ok(entry))
        } else {
            // Variable-length: read description_length prefix
            if self.entries.len() < 4 {
                self.entry_count = 0;
                return Some(Err(Error::in_box(
                    ErrorKind::NotEnoughBytes {
                        expected: 4,
                        remaining: self.entries.len(),
                    },
                    BoxType::SGPD,
                )));
            }

            let len = u32::from_be_bytes(
                self.entries[..4]
                    .try_into()
                    .expect("slice is exactly 4 bytes"),
            ) as usize;
            self.entries = &self.entries[4..];

            if self.entries.len() < len {
                self.entry_count = 0;
                return Some(Err(Error::in_box(
                    ErrorKind::NotEnoughBytes {
                        expected: len,
                        remaining: self.entries.len(),
                    },
                    BoxType::SGPD,
                )));
            }

            let (entry, rest) = self.entries.split_at(len);
            self.entries = rest;
            self.entry_count -= 1;
            Some(Ok(entry))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.entry_count as usize;
        if self.default_length > 0 {
            // Fixed-length: exact count is known
            (remaining, Some(remaining))
        } else {
            // Variable-length: upper bound is entry_count, but errors may reduce it
            (0, Some(remaining))
        }
    }
}

define_box_flags!(
    /// Flags for the Sample Group Description Box (`sgpd`).
    ///
    /// Reserved (should be 0).
    SgpdFlags {}
);

/// A reference to a Sample Group Description Box (`sgpd`).
///
/// Contains sample group description entries whose format depends on `grouping_type`.
///
/// # Version behavior
///
/// - **version 0** (deprecated): Entry size is determined by `grouping_type`.
///   No framing information is present; use [`entry_bytes()`](Self::entry_bytes).
/// - **version >= 1**: `default_length` field enables self-describing framing.
///   Use [`sample_group_entries()`](Self::sample_group_entries) to iterate.
/// - **version >= 2**: Adds `default_sample_description_index`.
#[derive(Debug)]
pub struct SgpdBoxView<'a> {
    /// Box version.
    pub version: u8,
    /// Reserved flags (should be 0).
    pub flags: SgpdFlags,
    /// A 4-character code identifying the grouping type (e.g., `"roll"`, `"rap "`).
    pub grouping_type: FourCC,
    /// Default entry length in bytes (version >= 1 only).
    /// `Some(0)` means each entry carries its own `description_length` prefix.
    pub default_length: Option<u32>,
    /// Default sample description index (version >= 2 only).
    pub default_sample_description_index: Option<u32>,
    /// Number of entries.
    pub entry_count: u32,
    entries: &'a [u8],
}

impl<'a> SgpdBoxView<'a> {
    /// Returns the raw bytes of all sample group entries without framing.
    ///
    /// For version 0 this is the only way to access entries, since
    /// entry boundaries depend on `grouping_type` semantics.
    pub fn entry_bytes(&self) -> &'a [u8] {
        self.entries
    }

    /// Returns an iterator over the sample group entries (version >= 1 only).
    ///
    /// Returns `None` for version 0, where entry framing requires
    /// external knowledge of the `grouping_type`.
    pub fn sample_group_entries(&self) -> Option<SgpdEntryIter<'a>> {
        self.default_length.map(|dl| SgpdEntryIter {
            entries: self.entries,
            entry_count: self.entry_count,
            default_length: dl,
        })
    }
}

impl BoxCodec for SgpdBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::SGPD
    }
}

impl<'de> BoxDecode<'de> for SgpdBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = SgpdFlags::from_be_bytes(cur.read_array::<3>()?);
        let grouping_type = FourCC::from(cur.read_array::<4>()?);

        let default_length = if version >= 1 {
            Some(cur.read_u32_be()?)
        } else {
            None
        };

        let default_sample_description_index = if version >= 2 {
            Some(cur.read_u32_be()?)
        } else {
            None
        };

        let entry_count = cur.read_u32_be()?;

        // Validate fixed-length entry data size when possible
        if let Some(dl) = default_length {
            if dl > 0 {
                let expected_size = entry_count as usize * dl as usize;
                if cur.remaining() != expected_size {
                    return Err(Error::at_in_box(
                        ErrorKind::InvalidBoxSize {
                            reason: "Entries length does not match entry_count * default_length",
                            got: cur.remaining() as u64,
                        },
                        cur.position() as u64,
                        BoxType::SGPD,
                    ));
                }
            }
        }

        let entries = cur.take(cur.remaining())?;

        Ok(SgpdBoxView {
            version,
            flags,
            grouping_type,
            default_length,
            default_sample_description_index,
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

    use crate::cursor::WriteCursor;

    /// An owned Sample Group Description Box (`sgpd`).
    ///
    /// For version >= 1, entries are stored individually as `Vec<Vec<u8>>`.
    /// For version 0 (deprecated), entries are stored as a single raw byte blob
    /// since entry boundaries cannot be determined without `grouping_type` knowledge.
    #[derive(Debug, Clone)]
    pub struct SgpdBox {
        /// Box version.
        pub version: u8,
        /// Reserved flags (should be 0).
        pub flags: SgpdFlags,
        /// A 4-character code identifying the grouping type.
        pub grouping_type: FourCC,
        /// Default entry length in bytes (version >= 1 only).
        pub default_length: Option<u32>,
        /// Default sample description index (version >= 2 only).
        pub default_sample_description_index: Option<u32>,
        /// Number of entries.
        ///
        /// For version >= 1 this is derived from `entries.len()` during encoding.
        /// For version 0 (deprecated), this preserves the original count since
        /// `entries` contains a single blob whose boundaries are unknown.
        pub entry_count: u32,
        /// Sample group description entries.
        ///
        /// For version >= 1, each element is one entry's raw bytes.
        /// For version 0, this contains a single element with all entry bytes concatenated.
        pub entries: Vec<Vec<u8>>,
    }

    impl TryFrom<&SgpdBoxView<'_>> for SgpdBox {
        type Error = Error;

        fn try_from(view: &SgpdBoxView<'_>) -> Result<Self> {
            let entries = if let Some(iter) = view.sample_group_entries() {
                iter.map(|res| res.map(|entry| entry.to_vec()))
                    .collect::<Result<Vec<_>>>()?
            } else {
                // version 0: store all entry bytes as a single blob
                alloc::vec![view.entry_bytes().to_vec()]
            };

            Ok(SgpdBox {
                version: view.version,
                flags: view.flags,
                grouping_type: view.grouping_type,
                default_length: view.default_length,
                default_sample_description_index: view.default_sample_description_index,
                entry_count: view.entry_count,
                entries,
            })
        }
    }

    impl BoxCodec for SgpdBox {
        fn boxtype(&self) -> BoxType {
            BoxType::SGPD
        }
    }

    impl BoxDecode<'_> for SgpdBox {
        fn decode(bytes: &'_ [u8]) -> Result<Self> {
            let view = SgpdBoxView::decode(bytes)?;
            SgpdBox::try_from(&view)
        }
    }

    impl BoxEncode for SgpdBox {
        fn encoded_len(&self) -> usize {
            let mut len = 1 + 3; // version + flags
            len += 4; // grouping_type
            if self.version >= 1 {
                len += 4; // default_length
            }
            if self.version >= 2 {
                len += 4; // default_sample_description_index
            }
            len += 4; // entry_count

            if self.version >= 1 && self.default_length == Some(0) {
                // Variable-length: each entry has a 4-byte description_length prefix
                for entry in &self.entries {
                    len += 4 + entry.len();
                }
            } else {
                // Fixed-length (version >= 1, default_length > 0) or version 0: no prefix
                for entry in &self.entries {
                    len += entry.len();
                }
            }

            len
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_be_bytes())?;

            cur.write_array(self.grouping_type.as_bytes())?;

            if self.version >= 1 {
                let default_length = self.default_length.ok_or_else(|| {
                    Error::in_box(
                        ErrorKind::InvalidBoxField {
                            field: "default_length",
                            reason: "default_length is required for version >= 1",
                        },
                        BoxType::SGPD,
                    )
                })?;
                cur.write_u32_be(default_length)?;
            }

            if self.version >= 2 {
                let default_sample_description_index = self
                    .default_sample_description_index
                    .ok_or_else(|| {
                        Error::in_box(
                            ErrorKind::InvalidBoxField {
                                field: "default_sample_description_index",
                                reason:
                                    "default_sample_description_index is required for version >= 2",
                            },
                            BoxType::SGPD,
                        )
                    })?;
                cur.write_u32_be(default_sample_description_index)?;
            }

            // For version >= 1, derive entry_count from entries.len() to prevent desync.
            // For version 0, use the stored entry_count since entries is a single blob.
            let entry_count = if self.version >= 1 {
                u32::try_from(self.entries.len())?
            } else {
                self.entry_count
            };
            cur.write_u32_be(entry_count)?;

            let variable_length = self.version >= 1 && self.default_length == Some(0);

            for entry in &self.entries {
                if variable_length {
                    // Write description_length prefix
                    cur.write_u32_be(u32::try_from(entry.len())?)?;
                }
                cur.write_slice(entry)?;
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

    /// version=1, grouping_type="roll", default_length=2, 2 entries
    fn raw_v1_fixed() -> [u8; 20] {
        [
            0x01, // version = 1
            0x00, 0x00, 0x00, // flags = 0
            b'r', b'o', b'l', b'l', // grouping_type = "roll"
            0x00, 0x00, 0x00, 0x02, // default_length = 2
            0x00, 0x00, 0x00, 0x02, // entry_count = 2
            // Entry 1: roll_distance = -1 (0xFFFF)
            0xFF, 0xFF, // Entry 2: roll_distance = 2 (0x0002)
            0x00, 0x02,
        ]
    }

    /// version=1, grouping_type="seig", default_length=0, 2 entries with length prefixes
    fn raw_v1_variable() -> [u8; 28] {
        [
            0x01, // version = 1
            0x00, 0x00, 0x00, // flags = 0
            b's', b'e', b'i', b'g', // grouping_type = "seig"
            0x00, 0x00, 0x00, 0x00, // default_length = 0
            0x00, 0x00, 0x00, 0x02, // entry_count = 2
            // Entry 1: description_length = 3, data = [0xAA, 0xBB, 0xCC]
            0x00, 0x00, 0x00, 0x03, 0xAA, 0xBB, 0xCC,
            // Entry 2: description_length = 1, data = [0xDD]
            0x00, 0x00, 0x00, 0x01, 0xDD,
        ]
    }

    /// version=2, grouping_type="rap ", default_length=1, default_sample_description_index=1
    fn raw_v2() -> [u8; 22] {
        [
            0x02, // version = 2
            0x00, 0x00, 0x00, // flags = 0
            b'r', b'a', b'p', b' ', // grouping_type = "rap "
            0x00, 0x00, 0x00, 0x01, // default_length = 1
            0x00, 0x00, 0x00, 0x01, // default_sample_description_index = 1
            0x00, 0x00, 0x00, 0x02, // entry_count = 2
            0x80, // num_leading_samples_known=1, num_leading_samples=0
            0x83, // num_leading_samples_known=1, num_leading_samples=3
        ]
    }

    #[test]
    fn test_sgpd_box_view_decode() {
        let data = raw_v1_fixed();
        let sgpd = SgpdBoxView::decode(&data).unwrap();

        assert_eq!(sgpd.version, 1);
        assert_eq!(sgpd.flags.bits(), 0);
        assert_eq!(sgpd.grouping_type, FourCC::from(*b"roll"));
        assert_eq!(sgpd.default_length, Some(2));
        assert!(sgpd.default_sample_description_index.is_none());
        assert_eq!(sgpd.entry_count, 2);

        let mut iter = sgpd.sample_group_entries().unwrap();
        assert_eq!(iter.next().unwrap().unwrap(), &[0xFF, 0xFF]);
        assert_eq!(iter.next().unwrap().unwrap(), &[0x00, 0x02]);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_sgpd_box_view_decode_variable() {
        let data = raw_v1_variable();
        let sgpd = SgpdBoxView::decode(&data).unwrap();

        assert_eq!(sgpd.version, 1);
        assert_eq!(sgpd.default_length, Some(0));
        assert_eq!(sgpd.entry_count, 2);

        let mut iter = sgpd.sample_group_entries().unwrap();
        assert_eq!(iter.next().unwrap().unwrap(), &[0xAA, 0xBB, 0xCC]);
        assert_eq!(iter.next().unwrap().unwrap(), &[0xDD]);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_sgpd_box_view_decode_v2() {
        let data = raw_v2();
        let sgpd = SgpdBoxView::decode(&data).unwrap();

        assert_eq!(sgpd.version, 2);
        assert_eq!(sgpd.grouping_type, FourCC::from(*b"rap "));
        assert_eq!(sgpd.default_length, Some(1));
        assert_eq!(sgpd.default_sample_description_index, Some(1));
        assert_eq!(sgpd.entry_count, 2);

        let mut iter = sgpd.sample_group_entries().unwrap();
        assert_eq!(iter.next().unwrap().unwrap(), &[0x80]);
        assert_eq!(iter.next().unwrap().unwrap(), &[0x83]);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_sgpd_box_view_decode_v0() {
        let data: [u8; 14] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            b'r', b'o', b'l', b'l', // grouping_type = "roll"
            0x00, 0x00, 0x00, 0x02, // entry_count = 2
            0xFF, 0xFF, // raw entry bytes (no framing)
        ];

        let sgpd = SgpdBoxView::decode(&data).unwrap();

        assert_eq!(sgpd.version, 0);
        assert!(sgpd.default_length.is_none());
        assert!(sgpd.default_sample_description_index.is_none());
        assert_eq!(sgpd.entry_count, 2);
        assert!(sgpd.sample_group_entries().is_none());
        assert_eq!(sgpd.entry_bytes(), &[0xFF, 0xFF]);
    }

    #[test]
    fn test_sgpd_box_view_empty_entries() {
        let data: [u8; 16] = [
            0x01, // version = 1
            0x00, 0x00, 0x00, // flags = 0
            b'r', b'o', b'l', b'l', // grouping_type = "roll"
            0x00, 0x00, 0x00, 0x02, // default_length = 2
            0x00, 0x00, 0x00, 0x00, // entry_count = 0
        ];

        let sgpd = SgpdBoxView::decode(&data).unwrap();
        assert_eq!(sgpd.entry_count, 0);
        assert_eq!(sgpd.sample_group_entries().unwrap().count(), 0);
    }

    #[test]
    fn test_sgpd_box_view_truncated() {
        let data: [u8; 2] = [0x00, 0x00];
        assert!(SgpdBoxView::decode(&data).is_err());
    }

    #[test]
    fn test_sgpd_box_view_invalid_fixed_length_size() {
        // default_length=4 but only 3 bytes of entry data (entry_count=1, expected 4 bytes)
        let data: [u8; 19] = [
            0x01, // version = 1
            0x00, 0x00, 0x00, // flags = 0
            b'r', b'o', b'l', b'l', // grouping_type
            0x00, 0x00, 0x00, 0x04, // default_length = 4
            0x00, 0x00, 0x00, 0x01, // entry_count = 1
            0xAA, 0xBB, 0xCC, // only 3 bytes
        ];

        // decode should fail because remaining (3) != entry_count * default_length (4)
        assert!(SgpdBoxView::decode(&data).is_err());
    }

    #[test]
    fn test_sgpd_box_view_invalid_variable_length_entry() {
        // default_length=0, description_length=4 but only 3 bytes of data
        let data: [u8; 23] = [
            0x01, // version = 1
            0x00, 0x00, 0x00, // flags = 0
            b'r', b'o', b'l', b'l', // grouping_type
            0x00, 0x00, 0x00, 0x00, // default_length = 0
            0x00, 0x00, 0x00, 0x01, // entry_count = 1
            0x00, 0x00, 0x00, 0x04, // description_length = 4
            0xAA, 0xBB, 0xCC, // only 3 bytes
        ];

        let sgpd = SgpdBoxView::decode(&data).unwrap();
        let mut iter = sgpd.sample_group_entries().unwrap();
        assert!(iter.next().unwrap().is_err());
        assert!(iter.next().is_none());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_sgpd_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_v1_fixed();
        let owned = SgpdBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; owned.encoded_len()];
        owned.encode_into(&mut encoded).unwrap();
        assert_eq!(encoded, original);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_sgpd_box_round_trip_variable() {
        use crate::BoxEncode;

        let original = raw_v1_variable();
        let owned = SgpdBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; owned.encoded_len()];
        owned.encode_into(&mut encoded).unwrap();
        assert_eq!(encoded, original);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_sgpd_box_round_trip_v2() {
        use crate::BoxEncode;

        let original = raw_v2();
        let owned = SgpdBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; owned.encoded_len()];
        owned.encode_into(&mut encoded).unwrap();
        assert_eq!(encoded, original);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_sgpd_box_to_owned_v0() {
        let data: [u8; 14] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            b'r', b'o', b'l', b'l', // grouping_type = "roll"
            0x00, 0x00, 0x00, 0x02, // entry_count = 2
            0xFF, 0xFE, // raw entry bytes
        ];

        let view = SgpdBoxView::decode(&data).unwrap();
        let owned = SgpdBox::try_from(&view).unwrap();

        assert_eq!(owned.version, 0);
        assert_eq!(owned.entry_count, 2);
        // Version 0 stores all bytes as a single blob
        assert_eq!(owned.entries.len(), 1);
        assert_eq!(owned.entries[0], &[0xFF, 0xFE]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_sgpd_box_to_owned_v1() {
        let data = raw_v1_fixed();
        let view = SgpdBoxView::decode(&data).unwrap();
        let owned = SgpdBox::try_from(&view).unwrap();

        assert_eq!(owned.version, 1);
        assert_eq!(owned.entries.len(), 2);
        assert_eq!(owned.entries[0], &[0xFF, 0xFF]);
        assert_eq!(owned.entries[1], &[0x00, 0x02]);
    }
}
