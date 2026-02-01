use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;
use crate::types::*;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

define_box_flags!(
    /// Flags for the Movie Header Box (`mvhd`).
    MvhdFlags {}
);

/// Movie Header Box (`mvhd`).
#[derive(Debug, Clone, Copy)]
pub struct MvhdBox {
    /// The version of the box.
    pub version: u8,
    /// The flags of the box.
    pub flags: MvhdFlags,
    /// The creation time.
    pub creation_time: QuickTimeDateTime,
    /// The modification time.
    pub modification_time: QuickTimeDateTime,
    /// The timescale.
    pub timescale: u32,
    /// The duration.
    pub duration: u64,
    /// The rate.
    pub rate: I16F16,
    /// The volume.
    pub volume: U8F8,
    /// The transformation matrix.
    pub matrix: Matrix,
    /// The next track ID.
    pub next_track_id: u32,
}

impl Default for MvhdBox {
    fn default() -> Self {
        MvhdBox {
            version: 0,
            flags: MvhdFlags::empty(),
            creation_time: QuickTimeDateTime::default(),
            modification_time: QuickTimeDateTime::default(),
            timescale: 0,
            duration: 0,
            rate: I16F16::from_raw(0x00010000),
            volume: U8F8::from_raw(0x0100),
            matrix: Matrix::identity(),
            next_track_id: 1, // non zero
        }
    }
}

/// `bit(16) reserved = 0`
const RESERVED_0: usize = 2;
/// `unsigned int(32)[2] reserved = 0`
const RESERVED_1: usize = 8;
/// `bit(32)[6] pre_defined = 0`
const PRE_DEFINED: usize = 24;

impl BoxCodec for MvhdBox {
    fn boxtype(&self) -> BoxType {
        BoxType::MVHD
    }
}

impl<'de> BoxDecode<'de> for MvhdBox {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        // Read version (1 byte)
        let version = cur.read_u8()?;
        // Read flags (3 bytes)
        let flags = MvhdFlags::from_be_bytes(cur.read_array::<3>()?);

        let creation_time: QuickTimeDateTime;
        let modification_time: QuickTimeDateTime;
        let timescale: u32;
        let duration: u64;

        match version {
            0 => {
                // Read creation_time (4 bytes)
                creation_time =
                    QuickTimeDateTime::from_quicktime_seconds(cur.read_u32_be()? as u64);
                // Read modification_time (4 bytes)
                modification_time =
                    QuickTimeDateTime::from_quicktime_seconds(cur.read_u32_be()? as u64);
                // Read timescale (4 bytes)
                timescale = cur.read_u32_be()?;
                // Read duration (4 bytes)
                duration = cur.read_u32_be()? as u64;
            }
            1 => {
                // Read creation_time (8 bytes)
                creation_time = QuickTimeDateTime::from_quicktime_seconds(cur.read_u64_be()?);
                // Read modification_time (8 bytes)
                modification_time = QuickTimeDateTime::from_quicktime_seconds(cur.read_u64_be()?);
                // Read timescale (4 bytes)
                timescale = cur.read_u32_be()?;
                // Read duration (8 bytes)
                duration = cur.read_u64_be()?;
            }
            _ => {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxVersion {
                        reason: "0 or 1 in this specification",
                        got: version,
                    },
                    BoxType::MVHD,
                ));
            }
        }

        let rate = I16F16::from_raw(cur.read_i32_be()?);
        let volume = U8F8::from_raw(cur.read_u16_be()?);

        // Skip reserved (2 + 8 bytes)
        cur.advance(RESERVED_0 + RESERVED_1)?;

        let mut matrix = [0i32; 9];
        for m in &mut matrix {
            *m = cur.read_i32_be()?;
        }
        let matrix = Matrix::from_raw(matrix);

        // Skip pre_defined (24 bytes)
        cur.advance(PRE_DEFINED)?;
        let next_track_id = cur.read_u32_be()?;

        Ok(MvhdBox {
            version,
            flags,
            creation_time,
            modification_time,
            timescale,
            duration,
            rate,
            volume,
            matrix,
            next_track_id,
        })
    }
}

