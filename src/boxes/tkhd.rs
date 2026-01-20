use core::mem;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

use crate::BoxType;
use crate::error::*;
use crate::types::*;

use super::FullBoxFlags;

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
            track_id: 1,
            duration: 1,
            layer: 0,
            alternate_group: 0,
            volume: U8F8::from_raw(0x0100), // full volume
            matrix: Matrix::identity(),
            width: U16F16::from_raw(0x00000000),
            height: U16F16::from_raw(0x00000000),
        }
    }
}

impl TkhdBox {
    const RESERVED_1_SIZE: usize = mem::size_of::<u32>();
    const RESERVED_2_SIZE: usize = mem::size_of::<u32>() * 2;
    const RESERVED_3_SIZE: usize = mem::size_of::<u16>();

    pub(crate) fn parse_in(cur: &mut ReadCursor<'_>) -> Result<TkhdBox> {
        let version = cur.read_u8()?;
        let flags = TkhdFlags::from_bytes(cur.read_array()?);

        let (creation_time, modification_time, track_id, duration) = match version {
            1 => {
                let creation_time = cur.read_u64_be()?;
                let modification_time = cur.read_u64_be()?;
                let track_id = cur.read_u32_be()?;
                cur.advance(Self::RESERVED_1_SIZE)?;
                let duration = cur.read_u64_be()?;
                (
                    QuickTimeDateTime::from_quicktime_seconds(creation_time),
                    QuickTimeDateTime::from_quicktime_seconds(modification_time),
                    track_id,
                    duration,
                )
            }
            0 => {
                let creation_time = cur.read_u32_be()? as u64;
                let modification_time = cur.read_u32_be()? as u64;
                let track_id = cur.read_u32_be()?;
                cur.advance(Self::RESERVED_1_SIZE)?;
                let duration = cur.read_u32_be()? as u64;
                (
                    QuickTimeDateTime::from_quicktime_seconds(creation_time),
                    QuickTimeDateTime::from_quicktime_seconds(modification_time),
                    track_id,
                    duration,
                )
            }
            v => {
                return Err(Error::at(
                    ErrorKind::InvalidBoxVersion {
                        reason: "unsupported tkhd version",
                        got: v,
                    },
                    cur.position() as u64,
                ));
            }
        };

        cur.advance(Self::RESERVED_2_SIZE)?;

        let layer = cur.read_i16_be()?;
        let alternate_group = cur.read_i16_be()?;
        let volume = cur.read_u16_be()?;
        let volume = U8F8::from_raw(volume);

        cur.advance(Self::RESERVED_3_SIZE)?;
        let mut matrix = [0i32; 9];
        for m in &mut matrix {
            *m = cur.read_i32_be()?;
        }
        let matrix = Matrix::from_raw(matrix);

        let width = cur.read_u32_be()?;
        let width = U16F16::from_raw(width);
        let height = cur.read_u32_be()?;
        let height = U16F16::from_raw(height);

        if !cur.is_empty() {
            return Err(Error::at_in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Extra data remaining after parsing",
                    got: cur.remaining() as u64,
                },
                cur.position() as u64,
                BoxType::TKHD,
            ));
        }

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

    /// Parses a `TkhdBox` from the given payload.
    pub fn parse(payload: &[u8]) -> Result<TkhdBox> {
        let mut cur = ReadCursor::new(payload);
        let this = TkhdBox::parse_in(&mut cur)?;

        Ok(this)
    }

    /// Returns the size of the `TkhdBox` payload in bytes.
    pub fn size(&self) -> usize {
        let base = 4; // version + flags
        let version_dependent = match self.version {
            1 => 8 + 8 + 4 + 4 + 8, // u64 creation_time + u64 modification_time + u32 track_id + reserved + u64 duration
            _ => 4 + 4 + 4 + 4 + 4, // u32 creation_time + u32 modification_time + u32 track_id + reserved + u32 duration
        };
        let common = Self::RESERVED_2_SIZE // reserved[2]
            + 2 + 2 + 2 // layer + alternate_group + volume
            + Self::RESERVED_3_SIZE // reserved
            + 9 * 4 // matrix
            + 4 + 4; // width + height
        base + version_dependent + common
    }

    pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
        cur.write_u8(self.version)?;
        cur.write_array(&self.flags.to_bytes())?;

        match self.version {
            1 => {
                cur.write_u64_be(self.creation_time.to_quicktime_seconds())?;
                cur.write_u64_be(self.modification_time.to_quicktime_seconds())?;
                cur.write_u32_be(self.track_id)?;
                cur.reserve_zeros(Self::RESERVED_1_SIZE)?;
                cur.write_u64_be(self.duration)?;
            }
            _ => {
                cur.write_u32_be(self.creation_time.to_quicktime_seconds() as u32)?;
                cur.write_u32_be(self.modification_time.to_quicktime_seconds() as u32)?;
                cur.write_u32_be(self.track_id)?;
                cur.reserve_zeros(Self::RESERVED_1_SIZE)?;
                cur.write_u32_be(self.duration as u32)?;
            }
        }

        // reserved[2]
        cur.reserve_zeros(Self::RESERVED_2_SIZE)?;

        cur.write_i16_be(self.layer)?;
        cur.write_i16_be(self.alternate_group)?;
        cur.write_u16_be(self.volume.to_raw())?;

        // reserved
        cur.reserve_zeros(Self::RESERVED_3_SIZE)?;

        // matrix
        for &m in &self.matrix.to_raw() {
            cur.write_i32_be(m)?;
        }

        cur.write_u32_be(self.width.to_raw())?;
        cur.write_u32_be(self.height.to_raw())?;

        if !cur.is_empty() {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Buffer larger than expected",
                    got: cur.remaining() as u64,
                },
                BoxType::TKHD,
            ));
        }

        Ok(())
    }

    /// Writes this `TkhdBox` into the given payload.
    pub fn write(&self, payload: &mut [u8]) -> Result<()> {
        let mut cursor = WriteCursor::new(payload);
        self.write_in(&mut cursor)
    }
}

