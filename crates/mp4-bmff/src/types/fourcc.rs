//! Four-character code (FourCC) type.
//!
//! FourCC is a 4-byte identifier used extensively in BMFF and related formats
//! to identify box types, brands, and codecs. Common examples include:
//!
//! - Box types: `ftyp`, `moov`, `mdat`, `trak`
//! - Brands: `isom`, `iso2`, `mp41`, `avc1`
//! - Handlers: `vide`, `soun`, `hint`
//!
//! # Display and Debug
//!
//! FourCC values display as ASCII when printable, with non-printable bytes
//! shown as `\xNN` escape sequences:
//!
//! ```
//! use mp4_bmff::types::FourCC;
//!
//! let ftyp = FourCC::new(*b"ftyp");
//! assert_eq!(format!("{}", ftyp), "ftyp");
//! ```

use core::fmt;
use core::slice;

/// 4-byte Four Character Code (FourCC) identifier.
///
/// FourCC codes identify box types, file brands, codec types, and other
/// entities in BMFF files. While often ASCII text, any 4-byte sequence
/// is valid.
///
/// # Example
///
/// ```
/// use mp4_bmff::types::FourCC;
///
/// let brand = FourCC::new(*b"isom");
/// assert_eq!(brand.as_bytes(), b"isom");
/// assert_eq!(brand.as_ascii(), Some("isom"));
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct FourCC([u8; 4]);

impl FourCC {
    /// Creates a FourCC from a byte array.
    pub const fn new(value: [u8; 4]) -> Self {
        Self(value)
    }

    #[inline]
    /// Returns the raw 4-byte array.
    pub const fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }

    #[inline]
    /// Interprets the code as ASCII and returns the string if it is valid.
    pub fn as_ascii(&self) -> Option<&str> {
        core::str::from_utf8(&self.0).ok()
    }

    fn fmt_escaped(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for &b in &self.0 {
            if (0x20..=0x7E).contains(&b) && b != b'\\' && b != b'"' {
                // SAFETY: byte is verified to be in printable ASCII range (0x20..=0x7E), which is valid UTF-8.
                f.write_str(unsafe { str::from_utf8_unchecked(slice::from_ref(&b)) })?;
            } else {
                write!(f, "\\x{b:02X}")?;
            }
        }
        Ok(())
    }
}

impl fmt::Debug for FourCC {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FourCC(\"")?;
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

impl fmt::Display for FourCC {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_escaped(f)
    }
}

impl From<[u8; 4]> for FourCC {
    fn from(value: [u8; 4]) -> Self {
        Self::new(value)
    }
}

impl From<FourCC> for [u8; 4] {
    fn from(value: FourCC) -> Self {
        value.0
    }
}

impl AsRef<[u8; 4]> for FourCC {
    fn as_ref(&self) -> &[u8; 4] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fourcc_accessors_and_ascii_validation() {
        let printable = FourCC::from(*b"moov");
        assert_eq!(printable.as_bytes(), b"moov");
        assert_eq!(printable.as_ascii(), Some("moov"));
        assert_eq!(
            format!("{printable:?}"),
            "FourCC(\"moov\" bytes=[6d 6f 6f 76])"
        );

        let non_utf8 = FourCC::new([0xFF, b'o', b'o', b'v']);
        assert_eq!(non_utf8.as_ascii(), None);
    }

    #[test]
    fn fourcc_display_escapes_non_printable_bytes() {
        let value = FourCC::new([0, b'B', b'\\', 0xFF]);
        assert_eq!(format!("{value}"), r"\x00B\x5C\xFF");
    }
}
