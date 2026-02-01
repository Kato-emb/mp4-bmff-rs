use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;

use crate::cursor::ReadCursor;

define_box_flags!(
    /// Flags for Extended language tag Box (`elng`).
    ElngFlags {}
);

/// A reference to an Extended language tag Box (`elng`).
#[derive(Debug)]
pub struct ElngBoxView<'a> {
    /// Box version (should be 0).
    pub version: u8,
    /// Box flags (should be 0).
    pub flags: ElngFlags,
    /// Extended language tag (RFC 4646).
    pub extended_language: &'a str,
}

impl BoxCodec for ElngBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::ELNG
    }
}

impl<'de> BoxDecode<'de> for ElngBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = ElngFlags::from_be_bytes(cur.read_array::<3>()?);

        let extended_language_bytes = cur.take_until(0)?;

        let extended_language = core::str::from_utf8(extended_language_bytes).map_err(|_| {
            Error::in_box(
                ErrorKind::InvalidBoxField {
                    field: "extended_language",
                    reason: "invalid UTF-8",
                },
                BoxType::ELNG,
            )
        })?;

        Ok(ElngBoxView {
            version,
            flags,
            extended_language,
        })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::{
        String, //
        ToString,
    };

    use super::*;
    use crate::BoxEncode;

    use crate::cursor::WriteCursor;

    /// An owned Extended language tag Box (`elng`).
    #[derive(Debug, Clone)]
    pub struct ElngBox {
        /// Box version (should be 0).
        pub version: u8,
        /// Box flags (should be 0).
        pub flags: ElngFlags,
        /// Extended language tag (RFC 4646).
        pub extended_language: String,
    }

    impl From<&ElngBoxView<'_>> for ElngBox {
        fn from(view: &ElngBoxView) -> Self {
            ElngBox {
                version: view.version,
                flags: view.flags,
                extended_language: view.extended_language.to_string(),
            }
        }
    }

    impl ElngBoxView<'_> {
        /// Converts this view into an owned `ElngBox`.
        pub fn to_owned(&self) -> ElngBox {
            ElngBox::from(self)
        }
    }

    impl BoxCodec for ElngBox {
        fn boxtype(&self) -> BoxType {
            BoxType::ELNG
        }
    }

    impl BoxDecode<'_> for ElngBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = ElngBoxView::decode(bytes)?;
            Ok(ElngBox::from(&view))
        }
    }

    impl BoxEncode for ElngBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            4 + self.extended_language.len() + 1 // version(1) + flags(3) + extended_language + null(1)
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_be_bytes())?;
            cur.write_slice(self.extended_language.as_bytes())?;
            cur.write_u8(0)?; // null-terminator

            Ok(cur.position())
        }
    }
}

#[cfg(feature = "alloc")]
pub use owned::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_data() -> [u8; 10] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            b'e', b'n', b'-', b'U', b'S', // extended_language = "en-US"
            0x00, // null terminator
        ]
    }

    #[test]
    fn test_elng_box_view_decode() {
        let data = raw_data();
        let elng = ElngBoxView::decode(&data).unwrap();

        assert_eq!(elng.version, 0);
        assert_eq!(elng.flags.bits(), 0);
        assert_eq!(elng.extended_language, "en-US");
    }

    #[test]
    fn test_elng_box_view_empty_language() {
        let data: [u8; 5] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, // empty extended_language with null terminator
        ];

        let elng = ElngBoxView::decode(&data).unwrap();
        assert_eq!(elng.version, 0);
        assert_eq!(elng.extended_language, "");
    }

    #[test]
    fn test_elng_box_view_truncated() {
        let data: [u8; 2] = [0x00, 0x00]; // too short

        let result = ElngBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_elng_box_view_invalid_utf8() {
        let data: [u8; 7] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            0xFF, 0xFE, // invalid UTF-8
            0x00, // null terminator
        ];

        let result = ElngBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_elng_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_data();
        let elng = ElngBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; elng.encoded_len()];
        elng.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_elng_box_to_owned() {
        let data = raw_data();
        let view = ElngBoxView::decode(&data).unwrap();
        let owned = view.to_owned();

        assert_eq!(owned.version, view.version);
        assert_eq!(owned.flags.bits(), view.flags.bits());
        assert_eq!(owned.extended_language, view.extended_language);
    }
}
