use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::FixedEntry;
use crate::types::FourCC;

define_box_flags!(
    /// Flags for the Sample To Group Box (`sbgp`).
    ///
    /// Reserved (should be 0).
    SbgpFlags {}
);

/// An entry in the Sample To Group Box (`sbgp`).
///
/// Contains a sample count and a group description index, indicating how
/// many consecutive samples belong to a particular group and which group they belong to.
/// The group description index refers to an entry in a corresponding Sample Group Description Box (`sgpd`).
#[derive(Debug, Clone, Copy)]
pub struct SbgpEntry {
    /// The number of consecutive samples that belong to the same group.
    pub sample_count: u32,
    /// The index of the group description in the Sample Group Description Box (`sgpd`).
    pub group_description_index: u32,
}

impl FixedEntry<8> for SbgpEntry {
    fn from_bytes(bytes: &[u8; 8]) -> Self {
        SbgpEntry {
            sample_count: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            group_description_index: u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        }
    }

    #[cfg(feature = "alloc")]
    fn to_bytes(&self) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        bytes[0..4].copy_from_slice(&self.sample_count.to_be_bytes());
        bytes[4..8].copy_from_slice(&self.group_description_index.to_be_bytes());
        bytes
    }
}

define_entry_iter!(
    /// An iterator over entries in the Sample To Group Box (`sbgp`).
    pub struct SbgpEntryIter(SbgpEntry, 8);
);

/// A reference to a Sample To Group Box (`sbgp`).
///
/// Defines how samples are grouped together by associating sample counts with group description indices.
/// This allows for efficient grouping of samples without needing to specify a group for each individual sample.
/// The group description index refers to entries in a corresponding Sample Group Description Box (`sgpd`), which
/// provides the actual group information (e.g., which samples belong to which groups and what properties those groups have).
///
/// # Structure
/// - `version`: Box version (0 or 1).
/// - `flags`: Reserved (should be 0).
/// - `grouping_type`: A 4-character code that identifies the type of grouping (e.g., "roll", "rap ", "tele", etc.).
/// - `grouping_type_parameter`: An optional 4-byte field that provides additional information about the grouping type (used in version 1).
/// - `entry_count`: Number of entries in the box, indicating how many sample groups are defined.
/// -  `entries`: A list of entries, each containing a sample count and a group description index, indicating how many consecutive samples belong to which group.
#[derive(Debug)]
pub struct SbgpBoxView<'a> {
    /// Box version (0 or 1).
    pub version: u8,
    /// Reserved flags (should be 0).
    pub flags: SbgpFlags,
    /// A 4-character code that identifies the type of grouping.
    pub grouping_type: FourCC,
    /// An optional parameter that provides additional information about the grouping type (used in version 1).
    pub grouping_type_parameter: Option<u32>,
    /// The number of entries in the box, indicating how many sample groups are defined.
    pub entry_count: u32,
    entries: &'a [u8],
}

impl<'a> SbgpBoxView<'a> {
    /// Returns an iterator over the SBGP entries.
    pub fn entries(&self) -> SbgpEntryIter<'a> {
        SbgpEntryIter::new(self.entries)
    }
}

impl BoxCodec for SbgpBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::SBGP
    }
}

