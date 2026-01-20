use crate::cursor::ReadCursor;

use crate::BoxType;
use crate::BoxFrame;
use crate::error::*;
use crate::header::FullBoxFlags;

/// An entry in a Track Fragment Random Access Box (`tfra`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TfraEntry {
    /// The time of the random access point, in the timescale of the track.
    pub time: u64,
    /// The offset from the start of the file to the moof box.
    pub moof_offset: u64,
    /// The track fragment number.
    pub traf_number: u32,
    /// The track run number within the track fragment.
    pub trun_number: u32,
    /// The sample number within the track run.
    pub sample_number: u32,
}

/// An iterator over entries in a Track Fragment Random Access Box (`tfra`).
#[derive(Debug)]
pub struct TfraEntryIter<'a> {
    entries: &'a [u8],
    version: u8,
    length_size_of_traf_num: u8,
    length_size_of_trun_num: u8,
    length_size_of_sample_num: u8,
}

impl<'a> TfraEntryIter<'a> {
    fn entry_size(&self) -> usize {
        let time_size = if self.version == 1 { 8 } else { 4 };
        let moof_offset_size = if self.version == 1 { 8 } else { 4 };
        let traf_num_size = (self.length_size_of_traf_num + 1) as usize;
        let trun_num_size = (self.length_size_of_trun_num + 1) as usize;
        let sample_num_size = (self.length_size_of_sample_num + 1) as usize;

        time_size + moof_offset_size + traf_num_size + trun_num_size + sample_num_size
    }

    fn read_variable_uint(bytes: &[u8], size: u8) -> u32 {
        match size {
            1 => bytes[0] as u32,
            2 => u16::from_be_bytes([bytes[0], bytes[1]]) as u32,
            3 => {
                let b1 = bytes[0] as u32;
                let b2 = u16::from_be_bytes([bytes[1], bytes[2]]) as u32;
                (b1 << 16) | b2
            }
            4 => u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            _ => unreachable!("Invalid length size validated during parsing"),
        }
    }
}

impl<'a> Iterator for TfraEntryIter<'a> {
    type Item = Result<TfraEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.entries.is_empty() {
            return None;
        }

        let entry_size = self.entry_size();
        let chunk = &self.entries[..entry_size];
        self.entries = &self.entries[entry_size..];

        // Helper function to parse one entry
        let parse_entry = || -> Result<TfraEntry> {
            let mut offset = 0;

            // Read time field
            let time = if self.version == 1 {
                let value = u64::from_be_bytes([
                    chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
                ]);
                offset += 8;
                value
            } else {
                let value = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as u64;
                offset += 4;
                value
            };

            // Read moof_offset field
            let moof_offset = if self.version == 1 {
                let value = u64::from_be_bytes([
                    chunk[offset],
                    chunk[offset + 1],
                    chunk[offset + 2],
                    chunk[offset + 3],
                    chunk[offset + 4],
                    chunk[offset + 5],
                    chunk[offset + 6],
                    chunk[offset + 7],
                ]);
                offset += 8;
                value
            } else {
                let value = u32::from_be_bytes([
                    chunk[offset],
                    chunk[offset + 1],
                    chunk[offset + 2],
                    chunk[offset + 3],
                ]) as u64;
                offset += 4;
                value
            };

            // Read variable-sized fields
            let traf_size = (self.length_size_of_traf_num + 1) as usize;
            let traf_number =
                Self::read_variable_uint(&chunk[offset..], self.length_size_of_traf_num + 1);
            offset += traf_size;

            let trun_size = (self.length_size_of_trun_num + 1) as usize;
            let trun_number =
                Self::read_variable_uint(&chunk[offset..], self.length_size_of_trun_num + 1);
            offset += trun_size;

            let sample_number =
                Self::read_variable_uint(&chunk[offset..], self.length_size_of_sample_num + 1);

            Ok(TfraEntry {
                time,
                moof_offset,
                traf_number,
                trun_number,
                sample_number,
            })
        };

