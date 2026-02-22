//! Track Header Box (`tkhd`) implementation.
//!
//! The Track Header Box contains characteristics of a single track,
//! including its unique identifier, duration, and visual presentation
//! properties (width, height, transformation matrix).

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
    ///
    /// These flags control track behavior during playback.
    TkhdFlags {
        /// Track is enabled for playback. Disabled tracks are treated as not present.
        TRACK_ENABLED = 0x000001,
        /// Track is used in the presentation. Controls whether track is included.
        TRACK_IN_MOVIE = 0x000002,
        /// Track is used when previewing. Controls preview inclusion.
        TRACK_IN_PREVIEW = 0x000004,
        /// Width and height represent aspect ratio, not actual pixel dimensions.
        TRACK_SIZE_IS_ASPECT_RATIO = 0x000008,
    }
);

/// A Track Header Box (`tkhd`).
///
/// The Track Header Box specifies characteristics of a single track.
/// It is contained within the Track Box (`trak`) and must appear exactly once.
///
/// This is a fixed-size box that implements `Copy`, so there is no separate
/// View/Owned distinction.
///
/// # Structure
///
/// - `version`: Box version (0 or 1). Version 1 uses 64-bit time/duration fields.
/// - `flags`: Track flags controlling enabled/in_movie/in_preview states.
/// - `creation_time`: When the track was created.
/// - `modification_time`: When the track was last modified.
/// - `track_id`: Unique identifier for this track (must be non-zero).
/// - `duration`: Length of the track in movie timescale units.
/// - `layer`: Front-to-back ordering (0 = normal, negative = closer to viewer).
/// - `alternate_group`: Group ID for alternate tracks (0 = no group).
/// - `volume`: Audio playback volume (1.0 = full, 0.0 = mute).
/// - `matrix`: Transformation matrix for visual tracks.
/// - `width`: Visual width as 16.16 fixed-point (0 for audio tracks).
/// - `height`: Visual height as 16.16 fixed-point (0 for audio tracks).
#[derive(Debug, Clone, Copy)]
pub struct TkhdBox {
    /// Box version (0 or 1). Version 1 uses 64-bit time and duration fields.
    pub version: u8,
    /// Track flags (enabled, in_movie, in_preview, size_is_aspect_ratio).
    pub flags: TkhdFlags,
    /// When the track was created (QuickTime epoch: 1904-01-01 UTC).
    pub creation_time: QuickTimeDateTime,
    /// When the track was last modified.
    pub modification_time: QuickTimeDateTime,
    /// Unique identifier for this track (must be non-zero, unique within the movie).
    pub track_id: u32,
    /// Length of the track in movie timescale units (from `mvhd`).
    pub duration: u64,
    /// Front-to-back ordering of video tracks (0 = normal depth).
    pub layer: i16,
    /// Group ID for alternate tracks (0 = not in any alternate group).
    pub alternate_group: i16,
    /// Audio playback volume as 8.8 fixed-point (0x0100 = full volume).
    pub volume: U8F8,
    /// Transformation matrix for video display (rotation, scaling, etc.).
    pub matrix: Matrix,
    /// Track visual width as 16.16 fixed-point pixels (0 for audio tracks).
    pub width: U16F16,
    /// Track visual height as 16.16 fixed-point pixels (0 for audio tracks).
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
                    QuickTimeDateTime::from_quicktime_seconds(u64::from(cur.read_u32_be()?));
                // Read modification time (4 bytes)
                modification_time =
                    QuickTimeDateTime::from_quicktime_seconds(u64::from(cur.read_u32_be()?));
                // Read track ID (4 bytes)
                track_id = cur.read_u32_be()?;
                cur.advance(Self::RESERVED_0)?;
                // Read duration (4 bytes)
                duration = u64::from(cur.read_u32_be()?);
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
                cur.write_u32_be(u32::try_from(self.creation_time.to_quicktime_seconds())?)?;
                // Write modification time (4 bytes)
                cur.write_u32_be(u32::try_from(
                    self.modification_time.to_quicktime_seconds(),
                )?)?;
                // Write track ID (4 bytes)
                cur.write_u32_be(self.track_id)?;
                // Write reserved (4 bytes)
                cur.reserve_zeros(TkhdBox::RESERVED_0)?;
                // Write duration (4 bytes)
                cur.write_u32_be(u32::try_from(self.duration)?)?;
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
