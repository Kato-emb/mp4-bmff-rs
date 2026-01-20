use core::mem;

use crate::BoxType;
use crate::cursor::ReadCursor;
use crate::error::*;
use crate::header::FullBoxFlags;
use crate::types::*;

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
        let version = cur
            .read_u8()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let flags = TkhdFlags::from_bytes(
            cur.read_array()
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
        );

        let (creation_time, modification_time, track_id, duration) = match version {
            1 => {
                let creation_time = cur
                    .read_u64_be()
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
                let modification_time = cur
                    .read_u64_be()
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
                let track_id = cur
                    .read_u32_be()
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
                cur.advance(Self::RESERVED_1_SIZE)
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
                let duration = cur
                    .read_u64_be()
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
                (
                    QuickTimeDateTime::from_quicktime_seconds(creation_time),
                    QuickTimeDateTime::from_quicktime_seconds(modification_time),
                    track_id,
                    duration,
                )
            }
            0 => {
                let creation_time = cur
                    .read_u32_be()
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?
                    as u64;
                let modification_time = cur
                    .read_u32_be()
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?
                    as u64;
                let track_id = cur
                    .read_u32_be()
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
                cur.advance(Self::RESERVED_1_SIZE)
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
                let duration = cur
                    .read_u32_be()
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?
                    as u64;
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

        cur.advance(Self::RESERVED_2_SIZE)
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        let layer = cur
            .read_i16_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let alternate_group = cur
            .read_i16_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let volume = cur
            .read_u16_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let volume = U8F8::from_raw(volume);

        cur.advance(Self::RESERVED_3_SIZE)
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let mut matrix = [0i32; 9];
        for m in &mut matrix {
            *m = cur
                .read_i32_be()
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        }
        let matrix = Matrix::from_raw(matrix);

        let width = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let width = U16F16::from_raw(width);
        let height = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
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
    fn parse_version0_tkhd_box() {
        let flags = TkhdFlags::TRACK_ENABLED | TkhdFlags::TRACK_IN_MOVIE;
        let creation_time: u32 = 0x0102_0304;
        let modification_time: u32 = 0x0506_0708;
        let track_id: u32 = 0x1122_3344;
        let duration: u32 = 0x99AA_BBCC;
        let layer: i16 = 3;
        let alternate_group: i16 = 4;
        let volume_raw: u16 = 0x0100;
        let matrix_raw: [i32; 9] = Matrix::identity().to_raw();
        let width_raw: u32 = 0x0002_0000;
        let height_raw: u32 = 0x0003_0000;

        let mut payload = Vec::new();
        payload.push(0); // version
        payload.extend_from_slice(&[
            (flags.get() >> 16) as u8,
            (flags.get() >> 8) as u8,
            flags.get() as u8,
        ]);
        payload.extend_from_slice(&creation_time.to_be_bytes());
        payload.extend_from_slice(&modification_time.to_be_bytes());
        payload.extend_from_slice(&track_id.to_be_bytes());
        payload.extend_from_slice(&0u32.to_be_bytes()); // reserved
        payload.extend_from_slice(&duration.to_be_bytes());
        payload.extend_from_slice(&0u32.to_be_bytes()); // reserved[0]
        payload.extend_from_slice(&0u32.to_be_bytes()); // reserved[1]
        payload.extend_from_slice(&layer.to_be_bytes());
        payload.extend_from_slice(&alternate_group.to_be_bytes());
        payload.extend_from_slice(&volume_raw.to_be_bytes());
        payload.extend_from_slice(&0u16.to_be_bytes()); // reserved
        for &value in &matrix_raw {
            payload.extend_from_slice(&value.to_be_bytes());
        }
        payload.extend_from_slice(&width_raw.to_be_bytes());
        payload.extend_from_slice(&height_raw.to_be_bytes());

        let tkhd = TkhdBox::parse(&payload).expect("parse version 0 tkhd");

        assert_eq!(tkhd.version, 0);
        assert_eq!(tkhd.flags.get(), flags.get());
        assert_eq!(
            tkhd.creation_time.to_quicktime_seconds(),
            creation_time as u64
        );
        assert_eq!(
            tkhd.modification_time.to_quicktime_seconds(),
            modification_time as u64
        );
        assert_eq!(tkhd.track_id, track_id);
        assert_eq!(tkhd.duration, duration as u64);
        assert_eq!(tkhd.layer, layer);
        assert_eq!(tkhd.alternate_group, alternate_group);
        assert_eq!(tkhd.volume.to_raw(), volume_raw);
        assert_eq!(tkhd.matrix.to_raw(), matrix_raw);
        assert_eq!(tkhd.width.to_raw(), width_raw);
        assert_eq!(tkhd.height.to_raw(), height_raw);
    }

    #[test]
    fn parse_version1_tkhd_box() {
        let flags = TkhdFlags::TRACK_IN_PREVIEW;
        let creation_time: u64 = 0x0102_0304_0506_0708;
        let modification_time: u64 = 0x1112_1314_1516_1718;
        let track_id: u32 = 0x5566_7788;
        let duration: u64 = 0x2222_3333_4444_5555;
        let layer: i16 = -2;
        let alternate_group: i16 = 7;
        let volume_raw: u16 = 0x0080;
        let matrix_raw: [i32; 9] = [
            0x0001_0000,
            0x0000_0001,
            0x0000_0002,
            0x0000_0003,
            0x0001_0000,
            0x0000_0004,
            0x0000_0005,
            0x0000_0006,
            0x4000_0000,
        ];
        let width_raw: u32 = 0x000A_0000;
        let height_raw: u32 = 0x000B_0000;

        let mut payload = Vec::new();
        payload.push(1); // version
        payload.extend_from_slice(&[
            (flags.get() >> 16) as u8,
            (flags.get() >> 8) as u8,
            flags.get() as u8,
        ]);
        payload.extend_from_slice(&creation_time.to_be_bytes());
        payload.extend_from_slice(&modification_time.to_be_bytes());
        payload.extend_from_slice(&track_id.to_be_bytes());
        payload.extend_from_slice(&0u32.to_be_bytes()); // reserved
        payload.extend_from_slice(&duration.to_be_bytes());
        payload.extend_from_slice(&0u32.to_be_bytes()); // reserved[0]
        payload.extend_from_slice(&0u32.to_be_bytes()); // reserved[1]
        payload.extend_from_slice(&layer.to_be_bytes());
        payload.extend_from_slice(&alternate_group.to_be_bytes());
        payload.extend_from_slice(&volume_raw.to_be_bytes());
        payload.extend_from_slice(&0u16.to_be_bytes()); // reserved
        for &value in &matrix_raw {
            payload.extend_from_slice(&value.to_be_bytes());
        }
        payload.extend_from_slice(&width_raw.to_be_bytes());
        payload.extend_from_slice(&height_raw.to_be_bytes());

        let tkhd = TkhdBox::parse(&payload).expect("parse version 1 tkhd");

        assert_eq!(tkhd.version, 1);
        assert_eq!(tkhd.flags.get(), flags.get());
        assert_eq!(tkhd.creation_time.to_quicktime_seconds(), creation_time);
        assert_eq!(
            tkhd.modification_time.to_quicktime_seconds(),
            modification_time
        );
        assert_eq!(tkhd.track_id, track_id);
        assert_eq!(tkhd.duration, duration);
        assert_eq!(tkhd.layer, layer);
        assert_eq!(tkhd.alternate_group, alternate_group);
        assert_eq!(tkhd.volume.to_raw(), volume_raw);
        assert_eq!(tkhd.matrix.to_raw(), matrix_raw);
        assert_eq!(tkhd.width.to_raw(), width_raw);
        assert_eq!(tkhd.height.to_raw(), height_raw);
    }
}
