//! Track Fragment Random Access Box (`tfra`) implementation.
//!
//! The Track Fragment Random Access Box provides a table mapping presentation
//! times to byte offsets of movie fragments for a single track. This enables
//! efficient seeking to specific times in fragmented MP4 files.
//!
//! This box is optional within the Movie Fragment Random Access Box (`mfra`).

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;

use crate::cursor::ReadCursor;

define_box_flags!(
    /// Flags for the Track Fragment Random Access Box (`tfra`).
    ///
    /// Reserved (should be 0).
    TfraFlags {}
);

/// An entry in the Track Fragment Random Access Box (`tfra`).
///
/// Maps a presentation time to a specific sample in a movie fragment,
/// identifying the `moof` offset and indices within the fragment.
#[derive(Debug, Clone, Copy)]
pub struct TfraEntry {
    /// Presentation time of this random access point (track timescale).
    pub time: u64,
    /// Byte offset of the `moof` box containing this access point.
    pub moof_offset: u64,
    /// 1-based index of the `traf` box within the `moof`.
    pub traf_number: u32,
    /// 1-based index of the `trun` box within the `traf`.
    pub trun_number: u32,
    /// 1-based index of the sample within the `trun`.
    pub sample_number: u32,
}

/// An iterator over entries in a Track Fragment Random Access Box (`tfra`).
#[derive(Debug)]
pub struct TfraEntryIter<'a> {
    entries: &'a [u8],
    version: u8,
    number_of_entry: u32,
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

    fn read_variable_uint(cur: &mut ReadCursor<'_>, size: u8) -> Result<u32> {
        let value = match size {
            1 => cur.read_u8().map(|v| v as u32)?,
            2 => cur.read_u16_be().map(|v| v as u32)?,
            3 => {
                let bytes = cur.read_array::<3>()?;
                u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]])
            }
            4 => cur.read_u32_be()?,
            _ => unreachable!("Invalid length size validated during parsing"),
        };

        Ok(value)
    }
}

impl<'a> Iterator for TfraEntryIter<'a> {
    type Item = Result<TfraEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.number_of_entry == 0 {
            return None;
        }

        let mut cur = ReadCursor::new(self.entries);

        let time: u64;
        let moof_offset: u64;
        match self.version {
            0 => {
                match cur.read_u32_be() {
                    Ok(v) => time = v as u64,
                    Err(e) => return Some(Err(e.into())),
                }
                match cur.read_u32_be() {
                    Ok(v) => moof_offset = v as u64,
                    Err(e) => return Some(Err(e.into())),
                }
            }
            1 => {
                match cur.read_u64_be() {
                    Ok(v) => time = v,
                    Err(e) => return Some(Err(e.into())),
                }
                match cur.read_u64_be() {
                    Ok(v) => moof_offset = v,
                    Err(e) => return Some(Err(e.into())),
                }
            }
            _ => {
                return Some(Err(Error::in_box(
                    ErrorKind::InvalidBoxVersion {
                        reason: "0 or 1 in this specification",
                        got: self.version,
                    },
                    BoxType::TFRA,
                )));
            }
        }

        let traf_number_size = self.length_size_of_traf_num + 1;
        let traf_number = match Self::read_variable_uint(&mut cur, traf_number_size) {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };

        let trun_number_size = self.length_size_of_trun_num + 1;
        let trun_number = match Self::read_variable_uint(&mut cur, trun_number_size) {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };

        let sample_number_size = self.length_size_of_sample_num + 1;
        let sample_number = match Self::read_variable_uint(&mut cur, sample_number_size) {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };

        let total_entry_size = self.entry_size();
        self.entries = &self.entries[total_entry_size..];
        self.number_of_entry -= 1;

        Some(Ok(TfraEntry {
            time,
            moof_offset,
            traf_number,
            trun_number,
            sample_number,
        }))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (
            self.number_of_entry as usize,
            Some(self.number_of_entry as usize),
        )
    }
}

impl<'a> ExactSizeIterator for TfraEntryIter<'a> {}

