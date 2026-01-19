use core::fmt;
use core::slice;

/// An ISO-639-2/T language code represented as 3 ASCII bytes.
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
