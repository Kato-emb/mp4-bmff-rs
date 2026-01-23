use core::mem;

use crate::cursor::ReadCursor;
use crate::types::FourCC;

use crate::BoxCodec;
use crate::BoxDecode;
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
}

/// Specification for Handler Reference Box (`hdlr`).
pub struct HdlrSpec;

/// Flags for Handler Reference Box (`hdlr`).
pub type HdlrFlags = FullBoxFlags<HdlrSpec>;

impl BoxCodec for HdlrBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::HDLR
    }
}

impl<'de> BoxDecode<'de> for HdlrBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

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
}

impl<'a> TryFrom<&'a [u8]> for HdlrBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        HdlrBoxView::decode(value)
    }
}

#[cfg(feature = "alloc")]
pub use owned::HdlrBox;

#[cfg(feature = "alloc")]
mod owned {
    extern crate alloc;
    use alloc::string::String;

    use crate::BoxCodec;
    use crate::BoxDecode;
    use crate::BoxEncode;
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

    impl From<&HdlrBoxView<'_>> for HdlrBox {
        fn from(view: &HdlrBoxView) -> Self {
            HdlrBox {
                version: view.version,
                flags: view.flags,
                handler_type: view.handler_type,
                name: String::from(view.name),
            }
        }
    }

    impl BoxCodec for HdlrBox {
        fn boxtype(&self) -> BoxType {
            BoxType::HDLR
        }
    }

    impl BoxDecode<'_> for HdlrBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = HdlrBoxView::decode(bytes)?;
            Ok(HdlrBox::from(&view))
        }
    }

    impl BoxEncode for HdlrBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            let base_len = 4 // version(1) + flags(3)
                + HdlrBoxView::PRE_DEFINED_SIZE // pre_defined(4)
                + 4 // handler_type(4)
                + HdlrBoxView::RESERVED_SIZE; // reserved(12)

            let name_len = self.name.len() + 1; // name + null terminator

            base_len + name_len
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

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

            Ok(cur.position())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "alloc")]
    #[test]
    fn hdlr_box_round_trip() {
        use crate::BoxEncode;

        let original = HdlrBox {
            version: 0,
            flags: HdlrFlags::empty(),
            handler_type: FourCC::new(*b"vide"),
            name: String::from("VideoHandler"),
        };

        // Write
        let mut buf = vec![0u8; 256];
        let written = original.encode_into(&mut buf).unwrap();

        // Parse
        let reparsed = HdlrBox::decode(&buf[..written]).unwrap();

        assert_eq!(reparsed.version, original.version);
        assert_eq!(reparsed.flags.get(), original.flags.get());
        assert_eq!(reparsed.handler_type, original.handler_type);
        assert_eq!(reparsed.name, original.name);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn hdlr_box_round_trip_empty_name() {
        use crate::BoxEncode;

        let original = HdlrBox {
            version: 0,
            flags: HdlrFlags::empty(),
            handler_type: FourCC::new(*b"soun"),
            name: String::new(),
        };

        // Write
        let mut buf = vec![0u8; 256];
        let written = original.encode_into(&mut buf).unwrap();

        // Parse
        let reparsed = HdlrBox::decode(&buf[..written]).unwrap();
        assert_eq!(reparsed.handler_type, original.handler_type);
        assert_eq!(reparsed.name, original.name);
    }
}