/// A reference to a Track Fragment Random Access Box (`tfra`).
///
/// Provides a seek table for one track in a fragmented movie. Each entry
/// maps a time to a specific sample location within the fragments.
///
/// # Structure
///
/// - `version`: Box version (0 for 32-bit times, 1 for 64-bit).
/// - `flags`: Reserved (should be 0).
/// - `track_id`: Track this table applies to.
/// - `length_size_of_*`: Byte sizes for variable-length fields (minus 1).
/// - `number_of_entry`: Number of random access points.
/// - `entries`: Array of time/offset/index tuples.
#[derive(Debug)]
pub struct TfraBoxView<'a> {
    /// Box version (0 for 32-bit times/offsets, 1 for 64-bit).
    pub version: u8,
    /// Reserved flags (should be 0).
    pub flags: TfraFlags,
    /// Track ID this table applies to.
    pub track_id: u32,
    /// Byte size of traf_number minus 1 (0-3 means 1-4 bytes).
    pub length_size_of_traf_num: u8,
    /// Byte size of trun_number minus 1 (0-3 means 1-4 bytes).
    pub length_size_of_trun_num: u8,
    /// Byte size of sample_number minus 1 (0-3 means 1-4 bytes).
    pub length_size_of_sample_num: u8,
    /// Number of random access point entries.
    pub number_of_entry: u32,
    entries: &'a [u8],
}

impl<'a> TfraBoxView<'a> {
    /// Returns an iterator over the entries in the Track Fragment Random Access Box (`tfra`).
    pub fn entries(&self) -> TfraEntryIter<'a> {
        TfraEntryIter {
            entries: self.entries,
            version: self.version,
            number_of_entry: self.number_of_entry,
            length_size_of_traf_num: self.length_size_of_traf_num,
            length_size_of_trun_num: self.length_size_of_trun_num,
            length_size_of_sample_num: self.length_size_of_sample_num,
        }
    }
}

impl BoxCodec for TfraBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::TFRA
    }
}

