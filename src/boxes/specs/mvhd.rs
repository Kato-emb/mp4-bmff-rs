use core::mem;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;
use crate::types::I16F16;
use crate::types::Matrix;
use crate::types::QuickTimeDateTime;
use crate::types::U8F8;

use crate::boxes::error::*;
use crate::boxes::header::*;

/// A reference to a Movie Header Box (`mvhd`).
#[derive(Debug, Clone)]
pub struct MvhdBox {
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

    /// Returns the version of the `mvhd` box based on its fields.
    pub fn version(&self) -> u8 {
        if self.duration > u32::MAX as u64
            || self.creation_time.to_quicktime_seconds() > u32::MAX as u64
            || self.modification_time.to_quicktime_seconds() > u32::MAX as u64
        {
            1
        } else {
            0
        }
    }

    /// Returns the flags of the `mvhd` box.
    pub fn flags(&self) -> FullBoxFlags<MvhdBox> {
        FullBoxFlags::empty()
    }

    /// Parses an `MvhdBox` from the given payload.
    pub fn parse(payload: &[u8]) -> Result<MvhdBox> {
        let mut cursor = ReadCursor::new(payload);

        let full_box_header = FullBoxHeader::<MvhdBox>::parse(&mut cursor)?;

        let (creation_time, modification_time, timescale, duration) =
            if full_box_header.version() == 1 {
                let creation_time = cursor
                    .read_u64_be()
                    .map_err(|e| Error::new(e.into()).at(cursor.position() as u64))?;
                let modification_time = cursor
                    .read_u64_be()
                    .map_err(|e| Error::new(e.into()).at(cursor.position() as u64))?;
                let timescale = cursor
                    .read_u32_be()
                    .map_err(|e| Error::new(e.into()).at(cursor.position() as u64))?;
                let duration = cursor
                    .read_u64_be()
                    .map_err(|e| Error::new(e.into()).at(cursor.position() as u64))?;
                (
                    QuickTimeDateTime::from_quicktime_seconds(creation_time),
                    QuickTimeDateTime::from_quicktime_seconds(modification_time),
                    timescale,
                    duration,
                )
            } else {
                let creation_time = cursor
                    .read_u32_be()
                    .map_err(|e| Error::new(e.into()).at(cursor.position() as u64))?
                    as u64;
                let modification_time = cursor
                    .read_u32_be()
                    .map_err(|e| Error::new(e.into()).at(cursor.position() as u64))?
                    as u64;
                let timescale = cursor
                    .read_u32_be()
                    .map_err(|e| Error::new(e.into()).at(cursor.position() as u64))?;
                let duration = cursor
                    .read_u32_be()
                    .map_err(|e| Error::new(e.into()).at(cursor.position() as u64))?
                    as u64;
                (
                    QuickTimeDateTime::from_quicktime_seconds(creation_time),
                    QuickTimeDateTime::from_quicktime_seconds(modification_time),
                    timescale,
                    duration,
                )
            };

        let rate = cursor
            .read_i32_be()
            .map_err(|e| Error::new(e.into()).at(cursor.position() as u64))?;
        let rate = I16F16::from_raw(rate);
        let volume = cursor
            .read_u16_be()
            .map_err(|e| Error::new(e.into()).at(cursor.position() as u64))?;
        let volume = U8F8::from_raw(volume);

        cursor
            .skip(MvhdBox::RESERVED_SIZE)
            .map_err(|e| Error::new(e.into()).at(cursor.position() as u64))?;

        let mut matrix = [0i32; 9];
        for m in &mut matrix {
            *m = cursor
                .read_i32_be()
                .map_err(|e| Error::new(e.into()).at(cursor.position() as u64))?;
        }
        let matrix = Matrix::from_raw(matrix);

        cursor
            .skip(MvhdBox::PRE_DEFINED_SIZE)
            .map_err(|e| Error::new(e.into()).at(cursor.position() as u64))?;
        let next_track_id = cursor
            .read_u32_be()
            .map_err(|e| Error::new(e.into()).at(cursor.position() as u64))?;

        Ok(MvhdBox {
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

    /// Writes the `MvhdBox` to the given `WriteCursor`.
    pub fn write(&self, cur: &mut WriteCursor) -> Result<()> {
        let version = self.version();
        let full_box_header = FullBoxHeader::new(version, self.flags());
        full_box_header.write(cur)?;

        if version == 1 {
            cur.write_u64_be(self.creation_time.to_quicktime_seconds())
                .map_err(|e| Error::new(e.into()).at(cur.position() as u64))?;
            cur.write_u64_be(self.modification_time.to_quicktime_seconds())
                .map_err(|e| Error::new(e.into()).at(cur.position() as u64))?;
            cur.write_u32_be(self.timescale)
                .map_err(|e| Error::new(e.into()).at(cur.position() as u64))?;
            cur.write_u64_be(self.duration)
                .map_err(|e| Error::new(e.into()).at(cur.position() as u64))?;
        } else {
            cur.write_u32_be(self.creation_time.to_quicktime_seconds() as u32)
                .map_err(|e| Error::new(e.into()).at(cur.position() as u64))?;
            cur.write_u32_be(self.modification_time.to_quicktime_seconds() as u32)
                .map_err(|e| Error::new(e.into()).at(cur.position() as u64))?;
            cur.write_u32_be(self.timescale)
                .map_err(|e| Error::new(e.into()).at(cur.position() as u64))?;
            cur.write_u32_be(self.duration as u32)
                .map_err(|e| Error::new(e.into()).at(cur.position() as u64))?;
        }

        cur.write_i32_be(self.rate.to_raw())
            .map_err(|e| Error::new(e.into()).at(cur.position() as u64))?;
        cur.write_u16_be(self.volume.to_raw())
            .map_err(|e| Error::new(e.into()).at(cur.position() as u64))?;

        cur.reserve_zeros(MvhdBox::RESERVED_SIZE)
            .map_err(|e| Error::new(e.into()).at(cur.position() as u64))?;

        for &m in &self.matrix.to_raw() {
            cur.write_i32_be(m)
                .map_err(|e| Error::new(e.into()).at(cur.position() as u64))?;
        }

        cur.reserve_zeros(MvhdBox::PRE_DEFINED_SIZE)
            .map_err(|e| Error::new(e.into()).at(cur.position() as u64))?;
        cur.write_u32_be(self.next_track_id)
            .map_err(|e| Error::new(e.into()).at(cur.position() as u64))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_v0_payload() -> Vec<u8> {
        let mut data = Vec::new();

        // FullBoxHeader: version=0, flags=0
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

        // creation_time (u32): 0x12345678
        data.extend_from_slice(&0x12345678u32.to_be_bytes());
        // modification_time (u32): 0x23456789
        data.extend_from_slice(&0x23456789u32.to_be_bytes());
        // timescale (u32): 1000
        data.extend_from_slice(&1000u32.to_be_bytes());
        // duration (u32): 5000
        data.extend_from_slice(&5000u32.to_be_bytes());

        // rate (i32): 0x00010000 = 1.0
        data.extend_from_slice(&0x00010000i32.to_be_bytes());
        // volume (u16): 0x0100 = 1.0
        data.extend_from_slice(&0x0100u16.to_be_bytes());

        // reserved: 2 bytes + 2 * 4 bytes = 10 bytes
        data.extend_from_slice(&[0u8; 10]);

        // matrix: identity matrix (9 * i32)
        // [0x00010000, 0, 0, 0, 0x00010000, 0, 0, 0, 0x40000000]
        data.extend_from_slice(&0x00010000i32.to_be_bytes()); // a
        data.extend_from_slice(&0x00000000i32.to_be_bytes()); // b
        data.extend_from_slice(&0x00000000i32.to_be_bytes()); // u
        data.extend_from_slice(&0x00000000i32.to_be_bytes()); // c
        data.extend_from_slice(&0x00010000i32.to_be_bytes()); // d
        data.extend_from_slice(&0x00000000i32.to_be_bytes()); // v
        data.extend_from_slice(&0x00000000i32.to_be_bytes()); // x
        data.extend_from_slice(&0x00000000i32.to_be_bytes()); // y
        data.extend_from_slice(&0x40000000i32.to_be_bytes()); // w

        // pre_defined: 6 * 4 bytes = 24 bytes
        data.extend_from_slice(&[0u8; 24]);

        // next_track_id (u32): 2
        data.extend_from_slice(&2u32.to_be_bytes());

        data
    }

    fn make_v1_payload() -> Vec<u8> {
        let mut data = Vec::new();

        // FullBoxHeader: version=1, flags=0
        data.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);

        // creation_time (u64): 0x0000000112345678
        data.extend_from_slice(&0x0000000112345678u64.to_be_bytes());
        // modification_time (u64): 0x0000000123456789
        data.extend_from_slice(&0x0000000123456789u64.to_be_bytes());
        // timescale (u32): 90000
        data.extend_from_slice(&90000u32.to_be_bytes());
        // duration (u64): 0x0000000200000000 (exceeds u32::MAX)
        data.extend_from_slice(&0x0000000200000000u64.to_be_bytes());

        // rate (i32): 0x00020000 = 2.0
        data.extend_from_slice(&0x00020000i32.to_be_bytes());
        // volume (u16): 0x0080 = 0.5
        data.extend_from_slice(&0x0080u16.to_be_bytes());

        // reserved: 10 bytes
        data.extend_from_slice(&[0u8; 10]);

        // matrix: identity matrix
        data.extend_from_slice(&0x00010000i32.to_be_bytes());
        data.extend_from_slice(&0x00000000i32.to_be_bytes());
        data.extend_from_slice(&0x00000000i32.to_be_bytes());
        data.extend_from_slice(&0x00000000i32.to_be_bytes());
        data.extend_from_slice(&0x00010000i32.to_be_bytes());
        data.extend_from_slice(&0x00000000i32.to_be_bytes());
        data.extend_from_slice(&0x00000000i32.to_be_bytes());
        data.extend_from_slice(&0x00000000i32.to_be_bytes());
        data.extend_from_slice(&0x40000000i32.to_be_bytes());

        // pre_defined: 24 bytes
        data.extend_from_slice(&[0u8; 24]);

        // next_track_id (u32): 5
        data.extend_from_slice(&5u32.to_be_bytes());

        data
    }

    #[test]
    fn parse_v0() {
        let payload = make_v0_payload();
        let mvhd = MvhdBox::parse(&payload).unwrap();

        assert_eq!(mvhd.creation_time.to_quicktime_seconds(), 0x12345678);
        assert_eq!(mvhd.modification_time.to_quicktime_seconds(), 0x23456789);
        assert_eq!(mvhd.timescale, 1000);
        assert_eq!(mvhd.duration, 5000);
        assert_eq!(mvhd.rate.to_raw(), 0x00010000);
        assert_eq!(mvhd.volume.to_raw(), 0x0100);
        assert_eq!(mvhd.matrix, Matrix::identity());
        assert_eq!(mvhd.next_track_id, 2);
        assert_eq!(mvhd.version(), 0);
    }

    #[test]
    fn parse_v1() {
        let payload = make_v1_payload();
        let mvhd = MvhdBox::parse(&payload).unwrap();

        assert_eq!(
            mvhd.creation_time.to_quicktime_seconds(),
            0x0000000112345678
        );
        assert_eq!(
            mvhd.modification_time.to_quicktime_seconds(),
            0x0000000123456789
        );
        assert_eq!(mvhd.timescale, 90000);
        assert_eq!(mvhd.duration, 0x0000000200000000);
        assert_eq!(mvhd.rate.to_raw(), 0x00020000);
        assert_eq!(mvhd.volume.to_raw(), 0x0080);
        assert_eq!(mvhd.matrix, Matrix::identity());
        assert_eq!(mvhd.next_track_id, 5);
        assert_eq!(mvhd.version(), 1);
    }

    #[test]
    fn write_roundtrip_v0() {
        let original = MvhdBox {
            creation_time: QuickTimeDateTime::from_quicktime_seconds(1000),
            modification_time: QuickTimeDateTime::from_quicktime_seconds(2000),
            timescale: 1000,
            duration: 5000,
            rate: I16F16::from_raw(0x00010000),
            volume: U8F8::from_raw(0x0100),
            matrix: Matrix::identity(),
            next_track_id: 3,
        };

        assert_eq!(original.version(), 0);

        // Write
        let mut buf = vec![0u8; 108]; // v0 size
        let mut cursor = WriteCursor::new(&mut buf);
        original.write(&mut cursor).unwrap();

        // Parse back
        let parsed = MvhdBox::parse(&buf).unwrap();

        assert_eq!(parsed.creation_time, original.creation_time);
        assert_eq!(parsed.modification_time, original.modification_time);
        assert_eq!(parsed.timescale, original.timescale);
        assert_eq!(parsed.duration, original.duration);
        assert_eq!(parsed.rate, original.rate);
        assert_eq!(parsed.volume, original.volume);
        assert_eq!(parsed.matrix, original.matrix);
        assert_eq!(parsed.next_track_id, original.next_track_id);
    }

    #[test]
    fn write_roundtrip_v1() {
        let original = MvhdBox {
            creation_time: QuickTimeDateTime::from_quicktime_seconds(0x100000000),
            modification_time: QuickTimeDateTime::from_quicktime_seconds(0x100000001),
            timescale: 90000,
            duration: 0x200000000,
            rate: I16F16::from_raw(0x00020000),
            volume: U8F8::from_raw(0x0080),
            matrix: Matrix::identity(),
            next_track_id: 10,
        };

        assert_eq!(original.version(), 1);

        // Write
        let mut buf = vec![0u8; 120]; // v1 size
        let mut cursor = WriteCursor::new(&mut buf);
        original.write(&mut cursor).unwrap();

        // Parse back
        let parsed = MvhdBox::parse(&buf).unwrap();

        assert_eq!(parsed.creation_time, original.creation_time);
        assert_eq!(parsed.modification_time, original.modification_time);
        assert_eq!(parsed.timescale, original.timescale);
        assert_eq!(parsed.duration, original.duration);
        assert_eq!(parsed.rate, original.rate);
        assert_eq!(parsed.volume, original.volume);
        assert_eq!(parsed.matrix, original.matrix);
        assert_eq!(parsed.next_track_id, original.next_track_id);
    }

    #[test]
    fn version_detection() {
        // duration > u32::MAX -> version 1
        let mut mvhd = MvhdBox::default();
        mvhd.duration = u32::MAX as u64 + 1;
        assert_eq!(mvhd.version(), 1);

        // creation_time > u32::MAX -> version 1
        let mut mvhd = MvhdBox::default();
        mvhd.creation_time = QuickTimeDateTime::from_quicktime_seconds(u32::MAX as u64 + 1);
        assert_eq!(mvhd.version(), 1);

        // modification_time > u32::MAX -> version 1
        let mut mvhd = MvhdBox::default();
        mvhd.modification_time = QuickTimeDateTime::from_quicktime_seconds(u32::MAX as u64 + 1);
        assert_eq!(mvhd.version(), 1);

        // All within u32 range -> version 0
        let mvhd = MvhdBox::default();
        assert_eq!(mvhd.version(), 0);
    }

    #[test]
    fn default_values() {
        let mvhd = MvhdBox::default();

        assert_eq!(mvhd.rate.to_raw(), 0x00010000); // 1.0
        assert_eq!(mvhd.volume.to_raw(), 0x0100); // 1.0
        assert_eq!(mvhd.matrix, Matrix::identity());
    }
}