impl BoxEncode for MvhdBox {
    fn encoded_len(&self) -> usize {
        let mut len = 4; // version (1) + flags (3)
        len += match self.version {
            0 => 4 + 4 + 4 + 4, // creation_time (4) + modification_time (4) + timescale (4) + duration (4)
            1 => 8 + 8 + 4 + 8, // creation_time (8) + modification_time (8) + timescale (4) + duration (8)
            _ => 0,
        };

        len += 4; // rate (4)
        len += 2; // volume (2)
        len += RESERVED_0 + RESERVED_1; // reserved (2 + 8)
        len += 36; // matrix (36)
        len += PRE_DEFINED; // pre_defined (24)
        len += 4; // next_track_id (4)
        len
    }

    fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

        // Write version (1 byte)
        cur.write_u8(self.version)?;
        // Write flags (3 bytes)
        cur.write_array(&self.flags.to_be_bytes())?;

        match self.version {
            0 => {
                // Write creation_time (4 bytes)
                cur.write_u32_be(self.creation_time.to_quicktime_seconds() as u32)?;
                // Write modification_time (4 bytes)
                cur.write_u32_be(self.modification_time.to_quicktime_seconds() as u32)?;
                // Write timescale (4 bytes)
                cur.write_u32_be(self.timescale)?;
                // Write duration (4 bytes)
                cur.write_u32_be(self.duration as u32)?;
            }
            1 => {
                // Write creation_time (8 bytes)
                cur.write_u64_be(self.creation_time.to_quicktime_seconds())?;
                // Write modification_time (8 bytes)
                cur.write_u64_be(self.modification_time.to_quicktime_seconds())?;
                // Write timescale (4 bytes)
                cur.write_u32_be(self.timescale)?;
                // Write duration (8 bytes)
                cur.write_u64_be(self.duration)?;
            }
            other => {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxVersion {
                        reason: "0 or 1 in this specification",
                        got: other,
                    },
                    BoxType::MVHD,
                ));
            }
        }

        // Write rate (4 bytes)
        cur.write_i32_be(self.rate.to_raw())?;
        // Write volume (2 bytes)
        cur.write_u16_be(self.volume.to_raw())?;

        // Write reserved (2 + 8 bytes)
        cur.reserve_zeros(RESERVED_0 + RESERVED_1)?;

        // Write matrix (36 bytes)
        for m in &self.matrix.to_raw() {
            cur.write_i32_be(*m)?;
        }

        // Write pre_defined (24 bytes)
        cur.reserve_zeros(PRE_DEFINED)?;
        // Write next_track_id (4 bytes)
        cur.write_u32_be(self.next_track_id)?;

        Ok(cur.position())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_data_v0() -> [u8; 100] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x00, 0x00, 0x01, // creation_time (4 bytes)
            0x00, 0x00, 0x00, 0x02, // modification_time (4 bytes)
            0x00, 0x00, 0x03, 0xE8, // timescale = 1000 (4 bytes)
            0x00, 0x00, 0x07, 0xD0, // duration = 2000 (4 bytes)
            0x00, 0x01, 0x00, 0x00, // rate = 1.0 (4 bytes)
            0x01, 0x00, // volume = 1.0 (2 bytes)
            0x00, 0x00, // reserved (2 bytes)
            0x00, 0x00, 0x00, 0x00, // reserved (4 bytes)
            0x00, 0x00, 0x00, 0x00, // reserved (4 bytes)
            // matrix (36 bytes) - identity matrix
            0x00, 0x01, 0x00, 0x00, // a = 1.0
            0x00, 0x00, 0x00, 0x00, // b = 0
            0x00, 0x00, 0x00, 0x00, // u = 0
            0x00, 0x00, 0x00, 0x00, // c = 0
            0x00, 0x01, 0x00, 0x00, // d = 1.0
            0x00, 0x00, 0x00, 0x00, // v = 0
            0x00, 0x00, 0x00, 0x00, // x = 0
            0x00, 0x00, 0x00, 0x00, // y = 0
            0x40, 0x00, 0x00, 0x00, // w = 1.0
            // pre_defined (24 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x05, // next_track_id = 5 (4 bytes)
        ]
    }

    fn raw_data_v1() -> [u8; 112] {
        [
            0x01, // version = 1
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, // creation_time (8 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, // modification_time (8 bytes)
            0x00, 0x00, 0x03, 0xE8, // timescale = 1000 (4 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0xD0, // duration = 2000 (8 bytes)
            0x00, 0x01, 0x00, 0x00, // rate = 1.0 (4 bytes)
            0x01, 0x00, // volume = 1.0 (2 bytes)
            0x00, 0x00, // reserved (2 bytes)
            0x00, 0x00, 0x00, 0x00, // reserved (4 bytes)
            0x00, 0x00, 0x00, 0x00, // reserved (4 bytes)
            // matrix (36 bytes) - identity matrix
            0x00, 0x01, 0x00, 0x00, // a = 1.0
            0x00, 0x00, 0x00, 0x00, // b = 0
            0x00, 0x00, 0x00, 0x00, // u = 0
            0x00, 0x00, 0x00, 0x00, // c = 0
            0x00, 0x01, 0x00, 0x00, // d = 1.0
            0x00, 0x00, 0x00, 0x00, // v = 0
            0x00, 0x00, 0x00, 0x00, // x = 0
            0x00, 0x00, 0x00, 0x00, // y = 0
            0x40, 0x00, 0x00, 0x00, // w = 1.0
            // pre_defined (24 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x05, // next_track_id = 5 (4 bytes)
        ]
    }

    #[test]
    fn test_mvhd_box_decode_v0() {
        let data = raw_data_v0();
        let mvhd = MvhdBox::decode(&data).unwrap();

        assert_eq!(mvhd.version, 0);
        assert_eq!(mvhd.flags.bits(), 0);
        assert_eq!(mvhd.creation_time.to_quicktime_seconds(), 1);
        assert_eq!(mvhd.modification_time.to_quicktime_seconds(), 2);
        assert_eq!(mvhd.timescale, 1000);
        assert_eq!(mvhd.duration, 2000);
        assert_eq!(mvhd.rate.to_raw(), 0x00010000);
        assert_eq!(mvhd.volume.to_raw(), 0x0100);
        assert_eq!(mvhd.matrix, Matrix::identity());
        assert_eq!(mvhd.next_track_id, 5);
    }

    #[test]
    fn test_mvhd_box_decode_v1() {
        let data = raw_data_v1();
        let mvhd = MvhdBox::decode(&data).unwrap();

        assert_eq!(mvhd.version, 1);
        assert_eq!(mvhd.flags.bits(), 0);
        assert_eq!(mvhd.creation_time.to_quicktime_seconds(), 1);
        assert_eq!(mvhd.modification_time.to_quicktime_seconds(), 2);
        assert_eq!(mvhd.timescale, 1000);
        assert_eq!(mvhd.duration, 2000);
        assert_eq!(mvhd.rate.to_raw(), 0x00010000);
        assert_eq!(mvhd.volume.to_raw(), 0x0100);
        assert_eq!(mvhd.matrix, Matrix::identity());
        assert_eq!(mvhd.next_track_id, 5);
    }

    #[test]
    fn test_mvhd_box_invalid_version() {
        let mut data = raw_data_v0();
        data[0] = 2; // invalid version

        let result = MvhdBox::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_mvhd_box_truncated() {
        let data: [u8; 10] = [0x00; 10]; // too short

        let result = MvhdBox::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_mvhd_box_round_trip_v0() {
        let original = raw_data_v0();
        let mvhd = MvhdBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; mvhd.encoded_len()];
        mvhd.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[test]
    fn test_mvhd_box_round_trip_v1() {
        let original = raw_data_v1();
        let mvhd = MvhdBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; mvhd.encoded_len()];
        mvhd.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }
}
