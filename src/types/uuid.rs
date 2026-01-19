//! UUID helper type used by `uuid` boxes and their user types.

use core::fmt;

/// 16-byte UUID used by extended `uuid` box types.
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