/// Specification type for Track Header Box (`tkhd`).
pub struct TkhdSpec;

/// Flags for the Track Header Box (`tkhd`).
pub type TkhdFlags = FullBoxFlags<TkhdSpec>;

impl TkhdFlags {
    /// Track is enabled.
    pub const TRACK_ENABLED: TkhdFlags = TkhdFlags::from_bits_truncate(0x000001);
    /// Track is part of the presentation.
    pub const TRACK_IN_MOVIE: TkhdFlags = TkhdFlags::from_bits_truncate(0x000002);
    /// Track is used in the preview.
    pub const TRACK_IN_PREVIEW: TkhdFlags = TkhdFlags::from_bits_truncate(0x000004);
    /// Track size is specified as aspect ratio.
    pub const TRACK_SIZE_IS_ASPECT_RATIO: TkhdFlags = TkhdFlags::from_bits_truncate(0x000008);

    /// Returns true if the track is enabled.
    pub fn is_enabled(&self) -> bool {
        self.contains(Self::TRACK_ENABLED)
    }

    /// Returns true if the track is part of the presentation.
    pub fn is_in_movie(&self) -> bool {
        self.contains(Self::TRACK_IN_MOVIE)
    }

    /// Returns true if the track is used in the preview.
    pub fn is_in_preview(&self) -> bool {
        self.contains(Self::TRACK_IN_PREVIEW)
    }