        Some(parse_entry())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let entry_size = self.entry_size();
        if entry_size == 0 {
            (0, Some(0))
        } else {
            let count = self.entries.len() / entry_size;
            (count, Some(count))
        }
    }
}

impl<'a> ExactSizeIterator for TfraEntryIter<'a> {}

/// A reference to a Track Fragment Random Access Box (`tfra`).
#[derive(Debug)]
pub struct TfraBoxView<'a> {
    /// The version of the box (0 or 1).
    pub version: u8,
    /// The flags of the box.
    pub flags: TfraFlags,
    /// The track ID.
    pub track_id: u32,
    /// The length in bytes of the traf_number field minus one.
    pub length_size_of_traf_num: u8,
    /// The length in bytes of the trun_number field minus one.
    pub length_size_of_trun_num: u8,
    /// The length in bytes of the sample_number field minus one.
    pub length_size_of_sample_num: u8,
    /// The number of entries.
    pub number_of_entry: u32,
    entries: &'a [u8],
}

impl<'a> TfraBoxView<'a> {
    /// Returns an iterator over the entries in the Track Fragment Random Access Box.
    pub fn entries(&self) -> TfraEntryIter<'a> {
        TfraEntryIter {
            entries: self.entries,
            version: self.version,
            length_size_of_traf_num: self.length_size_of_traf_num,
            length_size_of_trun_num: self.length_size_of_trun_num,
            length_size_of_sample_num: self.length_size_of_sample_num,
        }
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<TfraBoxView<'a>> {
        let version = cur
            .read_u8()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let flags = TfraFlags::from_bytes(
            cur.read_array()
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
        );

        if version > 1 {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxVersion {
                    reason: "tfra version must be 0 or 1",
                    got: version,
                },
                BoxType::TFRA,
            ));
        }

        let track_id = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        // Read reserved (26 bits) and length_size fields (2 bits each)
        let reserved_and_length = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        let length_size_of_traf_num = ((reserved_and_length >> 4) & 0x03) as u8;
        let length_size_of_trun_num = ((reserved_and_length >> 2) & 0x03) as u8;
        let length_size_of_sample_num = (reserved_and_length & 0x03) as u8;

        let number_of_entry = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        // Calculate expected entry size
        let time_size = if version == 1 { 8 } else { 4 };
        let moof_offset_size = if version == 1 { 8 } else { 4 };
        let traf_num_size = (length_size_of_traf_num + 1) as usize;
        let trun_num_size = (length_size_of_trun_num + 1) as usize;
        let sample_num_size = (length_size_of_sample_num + 1) as usize;

        let entry_size =
            time_size + moof_offset_size + traf_num_size + trun_num_size + sample_num_size;
        let expected_size = number_of_entry as usize * entry_size;

        if cur.remaining() != expected_size {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "tfra entries size does not match number_of_entry",
                    got: cur.remaining() as u64,
                },
                BoxType::TFRA,
            ));
        }

        let entries = cur.take(cur.remaining())?;

        Ok(TfraBoxView {
            version,
            flags,
            track_id,
            length_size_of_traf_num,
            length_size_of_trun_num,
            length_size_of_sample_num,
            number_of_entry,
            entries,
        })
    }

    /// Parses a `TfraBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<TfraBoxView<'a>> {
        let mut cur = ReadCursor::new(payload);
        TfraBoxView::parse_in(&mut cur)
    }
}

impl<'a> TryFrom<&'a [u8]> for TfraBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        TfraBoxView::parse(value)
    }
}

impl<'a> TryFrom<BoxFrame<'a>> for TfraBoxView<'a> {
    type Error = Error;

    fn try_from(value: BoxFrame<'a>) -> Result<Self> {
        if value.boxtype() != BoxType::TFRA {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::TFRA,
                found: value.boxtype(),
            }));
        }

        TfraBoxView::parse(value.payload())
    }
}

