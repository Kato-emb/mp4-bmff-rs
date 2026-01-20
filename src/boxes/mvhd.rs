use core::mem;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;
use crate::types::*;

use crate::error::*;
use crate::header::*;

use super::FullBoxFlags;

/// A reference to a Movie Header Box (`mvhd`).
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
            flags: FullBoxFlags::empty(),
            creation_time: QuickTimeDateTime::default(),
            modification_time: QuickTimeDateTime::default(),
            timescale: 0,
            duration: 0,
            rate: MvhdBox::DEFAULT_RATE,
            volume: MvhdBox::DEFAULT_VOLUME,
            matrix: Matrix::identity(),
            next_track_id: 0,
        }
    }
}

impl MvhdBox {
    const DEFAULT_RATE: I16F16 = I16F16::from_raw(0x00010000); // 1.0 in 16.16 fixed-point
    const DEFAULT_VOLUME: U8F8 = U8F8::from_raw(0x0100); // 1.0 in 8.8 fixed-point

    const RESERVED_SIZE: usize = mem::size_of::<u16>() + 2 * mem::size_of::<u32>(); // reserved
    const PRE_DEFINED_SIZE: usize = 6 * mem::size_of::<u32>(); // pre_defined

    pub(crate) fn parse_in(cur: &mut ReadCursor<'_>) -> Result<MvhdBox> {
        let version = cur.read_u8()?;
        let flags = MvhdFlags::from_bytes(cur.read_array()?);

        let (creation_time, modification_time, timescale, duration) = match version {
            1 => {
                let creation_time = cur.read_u64_be()?;
                let modification_time = cur.read_u64_be()?;
                let timescale = cur.read_u32_be()?;
                let duration = cur.read_u64_be()?;
                (
                    QuickTimeDateTime::from_quicktime_seconds(creation_time),
                    QuickTimeDateTime::from_quicktime_seconds(modification_time),
                    timescale,
                    duration,
                )
            }
            0 => {
                let creation_time = cur.read_u32_be()? as u64;
                let modification_time = cur.read_u32_be()? as u64;
                let timescale = cur.read_u32_be()?;
                let duration = cur.read_u32_be()? as u64;
                (
                    QuickTimeDateTime::from_quicktime_seconds(creation_time),
                    QuickTimeDateTime::from_quicktime_seconds(modification_time),
                    timescale,
                    duration,
                )
            }
            other => {
                return Err(Error::new(ErrorKind::InvalidBoxVersion {
                    reason: "0 or 1 in this specification",
                    got: other,
                }));
            }
        };

        let rate = cur.read_i32_be()?;
        let rate = I16F16::from_raw(rate);
        let volume = cur.read_u16_be()?;
        let volume = U8F8::from_raw(volume);

        cur.advance(MvhdBox::RESERVED_SIZE)?;

        let mut matrix = [0i32; 9];
        for m in &mut matrix {
            *m = cur.read_i32_be()?;
        }
        let matrix = Matrix::from_raw(matrix);

        cur.advance(MvhdBox::PRE_DEFINED_SIZE)?;
        let next_track_id = cur.read_u32_be()?;

        if !cur.is_empty() {
            return Err(Error::at_in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Extra data remaining after parsing",
                    got: cur.remaining() as u64,
                },
                cur.position() as u64,
                BoxType::MVHD,
            ));
        }

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

    /// Parses an `MvhdBox` from the given payload.
    pub fn parse(payload: &[u8]) -> Result<MvhdBox> {
        let mut cursor = ReadCursor::new(payload);
        let this = MvhdBox::parse_in(&mut cursor)?;

        Ok(this)
    }

    /// Returns the size of the `MvhdBox` payload in bytes.
    pub fn size(&self) -> usize {
        let base = 4; // version + flags
        let version_dependent = match self.version {
            1 => 8 + 8 + 4 + 8, // u64 creation_time + u64 modification_time + u32 timescale + u64 duration
            _ => 4 + 4 + 4 + 4, // u32 creation_time + u32 modification_time + u32 timescale + u32 duration
        };
        let common = 4 + 2 // rate + volume
            + Self::RESERVED_SIZE
            + 9 * 4 // matrix
            + Self::PRE_DEFINED_SIZE
            + 4; // next_track_id
        base + version_dependent + common
    }

    pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
        cur.write_u8(self.version)?;
        cur.write_array(&self.flags.to_bytes())?;

        match self.version {
            1 => {
                cur.write_u64_be(self.creation_time.to_quicktime_seconds())?;
                cur.write_u64_be(self.modification_time.to_quicktime_seconds())?;
                cur.write_u32_be(self.timescale)?;
                cur.write_u64_be(self.duration)?;
            }
            _ => {
                cur.write_u32_be(self.creation_time.to_quicktime_seconds() as u32)?;
                cur.write_u32_be(self.modification_time.to_quicktime_seconds() as u32)?;
                cur.write_u32_be(self.timescale)?;
                cur.write_u32_be(self.duration as u32)?;
            }
        }

        cur.write_i32_be(self.rate.to_raw())?;
        cur.write_u16_be(self.volume.to_raw())?;

        // reserved
        cur.reserve_zeros(Self::RESERVED_SIZE)?;

        // matrix
        for &m in &self.matrix.to_raw() {
            cur.write_i32_be(m)?;
        }

        // pre_defined
        cur.reserve_zeros(Self::PRE_DEFINED_SIZE)?;

        cur.write_u32_be(self.next_track_id)?;

        if !cur.is_empty() {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Buffer larger than expected",
                    got: cur.remaining() as u64,
                },
                BoxType::MVHD,
            ));
        }

        Ok(())
    }

    /// Writes this `MvhdBox` into the given payload.
    pub fn write(&self, payload: &mut [u8]) -> Result<()> {
        let mut cursor = WriteCursor::new(payload);
        self.write_in(&mut cursor)
    }
}