    /// Returns true if the track size is specified as aspect ratio.
    pub fn is_size_aspect_ratio(&self) -> bool {
        self.contains(Self::TRACK_SIZE_IS_ASPECT_RATIO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_v0() {
        let tkhd = TkhdBox {
            version: 0,
            flags: TkhdFlags::TRACK_ENABLED | TkhdFlags::TRACK_IN_MOVIE,
            creation_time: QuickTimeDateTime::from_quicktime_seconds(0x0102_0304),
            modification_time: QuickTimeDateTime::from_quicktime_seconds(0x0506_0708),
            track_id: 0x1122_3344,
            duration: 0x99AA_BBCC,
            layer: 3,
            alternate_group: 4,
            volume: U8F8::from_raw(0x0100),
            matrix: Matrix::identity(),
            width: U16F16::from_raw(0x0002_0000),
            height: U16F16::from_raw(0x0003_0000),
        };

        let mut buf = vec![0u8; tkhd.size()];
        tkhd.write(&mut buf).unwrap();

        let parsed = TkhdBox::parse(&buf).unwrap();
        assert_eq!(parsed.version, tkhd.version);
        assert_eq!(parsed.flags.get(), tkhd.flags.get());
        assert_eq!(
            parsed.creation_time.to_quicktime_seconds(),
            tkhd.creation_time.to_quicktime_seconds()
        );
        assert_eq!(
            parsed.modification_time.to_quicktime_seconds(),
            tkhd.modification_time.to_quicktime_seconds()
        );
        assert_eq!(parsed.track_id, tkhd.track_id);
        assert_eq!(parsed.duration, tkhd.duration);
        assert_eq!(parsed.layer, tkhd.layer);
        assert_eq!(parsed.alternate_group, tkhd.alternate_group);
        assert_eq!(parsed.volume.to_raw(), tkhd.volume.to_raw());
        assert_eq!(parsed.matrix, tkhd.matrix);
        assert_eq!(parsed.width.to_raw(), tkhd.width.to_raw());
        assert_eq!(parsed.height.to_raw(), tkhd.height.to_raw());
    }

    #[test]
    fn round_trip_v1() {
        let tkhd = TkhdBox {
            version: 1,
            flags: TkhdFlags::TRACK_IN_PREVIEW,
            creation_time: QuickTimeDateTime::from_quicktime_seconds(0x0102_0304_0506_0708),
            modification_time: QuickTimeDateTime::from_quicktime_seconds(0x1112_1314_1516_1718),
            track_id: 0x5566_7788,
            duration: 0x2222_3333_4444_5555, // exceeds u32::MAX
            layer: -2,
            alternate_group: 7,
            volume: U8F8::from_raw(0x0080),
            matrix: Matrix::from_raw([
                0x0001_0000,
                0x0000_0001,
                0x0000_0002,
                0x0000_0003,
                0x0001_0000,
                0x0000_0004,
                0x0000_0005,
                0x0000_0006,
                0x4000_0000,
            ]),
            width: U16F16::from_raw(0x000A_0000),
            height: U16F16::from_raw(0x000B_0000),
        };

        let mut buf = vec![0u8; tkhd.size()];
        tkhd.write(&mut buf).unwrap();

        let parsed = TkhdBox::parse(&buf).unwrap();
        assert_eq!(parsed.version, tkhd.version);
        assert_eq!(parsed.flags.get(), tkhd.flags.get());
        assert_eq!(
            parsed.creation_time.to_quicktime_seconds(),
            tkhd.creation_time.to_quicktime_seconds()
        );
        assert_eq!(
            parsed.modification_time.to_quicktime_seconds(),
            tkhd.modification_time.to_quicktime_seconds()
        );
        assert_eq!(parsed.track_id, tkhd.track_id);
        assert_eq!(parsed.duration, tkhd.duration);
        assert_eq!(parsed.layer, tkhd.layer);
        assert_eq!(parsed.alternate_group, tkhd.alternate_group);
        assert_eq!(parsed.volume.to_raw(), tkhd.volume.to_raw());
        assert_eq!(parsed.matrix, tkhd.matrix);
        assert_eq!(parsed.width.to_raw(), tkhd.width.to_raw());
        assert_eq!(parsed.height.to_raw(), tkhd.height.to_raw());
    }
}
