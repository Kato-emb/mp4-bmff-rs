//! UUID type for extended box types.
//!
//! When a box type is "uuid", an additional 16-byte UUID follows the
//! standard header to uniquely identify the box type. This allows vendors
//! to define custom boxes without risk of FourCC collision.
//!
//! # UUID Format
//!
//! UUIDs are displayed in the standard hyphenated format:
//! `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`
//!
//! # Example
//!
//! ```
//! use mp4_bmff::types::Uuid;
//!
//! let uuid = Uuid::new([
//!     0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
//!     0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
//! ]);
//! assert_eq!(format!("{}", uuid), "12345678-9abc-def0-1234-56789abcdef0");
//! ```

use core::fmt;

/// 16-byte UUID for extended box type identification.
///
/// Used when the box type FourCC is "uuid" to provide a unique identifier
/// for vendor-specific or experimental box types.
///
/// # Structure
///
/// The UUID is stored as a 16-byte array in big-endian order, matching
/// the wire format in BMFF files.
///
/// # Display
///
/// The `Display` implementation outputs the standard hyphenated UUID format:
/// `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Uuid([u8; 16]);

impl Uuid {
    /// Creates a UUID from a byte array.
    pub const fn new(value: [u8; 16]) -> Self {
        Self(value)
    }

    #[inline]
    /// Returns the raw 16-byte array.
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    fn fmt_hyphenated(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, &b) in self.0.iter().enumerate() {
            if i == 4 || i == 6 || i == 8 || i == 10 {
                f.write_str("-")?;
            }

            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}

impl fmt::Debug for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Uuid(\"")?;
        self.fmt_hyphenated(f)?;
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

impl fmt::Display for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_hyphenated(f)
    }
}

impl From<[u8; 16]> for Uuid {
    fn from(value: [u8; 16]) -> Self {
        Self::new(value)
    }
}

impl From<Uuid> for [u8; 16] {
    fn from(value: Uuid) -> Self {
        value.0
    }
}

impl AsRef<[u8; 16]> for Uuid {
    fn as_ref(&self) -> &[u8; 16] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_accessors_and_formatting_follow_reference() {
        let uuid = Uuid::new([
            0x12, 0x34, 0x56, 0x78, //
            0x9A, 0xBC, //
            0xDE, 0xF0, //
            0x12, 0x34, //
            0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0,
        ]);
        assert_eq!(
            uuid.as_bytes(),
            &[
                0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC,
                0xDE, 0xF0
            ]
        );
        assert_eq!(format!("{uuid}"), "12345678-9abc-def0-1234-56789abcdef0");
        assert_eq!(
            format!("{uuid:?}"),
            "Uuid(\"12345678-9abc-def0-1234-56789abcdef0\" bytes=[12 34 56 78 9a bc de f0 12 34 56 78 9a bc de f0])"
        );
    }
}