/// The specification for the Movie Header Box (`mvhd`).
pub struct MvhdSpec;

/// The flags for the Movie Header Box (`mvhd`).
pub type MvhdFlags = FullBoxFlags<MvhdSpec>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_v0() {
        let mvhd = MvhdBox {
            version: 0,
            flags: MvhdFlags::empty(),
            creation_time: QuickTimeDateTime::from_quicktime_seconds(0x12345678),
            modification_time: QuickTimeDateTime::from_quicktime_seconds(0x23456789),
            timescale: 1000,
            duration: 5000,
            rate: I16F16::from_raw(0x00010000),
            volume: U8F8::from_raw(0x0100),
            matrix: Matrix::identity(),
            next_track_id: 2,
        };

        let mut buf = vec![0u8; mvhd.size()];
        mvhd.write(&mut buf).unwrap();

        let parsed = MvhdBox::parse(&buf).unwrap();
        assert_eq!(parsed.version, mvhd.version);
        assert_eq!(
            parsed.creation_time.to_quicktime_seconds(),
            mvhd.creation_time.to_quicktime_seconds()
        );
        assert_eq!(
            parsed.modification_time.to_quicktime_seconds(),
            mvhd.modification_time.to_quicktime_seconds()
        );
        assert_eq!(parsed.timescale, mvhd.timescale);
        assert_eq!(parsed.duration, mvhd.duration);
        assert_eq!(parsed.rate.to_raw(), mvhd.rate.to_raw());
        assert_eq!(parsed.volume.to_raw(), mvhd.volume.to_raw());
        assert_eq!(parsed.matrix, mvhd.matrix);
        assert_eq!(parsed.next_track_id, mvhd.next_track_id);
    }

    #[test]
    fn round_trip_v1() {
        let mvhd = MvhdBox {
            version: 1,
            flags: MvhdFlags::empty(),
            creation_time: QuickTimeDateTime::from_quicktime_seconds(0x0000000112345678),
            modification_time: QuickTimeDateTime::from_quicktime_seconds(0x0000000123456789),
            timescale: 90000,
            duration: 0x0000000200000000, // exceeds u32::MAX
            rate: I16F16::from_raw(0x00020000),
            volume: U8F8::from_raw(0x0080),
            matrix: Matrix::identity(),
            next_track_id: 5,
        };

        let mut buf = vec![0u8; mvhd.size()];
        mvhd.write(&mut buf).unwrap();

        let parsed = MvhdBox::parse(&buf).unwrap();
        assert_eq!(parsed.version, mvhd.version);
        assert_eq!(
            parsed.creation_time.to_quicktime_seconds(),
            mvhd.creation_time.to_quicktime_seconds()
        );
        assert_eq!(
            parsed.modification_time.to_quicktime_seconds(),
            mvhd.modification_time.to_quicktime_seconds()
        );
        assert_eq!(parsed.timescale, mvhd.timescale);
        assert_eq!(parsed.duration, mvhd.duration);
        assert_eq!(parsed.rate.to_raw(), mvhd.rate.to_raw());
        assert_eq!(parsed.volume.to_raw(), mvhd.volume.to_raw());
        assert_eq!(parsed.matrix, mvhd.matrix);
        assert_eq!(parsed.next_track_id, mvhd.next_track_id);
    }
}
