use core::mem;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;
use crate::types::LanguageCode;
use crate::types::QuickTimeDateTime;

use crate::BoxFrame;
use crate::BoxType;
use crate::error::*;

use super::FullBoxFlags;

/// A Media Header Box (`mdhd`).
///
/// This box contains overall information about the media data in a track,
/// including the timescale and duration.
#[derive(Debug, Clone, Copy)]
pub struct MdhdBox {
    /// Box version (0 or 1).
    pub version: u8,
    /// Box flags (should be 0).
    pub flags: MdhdFlags,
    /// The creation time.
    pub creation_time: QuickTimeDateTime,
    /// The modification time.
    pub modification_time: QuickTimeDateTime,
    /// The time-scale for the media (number of time units per second).
    pub timescale: u32,
    /// The duration of the media in this track (in timescale units).
    pub duration: u64,
    /// The language code (ISO-639-2/T).
    pub language: LanguageCode,
}

impl Default for MdhdBox {
    fn default() -> Self {
        MdhdBox {
            version: 0,
            flags: MdhdFlags::empty(),
            creation_time: QuickTimeDateTime::default(),
            modification_time: QuickTimeDateTime::default(),
            timescale: 0,
            duration: 0,
            language: LanguageCode::UNDETERMINED,
        }
    }
}

impl MdhdBox {
    const PRE_DEFINED_SIZE: usize = mem::size_of::<u16>();

    pub(crate) fn parse_in(cur: &mut ReadCursor<'_>) -> Result<MdhdBox> {
        let version = cur
            .read_u8()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let flags = MdhdFlags::from_bytes(
            cur.read_array()
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
        );

        let (creation_time, modification_time, timescale, duration) = match version {
            1 => {
                let creation_time = cur
                    .read_u64_be()
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
                let modification_time = cur
                    .read_u64_be()
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
                let timescale = cur
                    .read_u32_be()
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
                let duration = cur
                    .read_u64_be()
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
                (
                    QuickTimeDateTime::from_quicktime_seconds(creation_time),
                    QuickTimeDateTime::from_quicktime_seconds(modification_time),
                    timescale,
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
                let timescale = cur
                    .read_u32_be()
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
                let duration = cur
                    .read_u32_be()
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?
                    as u64;
                (
                    QuickTimeDateTime::from_quicktime_seconds(creation_time),
                    QuickTimeDateTime::from_quicktime_seconds(modification_time),
                    timescale,
                    duration,
                )
            }
            other => {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxVersion {
                        reason: "0 or 1 in this specification",
                        got: other,
                    },
                    BoxType::MDHD,
                ));
            }
        };

        let language_packed = cur
            .read_u16_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        let language = LanguageCode::from_packed(language_packed).ok_or_else(|| {
            Error::new(ErrorKind::InvalidBoxField {
                field: "language",
                reason: "invalid packed language code",
            })
            .with_box_type(BoxType::MDHD)
        })?;

        // Skip pre_defined (2 bytes)
        cur.advance(Self::PRE_DEFINED_SIZE)
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        if !cur.is_empty() {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Extra data after parsing mdhd",
                    got: cur.remaining() as u64,
                },
                BoxType::MDHD,
            ));
        }

        Ok(MdhdBox {
            version,
            flags,
            creation_time,
            modification_time,
            timescale,
            duration,
            language,
        })
    }

    /// Parses a `MdhdBox` from the given payload.
    pub fn parse(payload: &[u8]) -> Result<MdhdBox> {
        let mut cursor = ReadCursor::new(payload);
        MdhdBox::parse_in(&mut cursor)
    }

    /// Returns the size of the `MdhdBox` payload in bytes.
    pub fn size(&self) -> usize {
        let base = 4; // version + flags
        let version_dependent = match self.version {
            1 => 8 + 8 + 4 + 8, // u64 creation_time + u64 modification_time + u32 timescale + u64 duration
            _ => 4 + 4 + 4 + 4, // u32 creation_time + u32 modification_time + u32 timescale + u32 duration
        };
        let common = 2 // language
            + Self::PRE_DEFINED_SIZE; // pre_defined
        base + version_dependent + common
    }

    pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
        cur.write_u8(self.version)
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        cur.write_array(&self.flags.to_bytes())
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        match self.version {
            1 => {
                cur.write_u64_be(self.creation_time.to_quicktime_seconds())
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
                cur.write_u64_be(self.modification_time.to_quicktime_seconds())
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
                cur.write_u32_be(self.timescale)
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
                cur.write_u64_be(self.duration)
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
            }
            _ => {
                cur.write_u32_be(self.creation_time.to_quicktime_seconds() as u32)
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
                cur.write_u32_be(self.modification_time.to_quicktime_seconds() as u32)
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
                cur.write_u32_be(self.timescale)
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
                cur.write_u32_be(self.duration as u32)
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
            }
        }

        cur.write_u16_be(self.language.to_packed())
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        // pre_defined
        cur.reserve_zeros(Self::PRE_DEFINED_SIZE)
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        if !cur.is_empty() {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Buffer larger than expected",
                    got: cur.remaining() as u64,
                },
                BoxType::MDHD,
            ));
        }

        Ok(())
    }

    /// Writes this `MdhdBox` into the given payload.
    pub fn write(&self, payload: &mut [u8]) -> Result<()> {
        let mut cursor = WriteCursor::new(payload);
        self.write_in(&mut cursor)
    }
}

