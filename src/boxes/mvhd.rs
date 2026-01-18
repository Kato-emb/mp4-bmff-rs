use core::mem;

use crate::cursor::ReadCursor;
use crate::types::*;

use crate::error::*;
use crate::header::*;

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

    /// Parses an `MvhdBox` from the given payload.
    pub fn parse(payload: &[u8]) -> Result<MvhdBox> {
        let mut cursor = ReadCursor::new(payload);

        let full_box_header = FullBoxHeader::<MvhdSpec>::parse(&mut cursor)?;

        let (creation_time, modification_time, timescale, duration) =
            match full_box_header.version() {
                1 => {
                    let creation_time = cursor
                        .read_u64_be()
                        .map_err(|e| Error::at(e.into(), cursor.position() as u64))?;
                    let modification_time = cursor
                        .read_u64_be()
                        .map_err(|e| Error::at(e.into(), cursor.position() as u64))?;
                    let timescale = cursor
                        .read_u32_be()
                        .map_err(|e| Error::at(e.into(), cursor.position() as u64))?;
                    let duration = cursor
                        .read_u64_be()
                        .map_err(|e| Error::at(e.into(), cursor.position() as u64))?;
                    (
                        QuickTimeDateTime::from_quicktime_seconds(creation_time),
                        QuickTimeDateTime::from_quicktime_seconds(modification_time),
                        timescale,
                        duration,
                    )
                }
                0 => {
                    let creation_time = cursor
                        .read_u32_be()
                        .map_err(|e| Error::at(e.into(), cursor.position() as u64))?
                        as u64;
                    let modification_time = cursor
                        .read_u32_be()
                        .map_err(|e| Error::at(e.into(), cursor.position() as u64))?
                        as u64;
                    let timescale = cursor
                        .read_u32_be()
                        .map_err(|e| Error::at(e.into(), cursor.position() as u64))?;
                    let duration = cursor
                        .read_u32_be()
                        .map_err(|e| Error::at(e.into(), cursor.position() as u64))?
                        as u64;
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

        let rate = cursor
            .read_i32_be()
            .map_err(|e| Error::at(e.into(), cursor.position() as u64))?;
        let rate = I16F16::from_raw(rate);
        let volume = cursor
            .read_u16_be()
            .map_err(|e| Error::at(e.into(), cursor.position() as u64))?;
        let volume = U8F8::from_raw(volume);

        cursor
            .advance(MvhdBox::RESERVED_SIZE)
            .map_err(|e| Error::at(e.into(), cursor.position() as u64))?;

        let mut matrix = [0i32; 9];
        for m in &mut matrix {
            *m = cursor
                .read_i32_be()
                .map_err(|e| Error::at(e.into(), cursor.position() as u64))?;
        }
        let matrix = Matrix::from_raw(matrix);

        cursor
            .advance(MvhdBox::PRE_DEFINED_SIZE)
            .map_err(|e| Error::at(e.into(), cursor.position() as u64))?;
        let next_track_id = cursor
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cursor.position() as u64))?;

        Ok(MvhdBox {
            version: full_box_header.version(),
            flags: full_box_header.flags(),
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

/// The specification for the Movie Header Box (`mvhd`).
pub struct MvhdSpec;

/// The flags for the Movie Header Box (`mvhd`).
pub type MvhdFlags = FullBoxFlags<MvhdSpec>;

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
        assert_eq!(mvhd.version, 0);
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
        assert_eq!(mvhd.version, 1);
    }

    #[test]
    fn default_values() {
        let mvhd = MvhdBox::default();

        assert_eq!(mvhd.rate.to_raw(), 0x00010000); // 1.0
        assert_eq!(mvhd.volume.to_raw(), 0x0100); // 1.0
        assert_eq!(mvhd.matrix, Matrix::identity());
    }
}
