//! Box type helpers shared across the Core BMFF boxes.

use core::error;
use core::fmt;

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

    /// Returns the 4-byte `type` field stored in the box header.
    pub fn type_field(&self) -> FourCC {
        self.boxtype
    }

    /// Returns `true` when this `BoxType` stores a UUID extension.
    pub fn is_uuid(&self) -> bool {
        self.boxtype == typecode::UUID
    }

    /// Returns the UUID extension if the type is `uuid`.
    pub fn user_type(&self) -> Option<Uuid> {
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

impl AsRef<FourCC> for BoxType {
    fn as_ref(&self) -> &FourCC {
        &self.boxtype
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

    /// `type` field value used by File Type boxes.
    pub const FTYP: FourCC = FourCC::new(*b"ftyp");
    /// `type` field value used by Free boxes.
    pub const FREE: FourCC = FourCC::new(*b"free");
    /// `type` field value used by Media Data boxes.
    pub const MDAT: FourCC = FourCC::new(*b"mdat");
    /// `type` field value used by Movie boxes.
    pub const MOOV: FourCC = FourCC::new(*b"moov");
    /// `type` field value used by Movie Header boxes.
    pub const MVHD: FourCC = FourCC::new(*b"mvhd");
    /// `type` field value used by Track boxes.
    pub const TRAK: FourCC = FourCC::new(*b"trak");
    /// `type` field value used by Track Header boxes.
    pub const TKHD: FourCC = FourCC::new(*b"tkhd");
    /// `type` field value used by Data Reference boxes.
    pub const DREF: FourCC = FourCC::new(*b"dref");
    /// `type` field value used by URL boxes.
    pub const URL_: FourCC = FourCC::new(*b"url ");
    /// `type` field value used by URN boxes.
    pub const URN_: FourCC = FourCC::new(*b"urn ");
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
