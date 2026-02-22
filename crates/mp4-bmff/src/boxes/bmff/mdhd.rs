//! Media Header Box (`mdhd`) implementation.
//!
//! The Media Header Box contains overall information about the media data
//! within a track. Unlike the Movie Header Box (`mvhd`) which describes
//! the entire presentation, each `mdhd` describes a single media track's
//! timescale, duration, and language.

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;
use crate::types::*;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

define_box_flags!(
    /// Flags for Media Header Box (`mdhd`).
    ///
    /// Reserved (should be 0).
    MdhdFlags {}
);

/// Media Header Box (`mdhd`).
///
/// The Media Header Box declares information about the media in a track,
/// independent of its coding. It is required within the Media Box (`mdia`).
///
/// # Structure
///
/// - `version`: 0 uses 32-bit time fields, 1 uses 64-bit time fields.
/// - `flags`: Reserved (should be 0).
/// - `creation_time`: When the media was created (QuickTime epoch: 1904-01-01).
/// - `modification_time`: When the media was last modified.
/// - `timescale`: Number of time units per second for this media.
/// - `duration`: Duration of the media in timescale units.
/// - `language`: ISO-639-2/T three-character language code.
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

/// `unsigned int(16) pre_defined = 0`
const PRE_DEFINED: usize = 2;

impl BoxCodec for MdhdBox {
    fn boxtype(&self) -> BoxType {
        BoxType::MDHD
    }
}

impl BoxDecode<'_> for MdhdBox {
    fn decode(bytes: &[u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        // Read version (1 byte)
        let version = cur.read_u8()?;
        // Read flags (3 bytes)
        let flags = MdhdFlags::from_be_bytes(cur.read_array::<3>()?);

        let creation_time: QuickTimeDateTime;
        let modification_time: QuickTimeDateTime;
        let timescale: u32;
        let duration: u64;

        match version {
            0 => {
                // Read creation_time (4 bytes)
                creation_time =
                    QuickTimeDateTime::from_quicktime_seconds(u64::from(cur.read_u32_be()?));
                // Read modification_time (4 bytes)
                modification_time =
                    QuickTimeDateTime::from_quicktime_seconds(u64::from(cur.read_u32_be()?));
                // Read timescale (4 bytes)
                timescale = cur.read_u32_be()?;
                // Read duration (4 bytes)
                duration = u64::from(cur.read_u32_be()?);
            }
            1 => {
                // Read creation_time (8 bytes)
                creation_time = QuickTimeDateTime::from_quicktime_seconds(cur.read_u64_be()?);
                // Read modification_time (8 bytes)
                modification_time = QuickTimeDateTime::from_quicktime_seconds(cur.read_u64_be()?);
                // Read timescale (4 bytes)
                timescale = cur.read_u32_be()?;
                // Read duration (8 bytes)
                duration = cur.read_u64_be()?;
            }
            _ => {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxVersion {
                        reason: "0 or 1 in this specification",
                        got: version,
                    },
                    BoxType::MDHD,
                ));
            }
        }

        let language_packed = cur.read_u16_be()?;
        let language = LanguageCode::from_packed(language_packed).ok_or_else(|| {
            Error::new(ErrorKind::InvalidBoxField {
                field: "language",
                reason: "invalid packed language code",
            })
            .with_box_type(BoxType::MDHD)
        })?;

        // Skip pre_defined
        cur.advance(PRE_DEFINED)?;

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

impl BoxEncode for MdhdBox {
    fn encoded_len(&self) -> usize {
        let mut len = 4; // version (1 byte) + flags (3 bytes)
        len += match self.version {
            0 => 4 + 4 + 4 + 4, // creation_time (4) + modification_time (4) + timescale (4) + duration (4)
            1 => 8 + 8 + 4 + 8, // creation_time (8) + modification_time (8) + timescale (4) + duration (8)
            _ => 0,
        };

        len += 2; // language (2 bytes)
        len += PRE_DEFINED; // pre_defined (2 bytes)
        len
    }

    #[allow(clippy::cast_possible_truncation)]
    fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

        // Write version (1 byte)
        cur.write_u8(self.version)?;
        // Write flags (3 bytes)
        cur.write_array(&self.flags.to_be_bytes())?;

