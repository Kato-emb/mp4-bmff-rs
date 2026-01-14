//! Box type helpers shared across the Core BMFF boxes.

use core::error;
use core::fmt;

use crate::boxes::error::ErrorKind;
use crate::types::{
    FourCC, //
    Uuid,
};

/// Errors that can occur when creating or manipulating `BoxType` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BoxTypeError {
    /// UUID BoxType cannot be created from a FourCC code.
    UuidFourCCNotAllowed,
}

impl fmt::Display for BoxTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BoxTypeError::UuidFourCCNotAllowed => {
                write!(f, "Cannot create UUID BoxType from FourCC code 'uuid'")
            }
        }
    }
}

impl error::Error for BoxTypeError {}

impl From<BoxTypeError> for ErrorKind {
    fn from(value: BoxTypeError) -> Self {
        match value {
            BoxTypeError::UuidFourCCNotAllowed => Self::InvalidBoxType {
                reason: "Cannot create UUID BoxType from FourCC code 'uuid'",
                got: typecode::UUID,
            },
        }
    }
}

/// User extensions use an extended type
pub type UserType = Uuid;

/// Type-safe representation of BMFF `boxtype` values.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoxType {
    boxtype: FourCC,
    usertype: Option<UserType>,
}

impl BoxType {
    /// Creates a `BoxType` from a regular FourCC code.
    ///
    /// # Errors
    /// Returns [BoxTypeError::UuidFourCCNotAllowed] if the provided FourCC code is `uuid`.
    pub fn from_fourcc(fourcc: FourCC) -> Result<Self, BoxTypeError> {
        if fourcc == typecode::UUID {
            Err(BoxTypeError::UuidFourCCNotAllowed)
        } else {
            Ok(Self {
                boxtype: fourcc,
                usertype: None,
            })
        }
    }

    /// Creates a UUID-based `BoxType`.
    pub fn from_uuid(user_type: Uuid) -> Self {
        Self {
            boxtype: typecode::UUID,
            usertype: Some(user_type),
        }
    }

    #[inline]
    /// Returns the 4-byte `type` field stored in the box header.
    pub fn type_field(&self) -> FourCC {
        self.boxtype
    }

    #[inline]
    /// Returns `true` when this `BoxType` stores a UUID extension.
    pub fn is_uuid(&self) -> bool {
        self.boxtype == typecode::UUID
    }

    #[inline]
    /// Returns the UUID extension if the type is `uuid`.
    pub fn user_type(&self) -> Option<Uuid> {
        self.usertype
    }
}

impl fmt::Debug for BoxType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ds = f.debug_struct("BoxType");
        ds.field("boxtype", &self.type_field());

        if let Some(uuid) = self.user_type() {
            ds.field("usertype", &uuid);
        }

        ds.finish()
    }
}

impl fmt::Display for BoxType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.type_field())?;

        if let Some(uuid) = self.user_type() {
            write!(f, "({})", uuid)?;
        }

        Ok(())
    }
}

impl TryFrom<FourCC> for BoxType {
    type Error = BoxTypeError;

    fn try_from(value: FourCC) -> Result<Self, Self::Error> {
        Self::from_fourcc(value)
    }
}

impl From<Uuid> for BoxType {
    fn from(value: Uuid) -> Self {
        Self::from_uuid(value)
    }
}

pub use typecode::*;

mod typecode {
    use crate::types::FourCC;

    /// `type` field value used by UUID boxes.
    pub const UUID: FourCC = FourCC::new(*b"uuid");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_helpers_reflect_variant() {
        let fourcc_type = BoxType::from_fourcc(FourCC::from(*b"moov")).unwrap();
        assert_eq!(fourcc_type.type_field(), FourCC::from(*b"moov"));
        assert!(!fourcc_type.is_uuid());
        assert_eq!(fourcc_type.user_type(), None);

        let uuid = Uuid::new([0x11; 16]);
        let uuid_type = BoxType::from_uuid(uuid);
        assert_eq!(uuid_type.type_field(), UUID);
        assert!(uuid_type.is_uuid());
        assert_eq!(uuid_type.user_type(), Some(uuid));
    }
}
