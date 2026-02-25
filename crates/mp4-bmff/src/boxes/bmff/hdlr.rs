//! Handler Reference Box (`hdlr`) implementation.
//!
//! The Handler Reference Box declares the process by which media data within
//! a track is presented. It identifies the media handler component that is
//! appropriate for interpreting the track's media data.

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::types::*;

use crate::cursor::ReadCursor;

define_box_flags!(
    /// Flags for Handler Reference Box (`hdlr`).
    ///
    /// Reserved (should be 0).
    HdlrFlags {}
);

/// A reference to a Handler Reference Box (`hdlr`).
///
/// The Handler Reference Box is required within the Media Box (`mdia`) and
/// declares the type of media data within the track. Decoders use this box
/// to identify which handler to use for processing the track's media.
///
/// # Common Handler Types
///
/// - `vide`: Video track - contains video samples.
/// - `soun`: Audio track - contains audio samples.
/// - `hint`: Hint track - contains streaming hints.
/// - `meta`: Timed metadata track.
/// - `text`: Text track (subtitles).
/// - `subt`: Subtitle track.
///
/// # Structure
///
/// - `version`: Box version (should be 0).
/// - `flags`: Reserved (should be 0).
/// - `handler_type`: FourCC identifying the media type.
/// - `name`: Human-readable name for the handler (null-terminated UTF-8).
#[derive(Debug)]
pub struct HdlrBoxView<'a> {
    /// Box version (should be 0).
    pub version: u8,
    /// Box flags (should be 0).
    pub flags: HdlrFlags,
    /// Handler type (e.g., 'vide' for video, 'soun' for sound).
    pub handler_type: FourCC,
    /// Human-readable name for the track type.
    pub name: &'a str,
}

const PRE_DEFINED: usize = 4;
const RESERVED: usize = 3 * 4;

impl BoxCodec for HdlrBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::HDLR
    }
}

impl<'de> BoxDecode<'de> for HdlrBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = HdlrFlags::from_be_bytes(cur.read_array::<3>()?);

        // Skip pre_defined (4 bytes)
        cur.advance(PRE_DEFINED)?;

        let handler_type_bytes = cur.read_array::<4>()?;
        let handler_type = FourCC::new(handler_type_bytes);

        // Skip reserved (12 bytes)
        cur.advance(RESERVED)?;

        // The remaining bytes are the name (null-terminated string)
        let name_bytes = cur.take_until(0)?;

        let name = core::str::from_utf8(name_bytes).map_err(|_| {
            Error::in_box(
                ErrorKind::InvalidBoxField {
                    field: "name",
                    reason: "invalid UTF-8",
                },
                BoxType::HDLR,
            )
        })?;

        Ok(HdlrBoxView {
            version,
            flags,
            handler_type,
            name,
        })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use alloc::string::{
        String, //
        ToString,
    };

    use super::*;
    use crate::BoxEncode;

    use crate::cursor::WriteCursor;

    /// An owned Handler Reference Box (`hdlr`).
    ///
    /// This is the owned variant of [`HdlrBoxView`] that stores the handler
    /// name in a heap-allocated string.
    ///
    /// # Structure
    ///
    /// - `version`: Box version (should be 0).
    /// - `flags`: Reserved flags (should be 0).
    /// - `handler_type`: FourCC identifying the media handler type.
    /// - `name`: Human-readable name describing the handler.
    ///
    /// # Example
    ///
    /// ```
    /// use mp4_bmff::BoxDecode;
    /// use mp4_bmff::boxes::bmff::HdlrBox;
    /// use mp4_bmff::types::FourCC;
    ///
    /// // A handler box for video
    /// let data: [u8; 30] = [
    ///     0x00,                               // version = 0
    ///     0x00, 0x00, 0x00,                   // flags
    ///     0x00, 0x00, 0x00, 0x00,             // pre_defined
    ///     b'v', b'i', b'd', b'e',             // handler_type = "vide"
    ///     0x00, 0x00, 0x00, 0x00,             // reserved[0]
    ///     0x00, 0x00, 0x00, 0x00,             // reserved[1]
    ///     0x00, 0x00, 0x00, 0x00,             // reserved[2]
    ///     b'V', b'i', b'd', b'e', b'o', 0x00, // name = "Video\0"
    /// ];
    ///
    /// let hdlr = HdlrBox::decode(&data).unwrap();
    /// assert_eq!(hdlr.handler_type, FourCC::new(*b"vide"));
    /// assert_eq!(hdlr.name, "Video");
    /// ```
    #[derive(Debug, Clone)]
    pub struct HdlrBox {
        /// Box version (should be 0).
        pub version: u8,
        /// Reserved flags (should be 0).
        pub flags: HdlrFlags,
        /// Handler type FourCC (e.g., "vide" for video, "soun" for audio).
        pub handler_type: FourCC,
        /// Human-readable name describing this handler.
        pub name: String,
    }

    impl From<&HdlrBoxView<'_>> for HdlrBox {
        fn from(view: &HdlrBoxView<'_>) -> Self {
            HdlrBox {
                version: view.version,
                flags: view.flags,
                handler_type: view.handler_type,
                name: view.name.to_string(),
            }
        }
    }

    impl HdlrBoxView<'_> {
        /// Converts this view into an owned `HdlrBox`.
        pub fn to_owned(&self) -> HdlrBox {
            HdlrBox::from(self)
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
        fn encoded_len(&self) -> usize {
            let mut len = 1 + 3; // version + flags
            len += PRE_DEFINED; // pre_defined
            len += 4; // handler_type
            len += RESERVED; // reserved
            len += self.name.len() + 1; // name + null terminator
            len
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            // Write version (1 byte)
            cur.write_u8(self.version)?;
            // Write flags (3 bytes)
            cur.write_array(&self.flags.to_be_bytes())?;

            // Write pre_defined (4 bytes)
            cur.reserve_zeros(PRE_DEFINED)?;

            // Write handler_type (4 bytes)
            cur.write_array(self.handler_type.as_bytes())?;

            // Write reserved (12 bytes)
            cur.reserve_zeros(RESERVED)?;

            // Write name (null-terminated string)
            cur.write_slice(self.name.as_bytes())?;
            cur.write_u8(0)?; // null terminator

            Ok(cur.position())
        }
    }
}