/// Specification for the Track Fragment Random Access Box (`tfra`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TfraSpec;

/// Flags for the Track Fragment Random Access Box (`tfra`).
pub type TfraFlags = FullBoxFlags<TfraSpec>;

#[cfg(feature = "alloc")]
pub use owned::TfraBox;

#[cfg(feature = "alloc")]
mod owned {
    extern crate alloc;
    use alloc::vec::Vec;

    use super::*;

    /// An owned Track Fragment Random Access Box (`tfra`).
    #[derive(Debug, Clone)]
    pub struct TfraBox {
        /// The version of the box (0 or 1).
        pub version: u8,
        /// The flags of the box.
        pub flags: TfraFlags,
        /// The track ID.
        pub track_id: u32,
        /// The length in bytes of the traf_number field minus one.
        pub length_size_of_traf_num: u8,
        /// The length in bytes of the trun_number field minus one.
        pub length_size_of_trun_num: u8,
        /// The length in bytes of the sample_number field minus one.
        pub length_size_of_sample_num: u8,
        /// The number of entries.
        pub number_of_entry: u32,
        /// The raw entry data bytes.
        entries: Vec<u8>,
    }

    impl TfraBox {
        /// Creates a `TfraBox` from a `TfraBoxView`.
        pub fn from_view(view: &TfraBoxView<'_>) -> Result<TfraBox> {
            Ok(TfraBox {
                version: view.version,
                flags: view.flags,
                track_id: view.track_id,
                length_size_of_traf_num: view.length_size_of_traf_num,
                length_size_of_trun_num: view.length_size_of_trun_num,
                length_size_of_sample_num: view.length_size_of_sample_num,
                number_of_entry: view.number_of_entry,
                entries: view.entries.to_vec(),
            })
        }

        /// Parses a `TfraBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<TfraBox> {
            let view = TfraBoxView::parse(payload)?;
            TfraBox::from_view(&view)
        }

