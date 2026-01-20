use crate::cursor::ReadCursor;
use crate::types::FourCC;

use crate::BoxFrame;
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

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<SbgpBoxView<'a>> {
        let version = cur
            .read_u8()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let flags = SbgpFlags::from_bytes(
            cur.read_array()
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
        );

        if version > 1 {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxVersion {
                    reason: "sbgp version must be 0 or 1",
                    got: version,
                },
                BoxType::SBGP,
            ));
        }

        let grouping_type_bytes = cur
            .read_array::<4>()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let grouping_type = FourCC::new(grouping_type_bytes);

        let grouping_type_parameter = if version == 1 {
            Some(
                cur.read_u32_be()
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
            )
        } else {
            None
        };

        let entry_count = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

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

    /// Parses a `SbgpBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<SbgpBoxView<'a>> {
        let mut cur = ReadCursor::new(payload);
        SbgpBoxView::parse_in(&mut cur)
    }
}

impl<'a> TryFrom<&'a [u8]> for SbgpBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        SbgpBoxView::parse(value)
    }
}

impl<'a> TryFrom<BoxFrame<'a>> for SbgpBoxView<'a> {
    type Error = Error;

    fn try_from(value: BoxFrame<'a>) -> Result<Self> {
        if value.boxtype() != BoxType::SBGP {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::SBGP,
                found: value.boxtype(),
            }));
        }

        SbgpBoxView::parse(value.payload())
    }
}

/// Specification for the Sample to Group Box (`sbgp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SbgpSpec;

/// Flags for the Sample to Group Box (`sbgp`).
pub type SbgpFlags = FullBoxFlags<SbgpSpec>;

#[cfg(feature = "alloc")]
pub use owned::SbgpBox;

#[cfg(feature = "alloc")]
mod owned {
    extern crate alloc;
    use alloc::vec::Vec;

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

    impl SbgpBox {
        /// Creates a `SbgpBox` from a `SbgpBoxView`.
        pub fn from_view(view: &SbgpBoxView<'_>) -> Result<SbgpBox> {
            let entries: Result<Vec<SbgpEntry>> = view.entries().collect();
            Ok(SbgpBox {
                version: view.version,
                flags: view.flags,
                grouping_type: view.grouping_type,
                grouping_type_parameter: view.grouping_type_parameter,
                entries: entries?,
            })
        }

        /// Parses a `SbgpBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<SbgpBox> {
            let view = SbgpBoxView::parse(payload)?;
            SbgpBox::from_view(&view)
        }

        /// Returns the size of the payload in bytes.
        pub fn size(&self) -> usize {
            // version(1) + flags(3) + grouping_type(4) + [grouping_type_parameter(4)] + entry_count(4) + entries(8 * n)
            let param_size = if self.version == 1 { 4 } else { 0 };
            1 + 3 + 4 + param_size + 4 + self.entries.len() * 8
        }

        pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            cur.write_u8(self.version)
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
            cur.write_array(&self.flags.to_bytes())
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
            cur.write_array(self.grouping_type.as_bytes())
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

            if self.version == 1 {
                cur.write_u32_be(self.grouping_type_parameter.unwrap_or(0))
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
            }

            cur.write_u32_be(self.entries.len() as u32)
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

            for entry in &self.entries {
                cur.write_u32_be(entry.sample_count)
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
                cur.write_u32_be(entry.group_description_index)
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
            }

            if !cur.is_empty() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxSize {
                        reason: "Buffer larger than expected",
                        got: cur.remaining() as u64,
                    },
                    BoxType::SBGP,
                ));
            }

            Ok(())
        }

        /// Writes this `SbgpBox` into the given payload.
        pub fn write(&self, payload: &mut [u8]) -> Result<()> {
            let mut cursor = WriteCursor::new(payload);
            self.write_in(&mut cursor)
        }
    }

    impl TryFrom<&SbgpBoxView<'_>> for SbgpBox {
        type Error = Error;

        fn try_from(value: &SbgpBoxView<'_>) -> Result<Self> {
            SbgpBox::from_view(value)
        }
    }

    impl TryFrom<BoxFrame<'_>> for SbgpBox {
        type Error = Error;

        fn try_from(value: BoxFrame<'_>) -> Result<Self> {
            let view = SbgpBoxView::try_from(value)?;
            SbgpBox::from_view(&view)
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
        let sbgp = SbgpBoxView::parse(&payload).unwrap();
        assert_eq!(sbgp.version, 0);
        assert!(sbgp.grouping_type_parameter.is_none());
        assert_eq!(sbgp.entries().count(), 0);

        // With entries v0
        let payload = make_sbgp_payload_v0(b"seig", &[(10, 1), (20, 2), (30, 0)]);
        let sbgp = SbgpBoxView::parse(&payload).unwrap();
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
        let sbgp = SbgpBoxView::parse(&payload).unwrap();
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
        assert!(SbgpBoxView::parse(&data).is_err());
    }

    #[test]
    fn wrong_box_type() {
        let payload = make_sbgp_payload_v0(b"roll", &[]);
        let mut box_data = Vec::new();
        box_data.extend_from_slice(&(8 + payload.len() as u32).to_be_bytes());
        box_data.extend_from_slice(b"sgpd");
        box_data.extend_from_slice(&payload);

        let mut cursor = ReadCursor::new(&box_data);
        let box_view = BoxFrame::parse_in(&mut cursor).unwrap();
        let result = SbgpBoxView::try_from(box_view);
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn owned_conversion() {
        let payload = make_sbgp_payload_v0(b"roll", &[(10, 1), (20, 2)]);
        let view = SbgpBoxView::parse(&payload).unwrap();
        let owned = SbgpBox::from_view(&view).unwrap();

        assert_eq!(owned.entries.len(), 2);
        assert_eq!(owned.entries[0].sample_count, 10);
        assert_eq!(owned.entries[1].group_description_index, 2);
    }
}
