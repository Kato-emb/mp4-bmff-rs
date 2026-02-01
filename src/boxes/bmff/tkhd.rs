use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;
use crate::types::*;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

define_box_flags!(
    /// Flags for the Track Header Box (`tkhd`).
    TkhdFlags {
        /// Track is enabled.
        TRACK_ENABLED = 0x000001,
        /// Track is included in the movie.
        TRACK_IN_MOVIE = 0x000002,
        /// Track is included in the preview.
        TRACK_IN_PREVIEW = 0x000004,
        /// Track size is aspect ratio.
        TRACK_SIZE_IS_ASPECT_RATIO = 0x000008,
    }
);

/// A Track Header Box (`tkhd`).
#[derive(Debug, Clone, Copy)]
pub struct TkhdBox {
    /// Box version.
    pub version: u8,
    /// Box flags.
    pub flags: TkhdFlags,
    /// Creation time of the track.
    pub creation_time: QuickTimeDateTime,
    /// Modification time of the track.
    pub modification_time: QuickTimeDateTime,
    /// Track ID.
    pub track_id: u32,
    /// Duration of the track.
    pub duration: u64,
    /// Layer of the track.
    pub layer: i16,
    /// Alternate group of the track.
    pub alternate_group: i16,
    /// Volume of the track.
    pub volume: U8F8,
    /// Transformation matrix.
    pub matrix: Matrix,
    /// Width of the track.
    pub width: U16F16,
    /// Height of the track.
    pub height: U16F16,
}

impl Default for TkhdBox {
    fn default() -> Self {
        TkhdBox {
            version: 0,
            flags: TkhdFlags::empty(),
            creation_time: QuickTimeDateTime::default(),
            modification_time: QuickTimeDateTime::default(),
            track_id: 1, // non-zero
            duration: 0,
            layer: 0,
            alternate_group: 0,
            volume: U8F8::from_raw(0x0000),
            matrix: Matrix::identity(),
            width: U16F16::from_raw(0x00000000),
            height: U16F16::from_raw(0x00000000),
        }
    }
}

impl TkhdBox {
    /// `unsigned int(32) reserved = 0`
    const RESERVED_0: usize = 4;
    /// `unsigned int(32)[2] reserved = 0`
    const RESERVED_1: usize = 8;
    /// `unsigned int(16) reserved = 0`
    const RESERVED_2: usize = 2;
}

impl BoxCodec for TkhdBox {
    fn boxtype(&self) -> BoxType {
        BoxType::TKHD
    }
}

impl BoxDecode<'_> for TkhdBox {
    fn decode(bytes: &'_ [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        // Read version (1 byte)
        let version = cur.read_u8()?;
        // Read flags (3 bytes)
        let flags = TkhdFlags::from_be_bytes(cur.read_array::<3>()?);

        let creation_time: QuickTimeDateTime;
        let modification_time: QuickTimeDateTime;
        let track_id: u32;
        let duration: u64;

        match version {
            0 => {
                // Read creation time (4 bytes)
                creation_time =
                    QuickTimeDateTime::from_quicktime_seconds(cur.read_u32_be()? as u64);
                // Read modification time (4 bytes)
                modification_time =
                    QuickTimeDateTime::from_quicktime_seconds(cur.read_u32_be()? as u64);
                // Read track ID (4 bytes)
                track_id = cur.read_u32_be()?;
                cur.advance(Self::RESERVED_0)?;
                // Read duration (4 bytes)
                duration = cur.read_u32_be()? as u64;
            }
            1 => {
                // Read creation time (8 bytes)
                creation_time = QuickTimeDateTime::from_quicktime_seconds(cur.read_u64_be()?);
                // Read modification time (8 bytes)
                modification_time = QuickTimeDateTime::from_quicktime_seconds(cur.read_u64_be()?);
                // Read track ID (4 bytes)
                track_id = cur.read_u32_be()?;
                cur.advance(Self::RESERVED_0)?;
                // Read duration (8 bytes)
                duration = cur.read_u64_be()?;
            }
            _ => {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxVersion {
                        reason: "0 or 1 in this specification",
                        got: version,
                    },
                    BoxType::TKHD,
                ));
            }
        }

        // Skip reserved (2 * 4 bytes)
        cur.advance(TkhdBox::RESERVED_1)?;

        // Read layer (2 bytes)
        let layer = cur.read_i16_be()?;
        // Read alternate group (2 bytes)
        let alternate_group = cur.read_i16_be()?;

        // Read volume (2 bytes)
        let volume = U8F8::from_raw(cur.read_u16_be()?);

        // Skip reserved (2 bytes)
        cur.advance(TkhdBox::RESERVED_2)?;

        // Read matrix (36 bytes)
        let mut matrix = [0i32; 9];
        for m in &mut matrix {
            *m = cur.read_i32_be()?;
        }
        let matrix = Matrix::from_raw(matrix);

        // Read width (4 bytes)
        let width = U16F16::from_raw(cur.read_u32_be()?);
        // Read height (4 bytes)
        let height = U16F16::from_raw(cur.read_u32_be()?);

        Ok(TkhdBox {
            version,
            flags,
            creation_time,
            modification_time,
            track_id,
            duration,
            layer,
            alternate_group,
            volume,
            matrix,
            width,
            height,
        })
    }
}

