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

/// The FourCC code used for UUID-based box types.
pub const UUID: FourCC = FourCC::new(*b"uuid");

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
        if fourcc == UUID {
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
            boxtype: UUID,
            usertype: Some(user_type),
        }
    }

    /// Returns the 4-byte `type` field stored in the box header.
    pub fn type_field(&self) -> FourCC {
        self.boxtype
    }

    /// Returns `true` when this `BoxType` stores a UUID extension.
    pub fn is_uuid(&self) -> bool {
        self.boxtype == UUID
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

/// Macro to define BoxType constants in a concise, table-like format.
macro_rules! define_box_types {
    ($(
        $(#[$meta:meta])*
        $name:ident = $fourcc:literal
    ),* $(,)?) => {
        impl BoxType {
            $(
                $(#[$meta])*
                pub const $name: Self = Self {
                    boxtype: FourCC::new(*$fourcc),
                    usertype: None,
                };
            )*
        }
    };
}

define_box_types! {
    // =========================================================================
    // ISO 14496-12 (BMFF) - Core boxes
    // =========================================================================

    /// File Type Box
    FTYP = b"ftyp",
    /// Segment Type Box
    STYP = b"styp",
    /// Free Space Box
    FREE = b"free",
    /// Media Data Box
    MDAT = b"mdat",

    // =========================================================================
    // ISO 14496-12 (BMFF) - Movie structure
    // =========================================================================

    /// Movie Box
    MOOV = b"moov",
    /// Movie Header Box
    MVHD = b"mvhd",
    /// Track Box
    TRAK = b"trak",
    /// Track Header Box
    TKHD = b"tkhd",
    /// Track Reference Box
    TREF = b"tref",
    /// Media Information Box
    MINF = b"minf",
    /// Handler Reference Box
    HDLR = b"hdlr",
    /// Media Header Box
    MDHD = b"mdhd",
    /// Media Box
    MDIA = b"mdia",

    // =========================================================================
    // ISO 14496-12 (BMFF) - Data information
    // =========================================================================

    /// Data Information Box
    DINF = b"dinf",
    /// Data Reference Box
    DREF = b"dref",
    /// URL Box
    URL_ = b"url ",
    /// URN Box
    URN_ = b"urn ",

    // =========================================================================
    // ISO 14496-12 (BMFF) - Sample table
    // =========================================================================

    /// Sample Table Box
    STBL = b"stbl",
    /// Sample Description Box
    STSD = b"stsd",
    /// Decoding Time to Sample Box
    STTS = b"stts",
    /// Composition Time to Sample Box
    CTTS = b"ctts",
    /// Composition to Decode Timeline Mapping Box
    CSLG = b"cslg",
    /// Sample to Chunk Box
    STSC = b"stsc",
    /// Sample Size Box
    STSZ = b"stsz",
    /// Sync Sample Table Box
    STSS = b"stss",
    /// Chunk Offset Box
    STCO = b"stco",
    /// 64-bit Chunk Offset Box
    CO64 = b"co64",

    // =========================================================================
    // ISO 14496-12 (BMFF) - Media headers
    // =========================================================================

    /// Video Media Header Box
    VMHD = b"vmhd",
    /// Sound Media Header Box
    SMHD = b"smhd",
    /// Hint Media Header Box
    HMHD = b"hmhd",
    /// Null Media Header Box
    NMHD = b"nmhd",

    // =========================================================================
    // ISO 14496-12 (BMFF) - Movie Extends (fragmented movies)
    // =========================================================================

    /// Movie Extends Box
    MVEX = b"mvex",
    /// Movie Extends Header Box
    MEHD = b"mehd",
    /// Track Extends Defaults Box
    TREX = b"trex",

    // =========================================================================
    // ISO 14496-12 (BMFF) - Movie Fragments
    // =========================================================================

    /// Movie Fragment Box
    MOOF = b"moof",
    /// Movie Fragment Header Box
    MFHD = b"mfhd",
    /// Track Fragment Box
    TRAF = b"traf",
    /// Track Fragment Header Box
    TFHD = b"tfhd",
    /// Track Fragment Decode Time Box
    TFDT = b"tfdt",
    /// Track Run Box
    TRUN = b"trun",

    // =========================================================================
    // ISO 14496-12 (BMFF) - Movie Fragment Random Access
    // =========================================================================

    /// Movie Fragment Random Access Box
    MFRA = b"mfra",
    /// Track Fragment Random Access Box
    TFRA = b"tfra",
    /// Movie Fragment Random Access Offset Box
    MFRO = b"mfro",

    // =========================================================================
    // ISO 14496-12 (BMFF) - Sample grouping
    // =========================================================================

    /// Sample to Group Box
    SBGP = b"sbgp",
    /// Sample Group Description Box
    SGPD = b"sgpd",

    // =========================================================================
    // ISO 14496-15 (AVC/HEVC file format)
    // =========================================================================

    /// AVC Sample Entry Box
    AVC1 = b"avc1",
    /// AVC Configuration Box
    AVCC = b"avcC",

    // =========================================================================
    // ISO 14496-14 (MP4 file format)
    // =========================================================================

    /// MP4 Audio Sample Entry Box
    MP4A = b"mp4a",
    /// Elementary Stream Descriptor Box
    ESDS = b"esds",
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