        match self.version {
            0 => {
                // Write creation_time (4 bytes)
                cur.write_u32_be(self.creation_time.to_quicktime_seconds() as u32)?;
                // Write modification_time (4 bytes)
                cur.write_u32_be(self.modification_time.to_quicktime_seconds() as u32)?;
                // Write timescale (4 bytes)
                cur.write_u32_be(self.timescale)?;
                // Write duration (4 bytes)
                cur.write_u32_be(self.duration as u32)?;
            }
            1 => {
                // Write creation_time (8 bytes)
                cur.write_u64_be(self.creation_time.to_quicktime_seconds())?;
                // Write modification_time (8 bytes)
                cur.write_u64_be(self.modification_time.to_quicktime_seconds())?;
                // Write timescale (4 bytes)
                cur.write_u32_be(self.timescale)?;
                // Write duration (8 bytes)
                cur.write_u64_be(self.duration)?;
            }
            _ => {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxVersion {
                        reason: "0 or 1 in this specification",
                        got: self.version,
                    },
                    BoxType::MDHD,
                ));
            }
        }

        // Write language (2 bytes)
        cur.write_u16_be(self.language.to_packed())?;

        // Write pre_defined (2 bytes)
        cur.reserve_zeros(PRE_DEFINED)?;

        Ok(cur.position())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_data_v0() -> [u8; 24] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x00, 0x00, 0x01, // creation_time (4 bytes)
            0x00, 0x00, 0x00, 0x02, // modification_time (4 bytes)
            0x00, 0x00, 0x03, 0xE8, // timescale = 1000 (4 bytes)
            0x00, 0x00, 0x07, 0xD0, // duration = 2000 (4 bytes)
            0x55, 0xC4, // language = "und" (2 bytes)
            0x00, 0x00, // pre_defined (2 bytes)
        ]
    }

    fn raw_data_v1() -> [u8; 36] {
        [
            0x01, // version = 1
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, // creation_time (8 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, // modification_time (8 bytes)
            0x00, 0x00, 0x03, 0xE8, // timescale = 1000 (4 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0xD0, // duration = 2000 (8 bytes)
            0x55, 0xC4, // language = "und" (2 bytes)
            0x00, 0x00, // pre_defined (2 bytes)
        ]
    }

    #[test]
    fn test_mdhd_box_decode_v0() {
        let data = raw_data_v0();
        let mdhd = MdhdBox::decode(&data).unwrap();

        assert_eq!(mdhd.version, 0);
        assert_eq!(mdhd.flags.bits(), 0);
        assert_eq!(mdhd.creation_time.to_quicktime_seconds(), 1);
        assert_eq!(mdhd.modification_time.to_quicktime_seconds(), 2);
        assert_eq!(mdhd.timescale, 1000);
        assert_eq!(mdhd.duration, 2000);
        assert_eq!(mdhd.language, LanguageCode::UNDETERMINED);
    }

    #[test]
    fn test_mdhd_box_decode_v1() {
        let data = raw_data_v1();
        let mdhd = MdhdBox::decode(&data).unwrap();

        assert_eq!(mdhd.version, 1);
        assert_eq!(mdhd.flags.bits(), 0);
        assert_eq!(mdhd.creation_time.to_quicktime_seconds(), 1);
        assert_eq!(mdhd.modification_time.to_quicktime_seconds(), 2);
        assert_eq!(mdhd.timescale, 1000);
        assert_eq!(mdhd.duration, 2000);
        assert_eq!(mdhd.language, LanguageCode::UNDETERMINED);
    }

    #[test]
    fn test_mdhd_box_invalid_version() {
        let mut data = raw_data_v0();
        data[0] = 2; // invalid version

        let result = MdhdBox::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_mdhd_box_truncated() {
        let data: [u8; 10] = [0x00; 10]; // too short

        let result = MdhdBox::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_mdhd_box_round_trip_v0() {
        let original = raw_data_v0();
        let mdhd = MdhdBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; mdhd.encoded_len()];
        mdhd.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[test]
    fn test_mdhd_box_round_trip_v1() {
        let original = raw_data_v1();
        let mdhd = MdhdBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; mdhd.encoded_len()];
        mdhd.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }
}
