//! Edit List Box (`elst`) implementation.
//!
//! The Edit List Box contains an explicit timeline map. Each entry defines
//! a segment of the track timeline by specifying a span of presentation time
//! and the corresponding media time. This enables operations like trimming,
//! looping, and inserting empty (dwell) time.

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::types::I16F16;

use crate::cursor::ReadCursor;

define_box_flags!(
    /// Flags for the Edit List Box (`elst`).
    ElstFlags {}
);

/// An entry in the Edit List Box (`elst`).
///
/// Each entry defines how a segment of the track timeline maps to media samples.
///
/// # Structure
///
/// - `segment_duration`: Duration of this edit segment in movie timescale units.
/// - `media_time`: Starting time in media timescale (-1 indicates an empty edit).
/// - `media_rate`: Playback rate as signed 16.16 fixed-point (1.0 = normal forward, 0 = dwell).
#[derive(Debug, Clone, Copy)]
pub struct ElstEntry {
    /// Duration of this edit segment in movie timescale units.
    pub segment_duration: u64,
    /// Starting media time in media timescale units (-1 = empty edit/dwell).
    pub media_time: i64,
    /// Playback rate as signed 16.16 fixed-point (1.0 = normal forward, 0 = dwell).
    pub media_rate: I16F16,
}

/// A reference to an Edit List Box (`elst`).
///
/// The Edit List Box defines the timeline mapping for a track. An edit list
/// with a single entry that maps the entire media is common; more complex
/// lists enable trimming, looping, or inserting gaps.
///
/// # Structure
///
/// - `version`: 0 uses 32-bit time fields, 1 uses 64-bit.
/// - `flags`: Reserved (should be 0).
/// - `entry_count`: Number of edit segments.
/// - `entries`: The edit list entries defining the timeline mapping.
#[derive(Debug)]
pub struct ElstBoxView<'a> {
    /// Box version (0 or 1). Version 1 uses 64-bit time fields.
    pub version: u8,
    /// Reserved flags (should be 0).
    pub flags: ElstFlags,
    /// Number of edit list entries.
    pub entry_count: u32,
    entries: &'a [u8],
}

impl<'a> ElstBoxView<'a> {
    /// Returns an iterator over the ELST entries.
    ///
    /// # Errors
    ///
    /// Returns an error if the entries data is not properly formatted according to the version and entry count.
    pub fn entries(&'a self) -> Result<impl Iterator<Item = ElstEntry> + 'a> {
        let chunk_size = match self.version {
            0 => 12,
            1 => 20,
            _ => {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxVersion {
                        reason: "ELST box version must be 0 or 1",
                        got: self.version,
                    },
                    BoxType::ELST,
                ));
            }
        };

        Ok(self.entries.chunks_exact(chunk_size).map(|bytes| {
            if self.version == 0 {
                ElstEntry {
                    segment_duration: u64::from(u32::from_be_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3],
                    ])),
                    media_time: i64::from(i32::from_be_bytes([
                        bytes[4], bytes[5], bytes[6], bytes[7],
                    ])),
                    media_rate: I16F16::from_raw(i32::from_be_bytes([
                        bytes[8], bytes[9], bytes[10], bytes[11],
                    ])),
                }
            } else {
                ElstEntry {
                    segment_duration: u64::from_be_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                        bytes[7],
                    ]),
                    media_time: i64::from_be_bytes([
                        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                        bytes[15],
                    ]),
                    media_rate: I16F16::from_raw(i32::from_be_bytes([
                        bytes[16], bytes[17], bytes[18], bytes[19],
                    ])),
                }
            }
        }))
    }
}

impl BoxCodec for ElstBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::ELST
    }
}