impl TryFrom<&[u8]> for MdhdBox {
    type Error = Error;

    fn try_from(payload: &[u8]) -> Result<Self> {
        MdhdBox::parse(payload)
    }
}

impl TryFrom<BoxFrame<'_>> for MdhdBox {
    type Error = Error;

    fn try_from(value: BoxFrame<'_>) -> Result<Self> {
        if value.boxtype() != BoxType::MDHD {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::MDHD,
                found: value.boxtype(),
            }));
        }

        MdhdBox::parse(value.payload())
    }
}

/// Specification for Media Header Box (`mdhd`).
pub struct MdhdSpec;

/// Flags for Media Header Box (`mdhd`).
pub type MdhdFlags = FullBoxFlags<MdhdSpec>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_v0() {
        let mdhd = MdhdBox {
            version: 0,
            flags: MdhdFlags::empty(),
            creation_time: QuickTimeDateTime::from_quicktime_seconds(0x12345678),
            modification_time: QuickTimeDateTime::from_quicktime_seconds(0x23456789),
            timescale: 44100,
            duration: 88200,
            language: LanguageCode::new(*b"eng"),
        };

        let mut buf = vec![0u8; mdhd.size()];
        mdhd.write(&mut buf).unwrap();

        let parsed = MdhdBox::parse(&buf).unwrap();
        assert_eq!(parsed.version, mdhd.version);
        assert_eq!(
            parsed.creation_time.to_quicktime_seconds(),
            mdhd.creation_time.to_quicktime_seconds()
        );
        assert_eq!(
            parsed.modification_time.to_quicktime_seconds(),
            mdhd.modification_time.to_quicktime_seconds()
        );
        assert_eq!(parsed.timescale, mdhd.timescale);
        assert_eq!(parsed.duration, mdhd.duration);
        assert_eq!(parsed.language, mdhd.language);
    }

    #[test]
    fn round_trip_v1() {
        let mdhd = MdhdBox {
            version: 1,
            flags: MdhdFlags::empty(),
            creation_time: QuickTimeDateTime::from_quicktime_seconds(0x0000000112345678),
            modification_time: QuickTimeDateTime::from_quicktime_seconds(0x0000000123456789),
            timescale: 48000,
            duration: 0x0000000200000000, // exceeds u32::MAX
            language: LanguageCode::new(*b"jpn"),
        };

        let mut buf = vec![0u8; mdhd.size()];
        mdhd.write(&mut buf).unwrap();

        let parsed = MdhdBox::parse(&buf).unwrap();
        assert_eq!(parsed.version, mdhd.version);
        assert_eq!(
            parsed.creation_time.to_quicktime_seconds(),
            mdhd.creation_time.to_quicktime_seconds()
        );
        assert_eq!(
            parsed.modification_time.to_quicktime_seconds(),
            mdhd.modification_time.to_quicktime_seconds()
        );
        assert_eq!(parsed.timescale, mdhd.timescale);
        assert_eq!(parsed.duration, mdhd.duration);
        assert_eq!(parsed.language, mdhd.language);
    }
}
