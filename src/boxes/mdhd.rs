use core::mem;

use crate::cursor::ReadCursor;
use crate::types::LanguageCode;
use crate::types::QuickTimeDateTime;

use crate::BoxType;
use crate::BoxView;
use crate::FullBoxFlags;
use crate::FullBoxHeader;
use crate::error::*;

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
        let full_box_header = FullBoxHeader::<MdhdSpec>::parse_in(cur)?;

        let (creation_time, modification_time, timescale, duration) =
            match full_box_header.version() {
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
            version: full_box_header.version(),
            flags: full_box_header.flags(),
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
}

impl TryFrom<&[u8]> for MdhdBox {
    type Error = Error;

    fn try_from(payload: &[u8]) -> Result<Self> {
        MdhdBox::parse(payload)
    }
}

impl TryFrom<&BoxView<'_>> for MdhdBox {
    type Error = Error;

    fn try_from(box_view: &BoxView<'_>) -> Result<Self> {
        if box_view.header.boxtype() != BoxType::MDHD {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::MDHD,
                found: box_view.header.boxtype(),
            }));
        }

        MdhdBox::parse(box_view.payload)
    }
}

/// Specification for Media Header Box (`mdhd`).
pub struct MdhdSpec;

/// Flags for Media Header Box (`mdhd`).
pub type MdhdFlags = FullBoxFlags<MdhdSpec>;

#[cfg(test)]
mod tests {
    use super::*;

    fn make_v0_payload(
        creation_time: u32,
        modification_time: u32,
        timescale: u32,
        duration: u32,
        language: LanguageCode,
    ) -> Vec<u8> {
        let mut data = Vec::new();

        // FullBoxHeader: version=0, flags=0
        data.push(0);
        data.extend_from_slice(&[0, 0, 0]);

        // creation_time (u32)
        data.extend_from_slice(&creation_time.to_be_bytes());
        // modification_time (u32)
        data.extend_from_slice(&modification_time.to_be_bytes());
        // timescale (u32)
        data.extend_from_slice(&timescale.to_be_bytes());
        // duration (u32)
        data.extend_from_slice(&duration.to_be_bytes());

        // language (u16)
        data.extend_from_slice(&language.to_packed().to_be_bytes());

        // pre_defined (u16)
        data.extend_from_slice(&[0, 0]);

        data
    }

    fn make_v1_payload(
        creation_time: u64,
        modification_time: u64,
        timescale: u32,
        duration: u64,
        language: LanguageCode,
    ) -> Vec<u8> {
        let mut data = Vec::new();

        // FullBoxHeader: version=1, flags=0
        data.push(1);
        data.extend_from_slice(&[0, 0, 0]);

        // creation_time (u64)
        data.extend_from_slice(&creation_time.to_be_bytes());
        // modification_time (u64)
        data.extend_from_slice(&modification_time.to_be_bytes());
        // timescale (u32)
        data.extend_from_slice(&timescale.to_be_bytes());
        // duration (u64)
        data.extend_from_slice(&duration.to_be_bytes());

        // language (u16)
        data.extend_from_slice(&language.to_packed().to_be_bytes());

        // pre_defined (u16)
        data.extend_from_slice(&[0, 0]);

        data
    }

    #[test]
    fn parse_mdhd_v0() {
        let lang_eng = LanguageCode::new(*b"eng");
        let payload = make_v0_payload(0x12345678, 0x23456789, 44100, 88200, lang_eng);
        let mdhd = MdhdBox::parse(&payload).unwrap();

        assert_eq!(mdhd.version, 0);
        assert_eq!(mdhd.creation_time.to_quicktime_seconds(), 0x12345678);
        assert_eq!(mdhd.modification_time.to_quicktime_seconds(), 0x23456789);
        assert_eq!(mdhd.timescale, 44100);
        assert_eq!(mdhd.duration, 88200);
        assert_eq!(mdhd.language, LanguageCode::new(*b"eng"));
    }

    #[test]
    fn parse_mdhd_v1() {
        let lang_jpn = LanguageCode::new(*b"jpn");
        let payload = make_v1_payload(
            0x0000000112345678,
            0x0000000123456789,
            48000,
            0x0000000200000000,
            lang_jpn,
        );
        let mdhd = MdhdBox::parse(&payload).unwrap();

        assert_eq!(mdhd.version, 1);
        assert_eq!(
            mdhd.creation_time.to_quicktime_seconds(),
            0x0000000112345678
        );
        assert_eq!(mdhd.timescale, 48000);
        assert_eq!(mdhd.duration, 0x0000000200000000);
        assert_eq!(mdhd.language, LanguageCode::new(*b"jpn"));
    }

    #[test]
    fn parse_mdhd_undetermined_language() {
        let payload = make_v0_payload(0, 0, 1000, 0, LanguageCode::UNDETERMINED);
        let mdhd = MdhdBox::parse(&payload).unwrap();

        assert_eq!(mdhd.language, LanguageCode::UNDETERMINED);
    }

    #[test]
    fn language_code_pack_unpack() {
        let eng = LanguageCode::new(*b"eng");
        assert_eq!(eng.to_packed(), 0x15C7);
        assert_eq!(LanguageCode::from_packed(0x15C7), Some(eng));

        let und = LanguageCode::new(*b"und");
        assert_eq!(und.to_packed(), 0x55C4);
        assert_eq!(LanguageCode::from_packed(0x55C4), Some(und));

        let jpn = LanguageCode::new(*b"jpn");
        assert_eq!(jpn.to_packed(), 0x2A0E);
        assert_eq!(LanguageCode::from_packed(0x2A0E), Some(jpn));
    }

    #[test]
    fn language_code_from_packed_invalid() {
        // Invalid packed values (characters outside 'a'..='z')
        assert_eq!(LanguageCode::from_packed(0x0000), None); // all zeros -> '`'
        assert_eq!(LanguageCode::from_packed(0x7FFF), None); // all ones -> out of range
    }

    #[test]
    fn parse_mdhd_extra_data() {
        let mut payload = make_v0_payload(0, 0, 1000, 0, LanguageCode::UNDETERMINED);
        payload.extend_from_slice(&[0xFF, 0xFF]); // Extra data

        let result = MdhdBox::parse(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn parse_mdhd_truncated() {
        let payload = make_v0_payload(0, 0, 1000, 0, LanguageCode::UNDETERMINED);
        let truncated = &payload[..payload.len() - 1];

        let result = MdhdBox::parse(truncated);
        assert!(result.is_err());
    }

    #[test]
    fn mdhd_default() {
        let mdhd = MdhdBox::default();

        assert_eq!(mdhd.version, 0);
        assert_eq!(mdhd.timescale, 0);
        assert_eq!(mdhd.duration, 0);
        assert_eq!(mdhd.language, LanguageCode::UNDETERMINED);
    }
}
