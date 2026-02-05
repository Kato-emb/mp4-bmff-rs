//! ISO-639-2/T language code type.
//!
//! BMFF uses ISO-639-2/T three-letter language codes to identify the
//! language of media tracks. These codes are stored in a packed 16-bit
//! format using 5 bits per character.
//!
//! # Packed Format
//!
//! Each character is encoded as its ASCII value minus 0x60 (so 'a' = 1),
//! packed into a 16-bit value:
//!
//! ```text
//! Bit:  15 14 13 12 11 | 10  9  8  7  6 |  5  4  3  2  1  0
//!       [ pad (1 bit) ] [ char 1 (5b)  ] [ char 2 (5b)   ] [ char 3 (5b)   ]
//! ```
//!
//! # Common Codes
//!
//! - `und`: Undetermined
//! - `eng`: English
//! - `jpn`: Japanese
//! - `fra`/`fre`: French
//! - `deu`/`ger`: German

use core::fmt;
use core::slice;

/// ISO-639-2/T language code (3 lowercase ASCII letters).
///
/// Represents a language using the ISO-639-2/T standard three-letter codes.
/// The code is stored as 3 ASCII bytes internally but can be converted to/from
/// the packed 16-bit format used in BMFF media headers.
///
/// # Example
///
/// ```
/// use mp4_bmff::types::LanguageCode;
///
/// // Create from bytes
/// let english = LanguageCode::new(*b"eng");
/// assert_eq!(english.as_str(), Some("eng"));
///
/// // Use the undetermined constant
/// let unknown = LanguageCode::UNDETERMINED;
/// assert_eq!(unknown.as_str(), Some("und"));
///
/// // Convert to/from packed format
/// let packed = english.to_packed();
/// let decoded = LanguageCode::from_packed(packed).unwrap();
/// assert_eq!(decoded, english);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct LanguageCode(pub(crate) [u8; 3]);

impl LanguageCode {
    /// Undetermined language code ("und").
    pub const UNDETERMINED: Self = Self([b'u', b'n', b'd']);

    /// Creates a new `LanguageCode` from a 3-byte array.
    pub const fn new(code: [u8; 3]) -> Self {
        Self(code)
    }

    /// Decodes a packed ISO-639-2/T language code (5 bits per character).
    ///
    /// Returns `None` if any character is outside the valid range ('a'..='z').
    pub const fn from_packed(packed: u16) -> Option<Self> {
        let c1 = ((packed >> 10) & 0x1F) as u8 + 0x60;
        let c2 = ((packed >> 5) & 0x1F) as u8 + 0x60;
        let c3 = (packed & 0x1F) as u8 + 0x60;

        if c1 >= b'a' && c1 <= b'z' && c2 >= b'a' && c2 <= b'z' && c3 >= b'a' && c3 <= b'z' {
            Some(Self([c1, c2, c3]))
        } else {
            None
        }
    }

    /// Encodes to a packed ISO-639-2/T language code (5 bits per character).
    pub const fn to_packed(&self) -> u16 {
        ((self.0[0] - 0x60) as u16) << 10
            | ((self.0[1] - 0x60) as u16) << 5
            | (self.0[2] - 0x60) as u16
    }

    /// Returns the language code as a 3-byte array.
    pub const fn as_bytes(&self) -> &[u8; 3] {
        &self.0
    }

    /// Returns the language code as a string slice if it's valid UTF-8.
    pub fn as_str(&self) -> Option<&str> {
        core::str::from_utf8(&self.0).ok()
    }

    fn fmt_escaped(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for &b in &self.0 {
            if (0x20..=0x7E).contains(&b) && b != b'\\' && b != b'"' {
                f.write_str(unsafe { core::str::from_utf8_unchecked(slice::from_ref(&b)) })?;
            } else {
                write!(f, "\\x{b:02X}")?;
            }
        }
        Ok(())
    }
}

impl fmt::Debug for LanguageCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LanguageCode(\"")?;
        self.fmt_escaped(f)?;
        f.write_str("\" bytes=[")?;

        for (i, b) in self.0.iter().enumerate() {
            if i != 0 {
                f.write_str(" ")?;
            }
            write!(f, "{:02x}", b)?;
        }

        f.write_str("])")
    }
}

impl fmt::Display for LanguageCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_escaped(f)
    }
}

impl From<[u8; 3]> for LanguageCode {
    fn from(value: [u8; 3]) -> Self {
        Self::new(value)
    }
}

impl From<LanguageCode> for [u8; 3] {
    fn from(value: LanguageCode) -> Self {
        value.0
    }
}

impl AsRef<[u8; 3]> for LanguageCode {
    fn as_ref(&self) -> &[u8; 3] {
        &self.0
    }
}
