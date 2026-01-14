use core::mem;

use crate::boxes::FullBoxHeader;
use crate::boxes::error::*;
use crate::boxes::header::FullBoxFlags;
use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;
use crate::types::*;

#[derive(Debug, Clone)]
pub struct TkhdBox {
    pub flags: TkhdFlags,
    pub creation_time: QuickTimeDateTime,
    pub modification_time: QuickTimeDateTime,
    pub track_id: u32,
    pub duration: u64,
    pub layer: i16,
    pub alternate_group: i16,
    pub volume: U8F8,
    pub matrix: Matrix,
    pub width: U16F16,
    pub height: U16F16,
}

impl TkhdBox {
    const RESERVED_1_SIZE: usize = mem::size_of::<u32>();
    const RESERVED_2_SIZE: usize = mem::size_of::<u32>() * 2;
    const RESERVED_3_SIZE: usize = mem::size_of::<u16>();

    pub fn version(&self) -> u8 {
        if self.creation_time.to_quicktime_seconds() > u32::MAX as u64
            || self.modification_time.to_quicktime_seconds() > u32::MAX as u64
            || self.duration > u32::MAX as u64
        {
            1
        } else {
            0
        }
    }

    pub fn parse(payload: &[u8]) -> Result<TkhdBox> {
        let at = |e: ErrorKind, cur: &ReadCursor| Error::new(e).at(cur.position() as u64);
        let mut cur = ReadCursor::new(payload);

        let full_box_header = FullBoxHeader::<TkhdBox>::parse(&mut cur)?;

        let (creation_time, modification_time, track_id, duration) = match full_box_header.version()
        {
            0 => {
                let creation_time = cur.read_u64_be().map_err(|e| at(e.into(), &cur))?;
                let modification_time = cur.read_u64_be().map_err(|e| at(e.into(), &cur))?;
                let track_id = cur.read_u32_be().map_err(|e| at(e.into(), &cur))?;
                cur.advance(Self::RESERVED_1_SIZE)
                    .map_err(|e| at(e.into(), &cur))?;
                let duration = cur.read_u64_be().map_err(|e| at(e.into(), &cur))?;
                (
                    QuickTimeDateTime::from_quicktime_seconds(creation_time),
                    QuickTimeDateTime::from_quicktime_seconds(modification_time),
                    track_id,
                    duration,
                )
            }
            1 => {
                let creation_time = cur.read_u32_be().map_err(|e| at(e.into(), &cur))? as u64;
                let modification_time = cur.read_u32_be().map_err(|e| at(e.into(), &cur))? as u64;
                let track_id = cur.read_u32_be().map_err(|e| at(e.into(), &cur))?;
                cur.advance(Self::RESERVED_1_SIZE)
                    .map_err(|e| at(e.into(), &cur))?;
                let duration = cur.read_u32_be().map_err(|e| at(e.into(), &cur))? as u64;
                (
                    QuickTimeDateTime::from_quicktime_seconds(creation_time),
                    QuickTimeDateTime::from_quicktime_seconds(modification_time),
                    track_id,
                    duration,
                )
            }
            v => {
                return Err(Error::new(ErrorKind::InvalidBoxVersion {
                    reason: "unsupported tkhd version",
                    got: v,
                })
                .at(cur.position() as u64));
            }
        };

        cur.advance(Self::RESERVED_2_SIZE)
            .map_err(|e| at(e.into(), &cur))?;

        let layer = cur.read_i16_be().map_err(|e| at(e.into(), &cur))?;
        let alternate_group = cur.read_i16_be().map_err(|e| at(e.into(), &cur))?;
        let volume = cur.read_u16_be().map_err(|e| at(e.into(), &cur))?;
        let volume = U8F8::from_raw(volume);

        cur.advance(Self::RESERVED_3_SIZE)
            .map_err(|e| at(e.into(), &cur))?;

        let mut matrix = [0i32; 9];
        for m in &mut matrix {
            *m = cur.read_i32_be().map_err(|e| at(e.into(), &cur))?;
        }
        let matrix = Matrix::from_raw(matrix);

        let width = cur.read_u32_be().map_err(|e| at(e.into(), &cur))?;
        let width = U16F16::from_raw(width);
        let height = cur.read_u32_be().map_err(|e| at(e.into(), &cur))?;
        let height = U16F16::from_raw(height);

        Ok(TkhdBox {
            flags: full_box_header.flags().clone(),
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

    pub fn write(&self, cur: &mut WriteCursor) -> Result<()> {
        // let version = self.v
        todo!()
    }
}

pub type TkhdFlags = FullBoxFlags<TkhdBox>;

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
