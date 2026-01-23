use core::mem;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;
use crate::types::LanguageCode;
use crate::types::QuickTimeDateTime;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
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
}

/// Specification for Media Header Box (`mdhd`).
pub struct MdhdSpec;

/// Flags for Media Header Box (`mdhd`).
pub type MdhdFlags = FullBoxFlags<MdhdSpec>;

impl BoxCodec for MdhdBox {
    fn boxtype(&self) -> BoxType {
        BoxType::MDHD
    }
}

impl BoxDecode<'_> for MdhdBox {
    fn decode(bytes: &[u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = MdhdFlags::from_bytes(cur.read_array()?);

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
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxVersion {
                        reason: "0 or 1 in this specification",
                        got: other,
                    },
                    BoxType::MDHD,
                ));
            }
        };

        let language_packed = cur.read_u16_be()?;

        let language = LanguageCode::from_packed(language_packed).ok_or_else(|| {
            Error::new(ErrorKind::InvalidBoxField {
                field: "language",
                reason: "invalid packed language code",
            })
            .with_box_type(BoxType::MDHD)
        })?;

        // Skip pre_defined (2 bytes)
        cur.advance(Self::PRE_DEFINED_SIZE)?;

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
}

impl TryFrom<&[u8]> for MdhdBox {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self> {
        MdhdBox::decode(value)
    }
}

impl BoxEncode for MdhdBox {
    #[inline]
    fn encoded_len(&self) -> usize {
        let base_len = 4; // version(1) + flags(3)
        let time_fields_len = match self.version {
            1 => 8 + 8 + 4 + 8, // creation_time(8) + modification_time(8) + timescale(4) + duration(8)
            _ => 4 + 4 + 4 + 4, // creation_time(4) + modification_time(4) + timescale(4) + duration(4)
        };
        let lang_and_predef_len = 2 + Self::PRE_DEFINED_SIZE; // language(2) + pre_defined(2)

        base_len + time_fields_len + lang_and_predef_len
    }

    fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

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

        cur.write_u16_be(self.language.to_packed())?;

        // pre_defined
        cur.reserve_zeros(Self::PRE_DEFINED_SIZE)?;

        Ok(cur.position())
    }
}

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

        let mut buf = vec![0u8; 256];
        mdhd.encode_into(&mut buf).unwrap();

        let parsed = MdhdBox::decode(&buf).unwrap();
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

        let mut buf = vec![0u8; 256];
        mdhd.encode_into(&mut buf).unwrap();

        let parsed = MdhdBox::decode(&buf).unwrap();
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
