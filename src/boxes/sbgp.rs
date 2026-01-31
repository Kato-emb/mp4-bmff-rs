use crate::cursor::ReadCursor;
use crate::types::FourCC;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::FixedSizeEntry;
use crate::iter::FixedSizeEntryIter;

use super::FullBoxFlags;

/// An entry in the Sample to Group Box (`sbgp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SbgpEntry {
    /// The number of consecutive samples with the same group description.
    pub sample_count: u32,
    /// An index that identifies a sample group description entry.
    /// The value 0 means that the samples are not assigned to any group.
    pub group_description_index: u32,
}

impl FixedSizeEntry for SbgpEntry {
    const ENTRY_SIZE: usize = 8;

    fn from_bytes(bytes: &[u8]) -> Self {
        debug_assert_eq!(bytes.len(), Self::ENTRY_SIZE);

        SbgpEntry {
            sample_count: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            group_description_index: u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        }
    }

    fn to_bytes(&self, bytes: &mut [u8]) {
        debug_assert_eq!(bytes.len(), Self::ENTRY_SIZE);
        bytes[0..4].copy_from_slice(&self.sample_count.to_be_bytes());
        bytes[4..8].copy_from_slice(&self.group_description_index.to_be_bytes());
    }
}

/// A reference to a Sample to Group Box (`sbgp`).
#[derive(Debug)]
pub struct SbgpBoxView<'a> {
    /// The version of the box (0 or 1).
    pub version: u8,
    /// The flags of the box.
    pub flags: SbgpFlags,
    /// A FourCC identifying the type of grouping.
    pub grouping_type: FourCC,
    /// An additional parameter for the grouping (version 1 only).
    pub grouping_type_parameter: Option<u32>,
    /// The number of entries.
    pub entry_count: u32,
    entries: &'a [u8],
}

impl<'a> SbgpBoxView<'a> {
    /// Returns an iterator over the entries in the Sample to Group Box.
    pub fn entries(&self) -> FixedSizeEntryIter<'a, SbgpEntry> {
        FixedSizeEntryIter::new(self.entries)
    }
}

/// Specification for the Sample to Group Box (`sbgp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SbgpSpec;

/// Flags for the Sample to Group Box (`sbgp`).
pub type SbgpFlags = FullBoxFlags<SbgpSpec>;

impl BoxCodec for SbgpBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::SBGP
    }
}

impl<'de> BoxDecode<'de> for SbgpBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = SbgpFlags::from_bytes(cur.read_array()?);

        if version > 1 {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxVersion {
                    reason: "sbgp version must be 0 or 1",
                    got: version,
                },
                BoxType::SBGP,
            ));
        }

        let grouping_type_bytes = cur.read_array::<4>()?;
        let grouping_type = FourCC::new(grouping_type_bytes);

        let grouping_type_parameter = if version == 1 {
            Some(cur.read_u32_be()?)
        } else {
            None
        };

        let entry_count = cur.read_u32_be()?;

        let expected_size = entry_count as usize * SbgpEntry::ENTRY_SIZE;
        if cur.remaining() != expected_size {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "sbgp entries size does not match entry_count",
                    got: cur.remaining() as u64,
                },
                BoxType::SBGP,
            ));
        }

        let entries = cur.take(cur.remaining())?;

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

impl<'a> TryFrom<&'a [u8]> for SbgpBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        SbgpBoxView::decode(value)
    }
}