impl<'de> BoxDecode<'de> for SbgpBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = crate::cursor::ReadCursor::new(bytes);

        // Read version (1 byte)
        let version = cur.read_u8()?;

        // Read flags (3 bytes)
        let flags = SbgpFlags::from_be_bytes(cur.read_array::<3>()?);

        // Read grouping type (4 bytes)
        let grouping_type = FourCC::from(cur.read_array::<4>()?);

        // Read optional grouping type parameter (4 bytes, only if version == 1)
        let grouping_type_parameter = if version == 1 {
            Some(cur.read_u32_be()?)
        } else {
            None
        };

        // Read entry count (4 bytes)
        let entry_count = cur.read_u32_be()?;

        let expected_size = entry_count as usize * SbgpEntry::ENTRY_SIZE; // Each entry is 8 bytes

        if cur.remaining() != expected_size {
            return Err(Error::at_in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Entries length does not match entry count",
                    got: cur.remaining() as u64,
                },
                cur.position() as u64,
                BoxType::SBGP,
            ));
        }

        let entries = cur.take(expected_size)?;

        Ok(SbgpBoxView {
            version,
            flags,
            grouping_type,
            grouping_type_parameter,
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

    /// An owned version of the Sample To Group Box (`sbgp`), containing owned data instead of references.
    ///
    /// This is the owned variant of [`SbgpBoxView`] that stores entries
    /// in a heap-allocated vector.
    ///
    /// # Structure
    /// - `version`: Box version (0 or 1).
    /// - `flags`: Reserved (should be 0).
    /// - `grouping_type`: A 4-character code that identifies the type of grouping (e.g., "roll", "rap ", "tele", etc.).
    /// - `grouping_type_parameter`: An optional 4-byte field that provides additional information about the grouping type (used in version 1).
    /// - `entries`: A list of entries, each containing a sample count and a group description index, indicating how many consecutive samples belong to which group.
    #[derive(Debug, Clone)]
    pub struct SbgpBox {
        /// Box version (0 or 1).
        pub version: u8,
        /// Reserved flags (should be 0).
        pub flags: SbgpFlags,
        /// A 4-character code that identifies the type of grouping.
        pub grouping_type: FourCC,
        /// An optional parameter that provides additional information about the grouping type (used in version 1).
        pub grouping_type_parameter: Option<u32>,
        /// A list of entries, each containing a sample count and a group description index, indicating how many consecutive samples belong to which group.
        pub entries: Vec<SbgpEntry>,
    }

    impl Default for SbgpBox {
        fn default() -> Self {
            SbgpBox {
                version: 0,
                flags: SbgpFlags::default(),
                grouping_type: FourCC::from([0; 4]),
                grouping_type_parameter: None,
                entries: Vec::new(),
            }
        }
    }

    impl From<&SbgpBoxView<'_>> for SbgpBox {
        fn from(view: &SbgpBoxView<'_>) -> Self {
            let entries = view.entries().collect();

            SbgpBox {
                version: view.version,
                flags: view.flags,
                grouping_type: view.grouping_type,
                grouping_type_parameter: view.grouping_type_parameter,
                entries,
            }
        }
    }

    impl SbgpBoxView<'_> {
        /// Converts this view into an owned `SbgpBox`.
        pub fn to_owned(&self) -> SbgpBox {
            SbgpBox::from(self)
        }
    }

    impl BoxCodec for SbgpBox {
        fn boxtype(&self) -> BoxType {
            BoxType::SBGP
        }
    }

    impl BoxDecode<'_> for SbgpBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = SbgpBoxView::decode(bytes)?;
            Ok(SbgpBox::from(&view))
        }
    }

    impl BoxEncode for SbgpBox {
        fn encoded_len(&self) -> usize {
            4 // version + flags
            + 4 // grouping_type
            + if self.version == 1 { 4 } else { 0 } // optional grouping_type_parameter
            + 4 // entry_count
            + self.entries.len() * SbgpEntry::ENTRY_SIZE // entries
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_be_bytes())?;

            cur.write_array(self.grouping_type.as_bytes())?;

            if self.version == 1 {
                if let Some(param) = self.grouping_type_parameter {
                    cur.write_u32_be(param)?;
                } else {
                    // If version is 1, grouping_type_parameter must be present
                    return Err(Error::in_box(
                        ErrorKind::InvalidBoxField {
                            field: "grouping_type_parameter",
                            reason: "Version 1 requires grouping_type_parameter",
                        },
                        BoxType::SBGP,
                    ));
                }
            }

            cur.write_u32_be(u32::try_from(self.entries.len())?)?;

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

    fn raw_v0() -> [u8; 28] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            b'r', b'o', b'l', b'l', // grouping_type = "roll"
            0x00, 0x00, 0x00, 0x02, // entry_count = 2
            // Entry 1: sample_count = 5, group_description_index = 1
            0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x01,
            // Entry 2: sample_count = 10, group_description_index = 2
            0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x02,
        ]
    }

    fn raw_v1() -> [u8; 24] {
        [
            0x01, // version = 1
            0x00, 0x00, 0x00, // flags = 0
            b't', b'e', b'l', b'e', // grouping_type = "tele"
            b'p', b'a', b'r', b'm', // grouping_type_parameter = "parm"
            0x00, 0x00, 0x00, 0x01, // entry_count = 1
            0x00, 0x00, 0x00, 0x0F, // sample_count = 15
            0x00, 0x00, 0x00, 0x03, // group_description_index = 3
        ]
    }

    #[test]
    fn test_sbgp_box_view_decode() {
        let data = raw_v0();
        let sbgp = SbgpBoxView::decode(&data).unwrap();

        assert_eq!(sbgp.version, 0);
        assert_eq!(sbgp.flags.bits(), 0);
        assert_eq!(sbgp.grouping_type, FourCC::from(*b"roll"));
        assert!(sbgp.grouping_type_parameter.is_none());
        assert_eq!(sbgp.entry_count, 2);

        let mut entries = sbgp.entries();
        let first = entries.next().unwrap();
        assert_eq!(first.sample_count, 5);
        assert_eq!(first.group_description_index, 1);
        let second = entries.next().unwrap();
        assert_eq!(second.sample_count, 10);
        assert_eq!(second.group_description_index, 2);
        assert!(entries.next().is_none());
    }

    #[test]
    fn test_sbgp_box_view_decode_v1() {
        let data = raw_v1();
        let sbgp = SbgpBoxView::decode(&data).unwrap();

        assert_eq!(sbgp.version, 1);
        assert_eq!(sbgp.grouping_type, FourCC::from(*b"tele"));
        assert_eq!(
            sbgp.grouping_type_parameter.unwrap(),
            u32::from_be_bytes(*b"parm")
        );
        assert_eq!(sbgp.entry_count, 1);

        let mut entries = sbgp.entries();
        let first = entries.next().unwrap();
        assert_eq!(first.sample_count, 15);
        assert_eq!(first.group_description_index, 3);
        assert!(entries.next().is_none());
    }

    #[test]
    fn test_sbgp_box_view_empty_entries() {
        let data: [u8; 12] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            b'r', b'o', b'l', b'l', // grouping_type = "roll"
            0x00, 0x00, 0x00, 0x00, // entry_count = 0
        ];

        let sbgp = SbgpBoxView::decode(&data).unwrap();
        assert_eq!(sbgp.entries().count(), 0);
    }

    #[test]
    fn test_sbgp_box_view_invalid_size() {
        // entry_count = 2 but only 1 entry provided
        let data: [u8; 20] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            b'r', b'o', b'l', b'l', // grouping_type = "roll"
            0x00, 0x00, 0x00, 0x02, // entry_count = 2
            0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x01,
        ];

        assert!(SbgpBoxView::decode(&data).is_err());
    }

    #[test]
    fn test_sbgp_box_view_truncated() {
        let data: [u8; 2] = [0x00, 0x00];
        assert!(SbgpBoxView::decode(&data).is_err());
    }

    #[test]
    fn test_sbgp_entry_round_trip() {
        let entry = SbgpEntry {
            sample_count: 12345,
            group_description_index: 67890,
        };

        let bytes = entry.to_bytes();
        let decoded = SbgpEntry::from_bytes(&bytes);

        assert_eq!(decoded.sample_count, entry.sample_count);
        assert_eq!(
            decoded.group_description_index,
            entry.group_description_index
        );
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_sbgp_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_v0();
        let owned = SbgpBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; owned.encoded_len()];
        owned.encode_into(&mut encoded).unwrap();
        assert_eq!(encoded, original);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_sbgp_box_round_trip_v1() {
        use crate::BoxEncode;

        let original = raw_v1();
        let owned = SbgpBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; owned.encoded_len()];
        owned.encode_into(&mut encoded).unwrap();
        assert_eq!(encoded, original);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_sbgp_box_to_owned() {
        let data = raw_v0();
        let view = SbgpBoxView::decode(&data).unwrap();
        let owned = view.to_owned();

        assert_eq!(owned.version, view.version);
        assert_eq!(owned.flags.bits(), view.flags.bits());
        assert_eq!(owned.grouping_type, view.grouping_type);
        assert_eq!(owned.grouping_type_parameter, view.grouping_type_parameter);
        assert_eq!(owned.entries.len() as u32, view.entry_count);

        for (owned_entry, view_entry) in owned.entries.iter().zip(view.entries()) {
            assert_eq!(owned_entry.sample_count, view_entry.sample_count);
            assert_eq!(
                owned_entry.group_description_index,
                view_entry.group_description_index
            );
        }
    }
}