impl<'de> BoxDecode<'de> for TfraBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        // Read version (1 byte)
        let version = cur.read_u8()?;

        // Read flags (3 bytes)
        let flags = TfraFlags::from_be_bytes(cur.read_array::<3>()?);

        // Read track_id (4 bytes)
        let track_id = cur.read_u32_be()?;

        // Read reserved (26 bits) and length_size fields (2 bits each)
        let reserved_and_length = cur.read_u32_be()?;

        let length_size_of_traf_num = ((reserved_and_length >> 4) & 0x03) as u8;
        let length_size_of_trun_num = ((reserved_and_length >> 2) & 0x03) as u8;
        let length_size_of_sample_num = (reserved_and_length & 0x03) as u8;

        // Read number_of_entry (4 bytes)
        let number_of_entry = cur.read_u32_be()?;

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

        let entries = cur.take(expected_size)?;

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
}

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use super::*;
    use crate::BoxEncode;

    use crate::cursor::WriteCursor;

    /// An owned Track Fragment Random Access Box (`tfra`).
    ///
    /// This is the owned variant of [`TfraBoxView`] that stores entries
    /// in a heap-allocated vector.
    ///
    /// # Structure
    ///
    /// - `version`: Box version (0 for 32-bit, 1 for 64-bit times).
    /// - `track_id`: Track this table applies to.
    /// - `length_size_of_*`: Byte sizes for variable-length fields.
    /// - `entries`: Random access point table.
    #[derive(Debug, Clone)]
    pub struct TfraBox {
        /// Box version (0 for 32-bit times/offsets, 1 for 64-bit).
        pub version: u8,
        /// Reserved flags (should be 0).
        pub flags: TfraFlags,
        /// Track ID this table applies to.
        pub track_id: u32,
        /// Byte size of traf_number minus 1.
        pub length_size_of_traf_num: u8,
        /// Byte size of trun_number minus 1.
        pub length_size_of_trun_num: u8,
        /// Byte size of sample_number minus 1.
        pub length_size_of_sample_num: u8,
        /// Random access point entries.
        pub entries: Vec<TfraEntry>,
    }

    impl TryFrom<&TfraBoxView<'_>> for TfraBox {
        type Error = Error;

        fn try_from(view: &TfraBoxView<'_>) -> Result<Self> {
            let entries = view.entries().collect::<Result<Vec<TfraEntry>>>()?;
            Ok(TfraBox {
                version: view.version,
                flags: view.flags,
                track_id: view.track_id,
                length_size_of_traf_num: view.length_size_of_traf_num,
                length_size_of_trun_num: view.length_size_of_trun_num,
                length_size_of_sample_num: view.length_size_of_sample_num,
                entries,
            })
        }
    }

    impl BoxCodec for TfraBox {
        fn boxtype(&self) -> BoxType {
            BoxType::TFRA
        }
    }

    impl BoxDecode<'_> for TfraBox {
        fn decode(bytes: &'_ [u8]) -> Result<Self> {
            let view = TfraBoxView::decode(bytes)?;
            TfraBox::try_from(&view)
        }
    }

    impl BoxEncode for TfraBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            1 // version
                + 3 // flags
                + 4 // track_id
                + 4 // reserved and length_size fields
                + 4 // number_of_entry
                + self.entries.len()
                    * (if self.version == 1 { 8 } else { 4 } // time
                        + if self.version == 1 { 8 } else { 4 } // moof_offset
                        + (self.length_size_of_traf_num + 1) as usize // traf_number
                        + (self.length_size_of_trun_num + 1) as usize // trun_number
                        + (self.length_size_of_sample_num + 1) as usize) // sample_number
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_be_bytes())?;

            cur.write_u32_be(self.track_id)?;

            let reserved_and_length: u32 = ((self.length_size_of_traf_num as u32 & 0x03) << 4)
                | ((self.length_size_of_trun_num as u32 & 0x03) << 2)
                | (self.length_size_of_sample_num as u32 & 0x03);
            cur.write_u32_be(reserved_and_length)?;

            cur.write_u32_be(self.entries.len() as u32)?;

            for entry in &self.entries {
                if self.version == 0 {
                    cur.write_u32_be(entry.time as u32)?;
                    cur.write_u32_be(entry.moof_offset as u32)?;
                } else {
                    cur.write_u64_be(entry.time)?;
                    cur.write_u64_be(entry.moof_offset)?;
                }

                let traf_number_size = self.length_size_of_traf_num + 1;
                match traf_number_size {
                    1 => cur.write_u8(entry.traf_number as u8)?,
                    2 => cur.write_u16_be(entry.traf_number as u16)?,
                    3 => {
                        let bytes = (entry.traf_number & 0x00FF_FFFF).to_be_bytes();
                        cur.write_slice(&bytes[1..4])?;
                    }
                    4 => cur.write_u32_be(entry.traf_number)?,
                    _ => unreachable!("Invalid length size validated during encoding"),
                }

                let trun_number_size = self.length_size_of_trun_num + 1;
                match trun_number_size {
                    1 => cur.write_u8(entry.trun_number as u8)?,
                    2 => cur.write_u16_be(entry.trun_number as u16)?,
                    3 => {
                        let bytes = (entry.trun_number & 0x00FF_FFFF).to_be_bytes();
                        cur.write_slice(&bytes[1..4])?;
                    }
                    4 => cur.write_u32_be(entry.trun_number)?,
                    _ => unreachable!("Invalid length size validated during encoding"),
                }

                let sample_number_size = self.length_size_of_sample_num + 1;
                match sample_number_size {
                    1 => cur.write_u8(entry.sample_number as u8)?,
                    2 => cur.write_u16_be(entry.sample_number as u16)?,
                    3 => {
                        let bytes = (entry.sample_number & 0x00FF_FFFF).to_be_bytes();
                        cur.write_slice(&bytes[1..4])?;
                    }
                    4 => cur.write_u32_be(entry.sample_number)?,
                    _ => unreachable!("Invalid length size validated during encoding"),
                }
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

    fn raw_data_v0_empty() -> [u8; 16] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x01, // track_id = 1
            0x00, 0x00, 0x00, 0x00, // reserved(26) + length_size fields (all 0 = 1-byte each)
            0x00, 0x00, 0x00, 0x00, // number_of_entry = 0
        ]
    }

    fn raw_data_v0_with_entry() -> [u8; 27] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x01, // track_id = 1
            0x00, 0x00, 0x00, 0x00, // reserved(26) + length_size fields (all 0 = 1-byte each)
            0x00, 0x00, 0x00, 0x01, // number_of_entry = 1
            // entry (v0: time=4, moof_offset=4, traf/trun/sample=1 each = 11 bytes)
            0x00, 0x00, 0x03, 0xE8, // time = 1000
            0x00, 0x00, 0x10, 0x00, // moof_offset = 4096
            0x01, // traf_number = 1
            0x01, // trun_number = 1
            0x01, // sample_number = 1
        ]
    }

    fn raw_data_v1_with_entry() -> [u8; 35] {
        [
            0x01, // version = 1
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x01, // track_id = 1
            0x00, 0x00, 0x00, 0x00, // reserved(26) + length_size fields (all 0 = 1-byte each)
            0x00, 0x00, 0x00, 0x01, // number_of_entry = 1
            // entry (v1: time=8, moof_offset=8, traf/trun/sample=1 each = 19 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0xE8, // time = 1000
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, // moof_offset = 4096
            0x01, // traf_number = 1
            0x01, // trun_number = 1
            0x01, // sample_number = 1
        ]
    }

    #[test]
    fn test_tfra_box_view_decode_empty() {
        let data = raw_data_v0_empty();
        let tfra = TfraBoxView::decode(&data).unwrap();

        assert_eq!(tfra.version, 0);
        assert_eq!(tfra.flags.bits(), 0);
        assert_eq!(tfra.track_id, 1);
        assert_eq!(tfra.number_of_entry, 0);
        assert_eq!(tfra.entries().count(), 0);
    }

    #[test]
    fn test_tfra_box_view_decode_v0_with_entry() {
        let data = raw_data_v0_with_entry();
        let tfra = TfraBoxView::decode(&data).unwrap();

        assert_eq!(tfra.version, 0);
        assert_eq!(tfra.track_id, 1);
        assert_eq!(tfra.number_of_entry, 1);

        let entries: Vec<_> = tfra.entries().collect();
        assert_eq!(entries.len(), 1);

        let entry = entries[0].as_ref().unwrap();
        assert_eq!(entry.time, 1000);
        assert_eq!(entry.moof_offset, 4096);
        assert_eq!(entry.traf_number, 1);
        assert_eq!(entry.trun_number, 1);
        assert_eq!(entry.sample_number, 1);
    }

    #[test]
    fn test_tfra_box_view_decode_v1_with_entry() {
        let data = raw_data_v1_with_entry();
        let tfra = TfraBoxView::decode(&data).unwrap();

        assert_eq!(tfra.version, 1);
        assert_eq!(tfra.track_id, 1);
        assert_eq!(tfra.number_of_entry, 1);

        let entries: Vec<_> = tfra.entries().collect();
        assert_eq!(entries.len(), 1);

        let entry = entries[0].as_ref().unwrap();
        assert_eq!(entry.time, 1000);
        assert_eq!(entry.moof_offset, 4096);
    }

    #[test]
    fn test_tfra_box_view_decode_truncated() {
        let data: [u8; 8] = [0x00; 8];
        let result = TfraBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_tfra_box_view_decode_entry_count_mismatch() {
        let data: [u8; 20] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x01, // track_id = 1
            0x00, 0x00, 0x00, 0x00, // reserved + length_size fields
            0x00, 0x00, 0x00, 0x02, // number_of_entry = 2 (but only partial data)
            0x00, 0x00, 0x00, 0x00, // incomplete entry data
        ];

        let result = TfraBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_tfra_box_round_trip_v0() {
        use crate::BoxEncode;

        let original = raw_data_v0_with_entry();
        let tfra = TfraBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; tfra.encoded_len()];
        let len = tfra.encode_into(&mut encoded).unwrap();

        assert_eq!(len, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_tfra_box_round_trip_v1() {
        use crate::BoxEncode;

        let original = raw_data_v1_with_entry();
        let tfra = TfraBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; tfra.encoded_len()];
        let len = tfra.encode_into(&mut encoded).unwrap();

        assert_eq!(len, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_tfra_box_try_from() {
        let data = raw_data_v0_with_entry();
        let view = TfraBoxView::decode(&data).unwrap();
        let owned = TfraBox::try_from(&view).unwrap();

        assert_eq!(owned.version, view.version);
        assert_eq!(owned.track_id, view.track_id);
        assert_eq!(owned.entries.len(), view.number_of_entry as usize);
    }
}