        /// Returns an iterator over the entries in the Track Fragment Random Access Box.
        pub fn entries(&self) -> TfraEntryIter<'_> {
            TfraEntryIter {
                entries: &self.entries,
                version: self.version,
                length_size_of_traf_num: self.length_size_of_traf_num,
                length_size_of_trun_num: self.length_size_of_trun_num,
                length_size_of_sample_num: self.length_size_of_sample_num,
            }
        }
    }

    impl TryFrom<&TfraBoxView<'_>> for TfraBox {
        type Error = Error;

        fn try_from(value: &TfraBoxView<'_>) -> Result<Self> {
            TfraBox::from_view(value)
        }
    }

    impl TryFrom<BoxFrame<'_>> for TfraBox {
        type Error = Error;

        fn try_from(value: BoxFrame<'_>) -> Result<Self> {
            let view = TfraBoxView::try_from(value)?;
            TfraBox::from_view(&view)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_full_box_header(version: u8, flags: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(version);
        data.extend_from_slice(&flags.to_be_bytes()[1..4]);
        data
    }

    fn make_tfra_payload_v0(
        track_id: u32,
        length_sizes: (u8, u8, u8),
        entries: &[(u32, u32, u32, u32, u32)],
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(0, 0));
        payload.extend_from_slice(&track_id.to_be_bytes());

        // Reserved (26 bits) and length_size fields (2 bits each)
        let reserved_and_length = ((length_sizes.0 as u32) << 4)
            | ((length_sizes.1 as u32) << 2)
            | (length_sizes.2 as u32);
        payload.extend_from_slice(&reserved_and_length.to_be_bytes());

        payload.extend_from_slice(&(entries.len() as u32).to_be_bytes());

        for &(time, moof_offset, traf, trun, sample) in entries {
            payload.extend_from_slice(&time.to_be_bytes());
            payload.extend_from_slice(&moof_offset.to_be_bytes());

            // Write variable-sized fields
            let write_var = |payload: &mut Vec<u8>, value: u32, size: u8| match size + 1 {
                1 => payload.push(value as u8),
                2 => payload.extend_from_slice(&(value as u16).to_be_bytes()),
                3 => {
                    payload.push((value >> 16) as u8);
                    payload.extend_from_slice(&((value & 0xFFFF) as u16).to_be_bytes());
                }
                4 => payload.extend_from_slice(&value.to_be_bytes()),
                _ => {}
            };

            write_var(&mut payload, traf, length_sizes.0);
            write_var(&mut payload, trun, length_sizes.1);
            write_var(&mut payload, sample, length_sizes.2);
        }

        payload
    }

    #[test]
    fn parse_tfra_v0_empty() {
        let payload = make_tfra_payload_v0(1, (0, 0, 0), &[]);
        let tfra = TfraBoxView::parse(&payload).unwrap();

        assert_eq!(tfra.version, 0);
        assert_eq!(tfra.track_id, 1);
        assert_eq!(tfra.length_size_of_traf_num, 0);
        assert_eq!(tfra.length_size_of_trun_num, 0);
        assert_eq!(tfra.length_size_of_sample_num, 0);
        assert_eq!(tfra.number_of_entry, 0);
        assert_eq!(tfra.entries().count(), 0);
    }

    #[test]
    fn parse_tfra_v0_with_entries() {
        let entries = vec![(1000, 5000, 1, 1, 1), (2000, 10000, 2, 1, 1)];
        let payload = make_tfra_payload_v0(1, (0, 0, 0), &entries);
        let tfra = TfraBoxView::parse(&payload).unwrap();

        assert_eq!(tfra.number_of_entry, 2);

        let parsed: Vec<_> = tfra.entries().collect();
        assert_eq!(parsed.len(), 2);

        assert_eq!(parsed[0].as_ref().unwrap().time, 1000);
        assert_eq!(parsed[0].as_ref().unwrap().moof_offset, 5000);
        assert_eq!(parsed[0].as_ref().unwrap().traf_number, 1);

        assert_eq!(parsed[1].as_ref().unwrap().time, 2000);
        assert_eq!(parsed[1].as_ref().unwrap().moof_offset, 10000);
    }

    #[test]
    fn parse_tfra_v0_with_larger_fields() {
        let entries = vec![(1000, 5000, 256, 512, 1024)];
        // Use size 1 (2 bytes) for traf, size 1 (2 bytes) for trun, size 1 (2 bytes) for sample
        let payload = make_tfra_payload_v0(1, (1, 1, 1), &entries);
        let tfra = TfraBoxView::parse(&payload).unwrap();

        let parsed: Vec<_> = tfra.entries().collect();
        assert_eq!(parsed[0].as_ref().unwrap().traf_number, 256);
        assert_eq!(parsed[0].as_ref().unwrap().trun_number, 512);
        assert_eq!(parsed[0].as_ref().unwrap().sample_number, 1024);
    }

    #[test]
    fn parse_tfra_invalid_version() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(2, 0));
        payload.extend_from_slice(&1u32.to_be_bytes()); // track_id

        let result = TfraBoxView::parse(&payload);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::InvalidBoxVersion { .. }));
        }
    }

    #[cfg(feature = "alloc")]
    mod alloc_tests {
        use super::*;

        #[test]
        fn tfra_box_from_view() {
            let entries = vec![(1000, 5000, 1, 1, 1)];
            let payload = make_tfra_payload_v0(1, (0, 0, 0), &entries);
            let view = TfraBoxView::parse(&payload).unwrap();
            let owned = TfraBox::from_view(&view).unwrap();

            assert_eq!(owned.track_id, 1);
            assert_eq!(owned.number_of_entry, 1);

            let parsed: Vec<_> = owned.entries().collect();
            assert_eq!(parsed[0].as_ref().unwrap().time, 1000);
        }
    }
}