impl BoxEncode for TkhdBox {
    fn encoded_len(&self) -> usize {
        let base_len = 4    // version (1) + flags (3)
            + 4             // reserved
            + 8             // reserved (2 * 4)
            + 2             // reserved
            + 2             // layer
            + 2             // alternate_group
            + 2             // volume
            + 36            // matrix (9 * 4)
            + 4             // width
            + 4; // height

        match self.version {
            0 => base_len + 4 + 4 + 4 + 4, // creation + modification + track_id + duration
            1 => base_len + 8 + 8 + 4 + 8, // creation + modification + track_id + duration
            _ => 0,
        }
    }

    fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

        // Write version (1 byte)
        cur.write_u8(self.version)?;
        // Write flags (3 bytes)
        cur.write_array(&self.flags.to_be_bytes())?;

        match self.version {
            0 => {
                // Write creation time (4 bytes)
                cur.write_u32_be(self.creation_time.to_quicktime_seconds() as u32)?;
                // Write modification time (4 bytes)
                cur.write_u32_be(self.modification_time.to_quicktime_seconds() as u32)?;
                // Write track ID (4 bytes)
                cur.write_u32_be(self.track_id)?;
                // Write reserved (4 bytes)
                cur.reserve_zeros(TkhdBox::RESERVED_0)?;
                // Write duration (4 bytes)
                cur.write_u32_be(self.duration as u32)?;
            }
            1 => {
                // Write creation time (8 bytes)
                cur.write_u64_be(self.creation_time.to_quicktime_seconds())?;
                // Write modification time (8 bytes)
                cur.write_u64_be(self.modification_time.to_quicktime_seconds())?;
                // Write track ID (4 bytes)
                cur.write_u32_be(self.track_id)?;
                // Write reserved (4 bytes)
                cur.reserve_zeros(TkhdBox::RESERVED_0)?;
                // Write duration (8 bytes)
                cur.write_u64_be(self.duration)?;
            }
            _ => {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxVersion {
                        reason: "0 or 1 in this specification",
                        got: self.version,
                    },
                    BoxType::TKHD,
                ));
            }
        }

        // Write reserved (2 * 4 bytes)
        cur.reserve_zeros(TkhdBox::RESERVED_1)?;

        // Write layer (2 bytes)
        cur.write_i16_be(self.layer)?;
        // Write alternate group (2 bytes)
        cur.write_i16_be(self.alternate_group)?;

        // Write volume (2 bytes)
        cur.write_u16_be(self.volume.to_raw())?;

        // Write reserved (2 bytes)
        cur.reserve_zeros(TkhdBox::RESERVED_2)?;

        // Write matrix (36 bytes)
        for m in &self.matrix.to_raw() {
            cur.write_i32_be(*m)?;
        }

        // Write width (4 bytes)
        cur.write_u32_be(self.width.to_raw())?;
        // Write height (4 bytes)
        cur.write_u32_be(self.height.to_raw())?;

        Ok(cur.position())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_data_v0() -> [u8; 84] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x07, // flags = 0x000007 (enabled, in_movie, in_preview)
            0x00, 0x00, 0x00, 0x01, // creation_time (4 bytes)
            0x00, 0x00, 0x00, 0x02, // modification_time (4 bytes)
            0x00, 0x00, 0x00, 0x03, // track_id = 3 (4 bytes)
            0x00, 0x00, 0x00, 0x00, // reserved (4 bytes)
            0x00, 0x00, 0x07, 0xD0, // duration = 2000 (4 bytes)
            0x00, 0x00, 0x00, 0x00, // reserved (4 bytes)
            0x00, 0x00, 0x00, 0x00, // reserved (4 bytes)
            0x00, 0x01, // layer = 1 (2 bytes)
            0x00, 0x02, // alternate_group = 2 (2 bytes)
            0x01, 0x00, // volume = 1.0 (2 bytes)
            0x00, 0x00, // reserved (2 bytes)
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
            0x01, 0x40, 0x00, 0x00, // width = 320.0 (4 bytes)
            0x00, 0xF0, 0x00, 0x00, // height = 240.0 (4 bytes)
        ]
    }

    fn raw_data_v1() -> [u8; 96] {
        [
            0x01, // version = 1
            0x00, 0x00, 0x07, // flags = 0x000007 (enabled, in_movie, in_preview)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, // creation_time (8 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, // modification_time (8 bytes)
            0x00, 0x00, 0x00, 0x03, // track_id = 3 (4 bytes)
            0x00, 0x00, 0x00, 0x00, // reserved (4 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0xD0, // duration = 2000 (8 bytes)
            0x00, 0x00, 0x00, 0x00, // reserved (4 bytes)
            0x00, 0x00, 0x00, 0x00, // reserved (4 bytes)
            0x00, 0x01, // layer = 1 (2 bytes)
            0x00, 0x02, // alternate_group = 2 (2 bytes)
            0x01, 0x00, // volume = 1.0 (2 bytes)
            0x00, 0x00, // reserved (2 bytes)
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
            0x01, 0x40, 0x00, 0x00, // width = 320.0 (4 bytes)
            0x00, 0xF0, 0x00, 0x00, // height = 240.0 (4 bytes)
        ]
    }

    #[test]
    fn test_tkhd_box_decode_v0() {
        let data = raw_data_v0();
        let tkhd = TkhdBox::decode(&data).unwrap();

        assert_eq!(tkhd.version, 0);
        assert_eq!(tkhd.flags.bits(), 0x000007);
        assert!(tkhd.flags.contains(TkhdFlags::TRACK_ENABLED));
        assert!(tkhd.flags.contains(TkhdFlags::TRACK_IN_MOVIE));
        assert!(tkhd.flags.contains(TkhdFlags::TRACK_IN_PREVIEW));
        assert!(!tkhd.flags.contains(TkhdFlags::TRACK_SIZE_IS_ASPECT_RATIO));
        assert_eq!(tkhd.creation_time.to_quicktime_seconds(), 1);
        assert_eq!(tkhd.modification_time.to_quicktime_seconds(), 2);
        assert_eq!(tkhd.track_id, 3);
        assert_eq!(tkhd.duration, 2000);
        assert_eq!(tkhd.layer, 1);
        assert_eq!(tkhd.alternate_group, 2);
        assert_eq!(tkhd.volume.to_raw(), 0x0100);
        assert_eq!(tkhd.matrix, Matrix::identity());
        assert_eq!(tkhd.width.to_raw(), 0x01400000); // 320.0
        assert_eq!(tkhd.height.to_raw(), 0x00F00000); // 240.0
    }

    #[test]
    fn test_tkhd_box_decode_v1() {
        let data = raw_data_v1();
        let tkhd = TkhdBox::decode(&data).unwrap();

        assert_eq!(tkhd.version, 1);
        assert_eq!(tkhd.flags.bits(), 0x000007);
        assert!(tkhd.flags.contains(TkhdFlags::TRACK_ENABLED));
        assert_eq!(tkhd.creation_time.to_quicktime_seconds(), 1);
        assert_eq!(tkhd.modification_time.to_quicktime_seconds(), 2);
        assert_eq!(tkhd.track_id, 3);
        assert_eq!(tkhd.duration, 2000);
        assert_eq!(tkhd.layer, 1);
        assert_eq!(tkhd.alternate_group, 2);
        assert_eq!(tkhd.volume.to_raw(), 0x0100);
        assert_eq!(tkhd.matrix, Matrix::identity());
        assert_eq!(tkhd.width.to_raw(), 0x01400000); // 320.0
        assert_eq!(tkhd.height.to_raw(), 0x00F00000); // 240.0
    }

    #[test]
    fn test_tkhd_box_invalid_version() {
        let mut data = raw_data_v0();
        data[0] = 2; // invalid version

        let result = TkhdBox::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_tkhd_box_truncated() {
        let data: [u8; 10] = [0x00; 10]; // too short

        let result = TkhdBox::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_tkhd_box_round_trip_v0() {
        let original = raw_data_v0();
        let tkhd = TkhdBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; tkhd.encoded_len()];
        let written = tkhd.encode_into(&mut encoded).unwrap();

        assert_eq!(written, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }

    #[test]
    fn test_tkhd_box_round_trip_v1() {
        let original = raw_data_v1();
        let tkhd = TkhdBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; tkhd.encoded_len()];
        let written = tkhd.encode_into(&mut encoded).unwrap();

        assert_eq!(written, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }
}