#[cfg(feature = "alloc")]
pub use owned::SbgpBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use crate::BoxEncode;
    use crate::cursor::WriteCursor;

    use super::*;

    /// An owned Sample to Group Box (`sbgp`).
    #[derive(Debug, Clone)]
    pub struct SbgpBox {
        /// The version of the box (0 or 1).
        pub version: u8,
        /// The flags of the box.
        pub flags: SbgpFlags,
        /// A FourCC identifying the type of grouping.
        pub grouping_type: FourCC,
        /// An additional parameter for the grouping (version 1 only).
        pub grouping_type_parameter: Option<u32>,
        /// The entries.
        pub entries: Vec<SbgpEntry>,
    }

    impl From<&SbgpBoxView<'_>> for SbgpBox {
        fn from(value: &SbgpBoxView<'_>) -> Self {
            let entries: Vec<SbgpEntry> = value.entries().collect();
            SbgpBox {
                version: value.version,
                flags: value.flags,
                grouping_type: value.grouping_type,
                grouping_type_parameter: value.grouping_type_parameter,
                entries,
            }
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
        #[inline]
        fn encoded_len(&self) -> usize {
            1 // version
                + 3 // flags
                + 4 // grouping_type
                + if self.version == 1 { 4 } else { 0 } // grouping_type_parameter
                + 4 // entry_count
                + self.entries.len() * SbgpEntry::ENTRY_SIZE // entries
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_bytes())?;
            cur.write_array(self.grouping_type.as_bytes())?;

            if self.version == 1 {
                cur.write_u32_be(self.grouping_type_parameter.unwrap_or(0))?;
            }

            cur.write_u32_be(self.entries.len() as u32)?;

            for entry in &self.entries {
                cur.write_u32_be(entry.sample_count)?;
                cur.write_u32_be(entry.group_description_index)?;
            }

            Ok(cur.position())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sbgp_payload_v0(grouping_type: &[u8; 4], entries: &[(u32, u32)]) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(0);
        data.extend_from_slice(&[0, 0, 0]);
        data.extend_from_slice(grouping_type);
        data.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        for (sample_count, group_description_index) in entries {
            data.extend_from_slice(&sample_count.to_be_bytes());
            data.extend_from_slice(&group_description_index.to_be_bytes());
        }
        data
    }

    #[test]
    fn parse_and_iterate_entries() {
        // Empty v0
        let payload = make_sbgp_payload_v0(b"roll", &[]);
        let sbgp = SbgpBoxView::decode(&payload).unwrap();
        assert_eq!(sbgp.version, 0);
        assert!(sbgp.grouping_type_parameter.is_none());
        assert_eq!(sbgp.entries().count(), 0);

        // With entries v0
        let payload = make_sbgp_payload_v0(b"seig", &[(10, 1), (20, 2), (30, 0)]);
        let sbgp = SbgpBoxView::decode(&payload).unwrap();
        assert_eq!(sbgp.entry_count, 3);
        let parsed: Vec<_> = sbgp.entries().collect();
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].sample_count, 10);
        assert_eq!(parsed[0].group_description_index, 1);

        // Version 1 with parameter
        let mut payload = Vec::new();
        payload.push(1);
        payload.extend_from_slice(&[0, 0, 0]);
        payload.extend_from_slice(b"roll");
        payload.extend_from_slice(&0x12345678u32.to_be_bytes());
        payload.extend_from_slice(&1u32.to_be_bytes());
        payload.extend_from_slice(&5u32.to_be_bytes());
        payload.extend_from_slice(&1u32.to_be_bytes());
        let sbgp = SbgpBoxView::decode(&payload).unwrap();
        assert_eq!(sbgp.version, 1);
        assert_eq!(sbgp.grouping_type_parameter, Some(0x12345678));
    }

    #[test]
    fn invalid_version() {
        let mut data = Vec::new();
        data.push(2);
        data.extend_from_slice(&[0, 0, 0]);
        data.extend_from_slice(b"roll");
        data.extend_from_slice(&0u32.to_be_bytes());
        assert!(SbgpBoxView::decode(&data).is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn owned_conversion() {
        let payload = make_sbgp_payload_v0(b"roll", &[(10, 1), (20, 2)]);
        let view = SbgpBoxView::decode(&payload).unwrap();
        let owned = SbgpBox::from(&view);

        assert_eq!(owned.entries.len(), 2);
        assert_eq!(owned.entries[0].sample_count, 10);
        assert_eq!(owned.entries[1].group_description_index, 2);
    }
}
