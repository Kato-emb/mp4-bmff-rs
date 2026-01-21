use core::mem;

use crate::cursor::ReadCursor;
use crate::types::FourCC;

use crate::RawBoxRef;
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
    pub(crate) const PRE_DEFINED_SIZE: usize = mem::size_of::<u32>();
    pub(crate) const RESERVED_SIZE: usize = 3 * mem::size_of::<u32>();

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<HdlrBoxView<'a>> {
        let version = cur.read_u8()?;
        let flags = HdlrFlags::from_bytes(cur.read_array()?);

        // Skip pre_defined (4 bytes, should be 0)
        cur.advance(Self::PRE_DEFINED_SIZE)?;

        let handler_type_bytes = cur.read_array::<4>()?;
        let handler_type = FourCC::new(handler_type_bytes);

        // Skip reserved (12 bytes)
        cur.advance(Self::RESERVED_SIZE)?;

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

impl<'a> TryFrom<RawBoxRef<'a>> for HdlrBoxView<'a> {
    type Error = Error;

    fn try_from(value: RawBoxRef<'a>) -> Result<Self> {
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

    use crate::cursor::WriteCursor;

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

        /// Returns the size of the `HdlrBox` data.
        pub fn size(&self) -> usize {
            1  // version
            + 3  // flags
            + HdlrBoxView::<'_>::PRE_DEFINED_SIZE  // pre_defined
            + 4  // handler_type
            + HdlrBoxView::<'_>::RESERVED_SIZE  // reserved
            + self.name.len()  // name
            + 1 // null terminator
        }

        pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            // Write version (1 byte)
            cur.write_u8(self.version)?;

            // Write flags (3 bytes)
            cur.write_slice(&self.flags.to_bytes())?;

            // Write pre_defined (4 bytes, should be 0)
            cur.write_slice(&[0u8; HdlrBoxView::<'_>::PRE_DEFINED_SIZE])?;

            // Write handler_type (4 bytes)
            cur.write_array(self.handler_type.as_bytes())?;

            // Write reserved (12 bytes)
            cur.write_slice(&[0u8; HdlrBoxView::<'_>::RESERVED_SIZE])?;

            // Write name (null-terminated string)
            cur.write_slice(self.name.as_bytes())?;
            cur.write_u8(0)?;

            if !cur.is_empty() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxSize {
                        reason: "Buffer larger than expected",
                        got: cur.remaining() as u64,
                    },
                    BoxType::HDLR,
                ));
            }

            Ok(())
        }

        /// Writes this `HdlrBox` into the given payload.
        pub fn write(&self, payload: &mut [u8]) -> Result<()> {
            let mut cursor = WriteCursor::new(payload);
            self.write_in(&mut cursor)
        }
    }

    impl From<&HdlrBoxView<'_>> for HdlrBox {
        fn from(view: &HdlrBoxView<'_>) -> Self {
            HdlrBox::from_view(view)
        }
    }

    impl TryFrom<RawBoxRef<'_>> for HdlrBox {
        type Error = Error;

        fn try_from(value: RawBoxRef<'_>) -> Result<Self> {
            let view = HdlrBoxView::try_from(value)?;
            Ok(HdlrBox::from_view(&view))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "alloc")]
    #[test]
    fn hdlr_box_round_trip() {
        let original = HdlrBox {
            version: 0,
            flags: HdlrFlags::empty(),
            handler_type: FourCC::new(*b"vide"),
            name: String::from("VideoHandler"),
        };

        // Write
        let mut buf = vec![0u8; original.size()];
        original.write(&mut buf).unwrap();

        // Parse
        let reparsed = HdlrBox::parse(&buf).unwrap();

        assert_eq!(reparsed.version, original.version);
        assert_eq!(reparsed.flags.get(), original.flags.get());
        assert_eq!(reparsed.handler_type, original.handler_type);
        assert_eq!(reparsed.name, original.name);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn hdlr_box_round_trip_empty_name() {
        let original = HdlrBox {
            version: 0,
            flags: HdlrFlags::empty(),
            handler_type: FourCC::new(*b"soun"),
            name: String::new(),
        };

        // Write
        let mut buf = vec![0u8; original.size()];
        original.write(&mut buf).unwrap();

        // Parse
        let reparsed = HdlrBox::parse(&buf).unwrap();

        assert_eq!(reparsed.handler_type, original.handler_type);
        assert_eq!(reparsed.name, original.name);
    }
}