impl<'de> BoxDecode<'de> for ElstBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = ElstFlags::from_be_bytes(cur.read_array()?);

        let entry_count = cur.read_u32_be()?;

        let expected_size = match version {
            0 => entry_count as usize * 12,
            1 => entry_count as usize * 20,
            _ => {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxVersion {
                        reason: "ELST box version must be 0 or 1",
                        got: version,
                    },
                    BoxType::ELST,
                ));
            }
        };

        if cur.remaining() != expected_size {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Entries length does not match entry count and version",
                    got: cur.remaining() as u64,
                },
                BoxType::ELST,
            ));
        }

        let entries = cur.take(expected_size)?;

        Ok(ElstBoxView {
            version,
            flags,
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

    /// An owned Edit List Box (`elst`).
    ///
    /// This is the owned variant of [`ElstBoxView`] that stores edit entries
    /// in a heap-allocated vector.
    #[derive(Debug, Clone)]
    pub struct ElstBox {
        /// Box version (0 or 1). Version 1 uses 64-bit time fields.
        pub version: u8,
        /// Reserved flags (should be 0).
        pub flags: ElstFlags,
        /// Edit list entries defining timeline-to-media mapping.
        pub entries: Vec<ElstEntry>,
    }

    impl ElstBox {
        /// Creates a new `ElstBox` with the given entries.
        ///
        /// Automatically determines the appropriate version based on the entry values.
        pub fn new(entries: Vec<ElstEntry>) -> Self {
            let version = if entries.iter().any(|e| {
                e.segment_duration > u64::from(u32::MAX)
                    || e.media_time > i64::from(i32::MAX)
                    || e.media_time < i64::from(i32::MIN)
            }) {
                1
            } else {
                0
            };
            ElstBox {
                version,
                flags: ElstFlags::empty(),
                entries,
            }
        }
    }

    impl TryFrom<&ElstBoxView<'_>> for ElstBox {
        type Error = Error;

        fn try_from(value: &ElstBoxView<'_>) -> Result<Self> {
            let entries = value.entries()?.collect();

            Ok(ElstBox {
                version: value.version,
                flags: value.flags,
                entries,
            })
        }
    }

    impl BoxCodec for ElstBox {
        fn boxtype(&self) -> BoxType {
            BoxType::ELST
        }
    }

    impl BoxDecode<'_> for ElstBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = ElstBoxView::decode(bytes)?;
            ElstBox::try_from(&view)
        }
    }

    impl BoxEncode for ElstBox {
        fn encoded_len(&self) -> usize {
            4 // version + flags
                + 4 // entry count
                + self
                    .entries
                    .iter()
                    .map(|_| if self.version == 0 { 12 } else { 20 })
                    .sum::<usize>()
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_be_bytes())?;

            cur.write_u32_be(u32::try_from(self.entries.len())?)?;

            for entry in &self.entries {
                if self.version == 0 {
                    cur.write_u32_be(u32::try_from(entry.segment_duration)?)?;
                    cur.write_i32_be(i32::try_from(entry.media_time)?)?;
                } else {
                    cur.write_u64_be(entry.segment_duration)?;
                    cur.write_i64_be(entry.media_time)?;
                }

                cur.write_i32_be(entry.media_rate.to_raw())?;
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

    fn raw_data_v0() -> [u8; 20] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x00, 0x00, 0x01, // entry_count = 1
            // entry (12 bytes):
            0x00, 0x00, 0x03, 0xE8, // segment_duration = 1000
            0xFF, 0xFF, 0xFF, 0xFF, // media_time = -1
            0x00, 0x01, // media_rate_integer = 1
            0x00, 0x00, // media_rate_fraction = 0
        ]
    }

    fn raw_data_v1() -> [u8; 28] {
        [
            0x01, // version = 1
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x00, 0x00, 0x01, // entry_count = 1
            // entry (20 bytes):
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0xE8, // segment_duration = 1000
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, // media_time = -1
            0x00, 0x01, // media_rate_integer = 1
            0x00, 0x00, // media_rate_fraction = 0
        ]
    }

    #[test]
    fn test_elst_box_view_decode_v0() {
        let data = raw_data_v0();
        let elst = ElstBoxView::decode(&data).unwrap();

        assert_eq!(elst.version, 0);
        assert_eq!(elst.flags.bits(), 0);
        assert_eq!(elst.entry_count, 1);

        let mut entries = elst.entries().unwrap();
        let entry = entries.next().unwrap();
        assert_eq!(entry.segment_duration, 1000);
        assert_eq!(entry.media_time, -1);
        assert_eq!(entry.media_rate, I16F16::from_raw(0x00010000)); // 1.0
        assert!(entries.next().is_none());
    }

    #[test]
    fn test_elst_box_view_decode_v1() {
        let data = raw_data_v1();
        let elst = ElstBoxView::decode(&data).unwrap();

        assert_eq!(elst.version, 1);
        assert_eq!(elst.flags.bits(), 0);
        assert_eq!(elst.entry_count, 1);

        let mut entries = elst.entries().unwrap();
        let entry = entries.next().unwrap();
        assert_eq!(entry.segment_duration, 1000);
        assert_eq!(entry.media_time, -1);
        assert_eq!(entry.media_rate, I16F16::from_raw(0x00010000)); // 1.0
        assert!(entries.next().is_none());
    }

    #[test]
    fn test_elst_box_view_empty_entries() {
        let data: [u8; 8] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x00, 0x00, 0x00, // entry_count = 0
        ];

        let elst = ElstBoxView::decode(&data).unwrap();
        assert_eq!(elst.version, 0);
        assert_eq!(elst.entry_count, 0);
        assert_eq!(elst.entries().unwrap().count(), 0);
    }

    #[test]
    fn test_elst_box_view_invalid_version() {
        let mut data = raw_data_v0();
        data[0] = 2; // invalid version

        let result = ElstBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_elst_box_view_invalid_size() {
        // entry_count = 2 but only 1 entry provided
        let data: [u8; 20] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x00, 0x00, 0x02, // entry_count = 2
            // only 1 entry (12 bytes):
            0x00, 0x00, 0x03, 0xE8, // segment_duration = 1000
            0xFF, 0xFF, 0xFF, 0xFF, // media_time = -1
            0x00, 0x01, // media_rate_integer = 1
            0x00, 0x00, // media_rate_fraction = 0
        ];

        let result = ElstBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_elst_box_view_truncated() {
        let data: [u8; 4] = [0x00; 4]; // too short

        let result = ElstBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_elst_box_round_trip_v0() {
        use crate::BoxEncode;

        let original = raw_data_v0();
        let elst = ElstBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; elst.encoded_len()];
        elst.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_elst_box_round_trip_v1() {
        use crate::BoxEncode;

        let original = raw_data_v1();
        let elst = ElstBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; elst.encoded_len()];
        elst.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_elst_box_try_from() {
        let data = raw_data_v0();
        let view = ElstBoxView::decode(&data).unwrap();
        let owned = ElstBox::try_from(&view).unwrap();

        assert_eq!(owned.version, view.version);
        assert_eq!(owned.flags.bits(), view.flags.bits());
        assert_eq!(owned.entries.len(), view.entry_count as usize);
    }
}