#[cfg(feature = "alloc")]
pub use owned::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_data() -> [u8; 37] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x00, 0x00, 0x00, // pre_defined (4 bytes)
            b'v', b'i', b'd', b'e', // handler_type = "vide"
            0x00, 0x00, 0x00, 0x00, // reserved[0] (4 bytes)
            0x00, 0x00, 0x00, 0x00, // reserved[1] (4 bytes)
            0x00, 0x00, 0x00, 0x00, // reserved[2] (4 bytes)
            b'V', b'i', b'd', b'e', b'o', b'H', b'a', b'n', b'd', b'l', b'e',
            b'r', // name = "VideoHandler"
            0x00, // null terminator
        ]
    }

    #[test]
    fn test_hdlr_box_view_decode() {
        let data = raw_data();
        let hdlr = HdlrBoxView::decode(&data).unwrap();

        assert_eq!(hdlr.version, 0);
        assert_eq!(hdlr.flags.bits(), 0);
        assert_eq!(hdlr.handler_type, FourCC::new(*b"vide"));
        assert_eq!(hdlr.name, "VideoHandler");
    }

    #[test]
    fn test_hdlr_box_view_empty_name() {
        let data: [u8; 25] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x00, 0x00, 0x00, // pre_defined (4 bytes)
            b'v', b'i', b'd', b'e', // handler_type = "vide"
            0x00, 0x00, 0x00, 0x00, // reserved[0] (4 bytes)
            0x00, 0x00, 0x00, 0x00, // reserved[1] (4 bytes)
            0x00, 0x00, 0x00, 0x00, // reserved[2] (4 bytes)
            0x00, // empty name with null terminator
        ];

        let hdlr = HdlrBoxView::decode(&data).unwrap();
        assert_eq!(hdlr.version, 0);
        assert_eq!(hdlr.name, "");
    }

    #[test]
    fn test_hdlr_box_view_truncated() {
        let data: [u8; 10] = [0x00; 10]; // too short

        let result = HdlrBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_hdlr_box_view_invalid_utf8() {
        let mut data = raw_data();
        // Replace name with invalid UTF-8
        let name_start = 24; // offset where name begins
        data[name_start] = 0xFF;
        data[name_start + 1] = 0xFE;

        let result = HdlrBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_hdlr_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_data();
        let hdlr = HdlrBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; hdlr.encoded_len()];
        hdlr.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_hdlr_box_to_owned() {
        let data = raw_data();
        let view = HdlrBoxView::decode(&data).unwrap();
        let owned = view.to_owned();

        assert_eq!(owned.version, view.version);
        assert_eq!(owned.flags.bits(), view.flags.bits());
        assert_eq!(owned.handler_type, view.handler_type);
        assert_eq!(owned.name, view.name);
    }
}
