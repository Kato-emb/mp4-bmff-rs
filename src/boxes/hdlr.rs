use core::mem;

use crate::cursor::ReadCursor;
use crate::types::FourCC;

use crate::BoxFrame;
use crate::BoxType;
use crate::error::*;

use super::FullBoxFlags;

/// A reference to a Handler Reference Box (`hdlr`).
///
/// This box declares the media type (handler) of the track or the metadata type.
/// The handler type determines how the media data or metadata is to be processed.
#[derive(Debug, Clone, Copy)]
pub struct HdlrBoxView<'a> {
    /// Box version (should be 0).
    pub version: u8,
    /// Box flags (should be 0).
    pub flags: HdlrFlags,
    /// Handler type (e.g., 'vide' for video, 'soun' for sound, 'hint' for hint, 'meta' for metadata).
    pub handler_type: FourCC,
    /// Human-readable name for the track type.
    pub name: &'a str,
}

impl<'a> HdlrBoxView<'a> {
    const PRE_DEFINED_SIZE: usize = mem::size_of::<u32>();
    const RESERVED_SIZE: usize = 3 * mem::size_of::<u32>();

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<HdlrBoxView<'a>> {
        let version = cur
            .read_u8()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let flags = HdlrFlags::from_bytes(
            cur.read_array()
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
        );

        // Skip pre_defined (4 bytes, should be 0)
        cur.advance(Self::PRE_DEFINED_SIZE)
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        let handler_type_bytes = cur
            .read_array::<4>()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let handler_type = FourCC::new(handler_type_bytes);

        // Skip reserved (12 bytes)
        cur.advance(Self::RESERVED_SIZE)
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        // The rest is the name (null-terminated UTF-8 string)
        let name_bytes = cur.take(cur.remaining())?;

        // Remove trailing null terminator if present
        let name_bytes = if name_bytes.last() == Some(&0) {
            &name_bytes[..name_bytes.len() - 1]
        } else {
            name_bytes
        };

        let name = core::str::from_utf8(name_bytes).map_err(|_| {
            Error::new(ErrorKind::InvalidBoxField {
                field: "name",
                reason: "invalid UTF-8",
            })
            .with_box_type(BoxType::HDLR)
        })?;

        Ok(HdlrBoxView {
            version,
            flags,
            handler_type,
            name,
        })
    }

    /// Parses a `HdlrBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<HdlrBoxView<'a>> {
        let mut cursor = ReadCursor::new(payload);
        HdlrBoxView::parse_in(&mut cursor)
    }
}

impl<'a> TryFrom<BoxFrame<'a>> for HdlrBoxView<'a> {
    type Error = Error;

    fn try_from(value: BoxFrame<'a>) -> Result<Self> {
        if value.boxtype() != BoxType::HDLR {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::HDLR,
                found: value.boxtype(),
            }));
        }

        HdlrBoxView::parse(value.payload())
    }
}

/// Specification for Handler Reference Box (`hdlr`).
pub struct HdlrSpec;

/// Flags for Handler Reference Box (`hdlr`).
pub type HdlrFlags = FullBoxFlags<HdlrSpec>;

#[cfg(feature = "alloc")]
pub use owned::HdlrBox;

#[cfg(feature = "alloc")]
mod owned {
    extern crate alloc;
    use alloc::string::String;

    use super::*;

    /// An owned Handler Reference Box (`hdlr`).
    #[derive(Debug, Clone)]
    pub struct HdlrBox {
        /// Box version (should be 0).
        pub version: u8,
        /// Box flags (should be 0).
        pub flags: HdlrFlags,
        /// Handler type (e.g., 'vide' for video, 'soun' for sound).
        pub handler_type: FourCC,
        /// Human-readable name for the track type.
        pub name: String,
    }

    impl HdlrBox {
        /// Creates a `HdlrBox` from a `HdlrBoxView`.
        pub fn from_view(view: &HdlrBoxView<'_>) -> HdlrBox {
            HdlrBox {
                version: view.version,
                flags: view.flags,
                handler_type: view.handler_type,
                name: String::from(view.name),
            }
        }

        /// Parses a `HdlrBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<HdlrBox> {
            let view = HdlrBoxView::parse(payload)?;
            Ok(HdlrBox::from_view(&view))
        }
    }

    impl From<&HdlrBoxView<'_>> for HdlrBox {
        fn from(view: &HdlrBoxView<'_>) -> Self {
            HdlrBox::from_view(view)
        }
    }

    impl TryFrom<BoxFrame<'_>> for HdlrBox {
        type Error = Error;

        fn try_from(value: BoxFrame<'_>) -> Result<Self> {
            let view = HdlrBoxView::try_from(value)?;
            Ok(HdlrBox::from_view(&view))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hdlr_payload(handler_type: &[u8; 4], name: &[u8]) -> Vec<u8> {
        let mut data = Vec::new();

        // FullBoxHeader: version (1 byte) + flags (3 bytes)
        data.push(0);
        data.extend_from_slice(&[0, 0, 0]);

        // pre_defined (4 bytes)
        data.extend_from_slice(&[0, 0, 0, 0]);

        // handler_type (4 bytes)
        data.extend_from_slice(handler_type);

        // reserved (12 bytes)
        data.extend_from_slice(&[0u8; 12]);

        // name (null-terminated string)
        data.extend_from_slice(name);

        data
    }

    #[test]
    fn parse_hdlr_video() {
        let payload = make_hdlr_payload(b"vide", b"VideoHandler\0");
        let hdlr = HdlrBoxView::parse(&payload).unwrap();

        assert_eq!(hdlr.version, 0);
        assert_eq!(hdlr.flags.get(), 0);
        assert_eq!(hdlr.handler_type, FourCC::new(*b"vide"));
        assert_eq!(hdlr.name, "VideoHandler");
    }

    #[test]
    fn parse_hdlr_sound() {
        let payload = make_hdlr_payload(b"soun", b"SoundHandler\0");
        let hdlr = HdlrBoxView::parse(&payload).unwrap();

        assert_eq!(hdlr.handler_type, FourCC::new(*b"soun"));
        assert_eq!(hdlr.name, "SoundHandler");
    }

    #[test]
    fn parse_hdlr_empty_name() {
        let payload = make_hdlr_payload(b"vide", b"\0");
        let hdlr = HdlrBoxView::parse(&payload).unwrap();

        assert_eq!(hdlr.handler_type, FourCC::new(*b"vide"));
        assert_eq!(hdlr.name, "");
    }

    #[test]
    fn parse_hdlr_no_null_terminator() {
        // Some encoders don't include null terminator
        let payload = make_hdlr_payload(b"vide", b"VideoHandler");
        let hdlr = HdlrBoxView::parse(&payload).unwrap();

        assert_eq!(hdlr.name, "VideoHandler");
    }

    #[test]
    fn parse_hdlr_truncated() {
        let payload = make_hdlr_payload(b"vide", b"");
        let truncated = &payload[..10]; // Too short

        let result = HdlrBoxView::parse(truncated);
        assert!(result.is_err());
    }

    #[test]
    fn parse_hdlr_invalid_utf8() {
        let payload = make_hdlr_payload(b"vide", &[0xFF, 0xFE, 0x00]); // Invalid UTF-8

        let result = HdlrBoxView::parse(&payload);
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn parse_hdlr_owned() {
        let payload = make_hdlr_payload(b"vide", b"VideoHandler\0");
        let hdlr = HdlrBox::parse(&payload).unwrap();

        assert_eq!(hdlr.handler_type, FourCC::new(*b"vide"));
        assert_eq!(hdlr.name, "VideoHandler");
    }
}
