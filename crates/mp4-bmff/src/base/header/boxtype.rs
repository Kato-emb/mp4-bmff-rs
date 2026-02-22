//! BMFF box type representation.
//!
//! This module defines the [`BoxType`] type, which represents the type of a
//! BMFF box. Box types can be either:
//!
//! - **FourCC**: Standard 4-character code (e.g., "ftyp", "moov", "mdat").
//! - **UUID**: Extended type using a 16-byte UUID for custom boxes.
//!
//! # UUID Box Types
//!
//! When the FourCC is "uuid", an additional 16-byte UUID field follows
//! the type field in the box header. This allows vendors to define
//! custom box types without risk of collision.
//!
//! # Constants
//!
//! Standard box type constants are defined on `BoxType` via the
//! `define_box_types!` macro (e.g., `BoxType::FTYP`, `BoxType::MOOV`).

use core::fmt;

use crate::types::{
    FourCC, //
    Uuid,
};

/// User-defined box type extension (UUID).
///
/// Used when the FourCC type field is "uuid" to provide a 16-byte
/// unique identifier for custom box types.
pub type UserType = Uuid;

/// The FourCC code used for UUID-based box types.
pub(super) const UUID: FourCC = FourCC::new(*b"uuid");

/// Type-safe representation of BMFF box type values.
///
/// Box types identify the kind of data contained in a box. Most boxes
/// use a standard FourCC code, but custom boxes can use UUID-based types.
///
/// # Structure
///
/// - `boxtype`: The 4-byte FourCC type field (or "uuid" for extended types).
/// - `usertype`: Optional 16-byte UUID for extended type identification.
///
/// # Creating Box Types
///
/// ```
/// use mp4_bmff::BoxType;
/// use mp4_bmff::types::FourCC;
///
/// // Use predefined constants
/// let ftyp = BoxType::FTYP;
///
/// // Create from FourCC
/// let custom = BoxType::new(FourCC::new(*b"test"));
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoxType {
    pub(crate) boxtype: FourCC,
    pub(crate) usertype: Option<UserType>,
}

impl BoxType {
    /// Creates a `BoxType` from a regular FourCC code.
    ///
    /// # Panics
    ///
    /// Panics if the FourCC is "uuid". Use [`BoxType::with_usertype`] instead
    /// for UUID-based box types.
    pub fn new(fourcc: FourCC) -> Self {
        assert!(
            fourcc != UUID,
            "Use BoxType::from_uuid to create UUID-based BoxType"
        );

        Self {
            boxtype: fourcc,
            usertype: None,
        }
    }

    /// Creates a UUID-based `BoxType`.
    pub fn with_usertype(user_type: UserType) -> Self {
        Self {
            boxtype: UUID,
            usertype: Some(user_type),
        }
    }

    /// Returns the 4-byte `type` field stored in the box header.
    #[inline]
    pub const fn type_field(&self) -> FourCC {
        self.boxtype
    }

    /// Returns `true` when this `BoxType` stores a UUID extension.
    #[inline]
    pub const fn is_uuid(&self) -> bool {
        matches!(self.boxtype, UUID)
    }

    /// Returns the UUID extension if the type is `uuid`.
    #[inline]
    pub const fn user_type(&self) -> Option<Uuid> {
        self.usertype
    }
}

impl fmt::Debug for BoxType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ds = f.debug_struct("BoxType");
        ds.field("boxtype", &self.boxtype);

        if let Some(uuid) = self.user_type() {
            ds.field("usertype", &uuid);
        }

        ds.finish()
    }
}

impl fmt::Display for BoxType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.boxtype)?;

        if let Some(uuid) = self.user_type() {
            write!(f, "({})", uuid)?;
        }

        Ok(())
    }
}

impl From<FourCC> for BoxType {
    fn from(value: FourCC) -> Self {
        Self::new(value)
    }
}

impl From<Uuid> for BoxType {
    fn from(value: Uuid) -> Self {
        Self::with_usertype(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_fourcc_boxtype() {
        let boxtype = BoxType::new(FourCC::from(*b"moov"));

        assert_eq!(boxtype.type_field(), FourCC::from(*b"moov"));
        assert!(!boxtype.is_uuid());
        assert_eq!(boxtype.user_type(), None);
    }

    #[test]
    #[should_panic(expected = "Use BoxType::from_uuid")]
    fn new_panics_on_uuid_fourcc() {
        let _ = BoxType::new(UUID);
    }

    #[test]
    fn with_usertype_creates_uuid_boxtype() {
        let uuid = Uuid::new([0x11; 16]);
        let boxtype = BoxType::with_usertype(uuid);

        assert_eq!(boxtype.type_field(), UUID);
        assert!(boxtype.is_uuid());
        assert_eq!(boxtype.user_type(), Some(uuid));
    }

    #[test]
    fn constants_are_not_uuid() {
        assert!(!BoxType::FTYP.is_uuid());
        assert!(!BoxType::MOOV.is_uuid());
        assert!(!BoxType::MDAT.is_uuid());
    }
}
