use crate::cursor::ReadCursor;
use crate::types::FourCC;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;

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
    const ENTRY_SIZE: usize = 8;

    /// Returns an iterator over the entries in the Sample to Group Box.
    pub fn entries(&self) -> impl Iterator<Item = Result<SbgpEntry>> + 'a {
        let entry_bytes = self.entries;
        let entry_count = self.entry_count as usize;

        entry_bytes
            .chunks_exact(Self::ENTRY_SIZE)
            .take(entry_count)
            .map(|chunk| {
                let sample_count = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                let group_description_index =
                    u32::from_be_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);

                Ok(SbgpEntry {
                    sample_count,
                    group_description_index,
                })
            })
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

        let expected_size = entry_count as usize * Self::ENTRY_SIZE;
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
    extern crate alloc;
    use alloc::vec::Vec;

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

    impl TryFrom<&SbgpBoxView<'_>> for SbgpBox {
        type Error = Error;

        fn try_from(view: &SbgpBoxView<'_>) -> Result<Self> {
            let entries: Result<Vec<SbgpEntry>> = view.entries().collect();
            Ok(SbgpBox {
                version: view.version,
                flags: view.flags,
                grouping_type: view.grouping_type,
                grouping_type_parameter: view.grouping_type_parameter,
                entries: entries?,
            })
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
            SbgpBox::try_from(&view)
        }
    }

    impl BoxEncode for SbgpBox {
        fn encode(&self, bytes: &mut [u8]) -> Result<usize> {
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
        let parsed: Vec<_> = sbgp.entries().map(|r| r.unwrap()).collect();
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
        let owned = SbgpBox::try_from(&view).unwrap();

        assert_eq!(owned.entries.len(), 2);
        assert_eq!(owned.entries[0].sample_count, 10);
        assert_eq!(owned.entries[1].group_description_index, 2);
    }
}
