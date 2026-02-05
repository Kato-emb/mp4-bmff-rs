//! BMFF box header structures.
//!
//! This module provides types for working with box headers as defined in
//! ISO/IEC 14496-12. Every BMFF box begins with a header containing:
//!
//! - **Size field** (4 bytes): Total box size, or markers for extended/EOF.
//! - **Type field** (4 bytes): FourCC identifying the box type.
//! - **Extended size** (8 bytes, optional): 64-bit size for large boxes.
//! - **User type** (16 bytes, optional): UUID for custom box types.
//!
//! # Types
//!
//! - [`BoxHeader`]: Complete header with size and type information.
//! - [`BoxSize`]: Type-safe representation of box sizes.
//! - [`BoxType`]: Type-safe representation of box types (FourCC or UUID).
//! - [`UserType`]: Alias for UUID used in extended box types.
//!
//! # Size Encoding
//!
//! | Size Field | Meaning |
//! |------------|---------|
//! | 0 | Box extends to end of file |
//! | 1 | 64-bit size follows in extended size field |
//! | 8+ | Actual box size (minimum valid size) |

use core::fmt;

mod boxsize;
mod boxtype;

// Re-export for convenience
pub use boxsize::BoxSize;
pub use boxtype::{
    BoxType, //
    UserType,
};

use crate::types::FourCC;

use crate::error::*;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

/// Represents the header of a BMFF box.
///
/// The header contains the box's size and type, and handles both compact
/// (32-bit) and extended (64-bit) size encodings, as well as standard
/// FourCC and UUID-based box types.
///
/// # Structure
///
/// - `size`: Box size (compact, extended, or to-end-of-file).
/// - `type_`: Box type (FourCC or UUID).
///
/// # Header Sizes
///
/// - **Base header**: 8 bytes (4-byte size + 4-byte type).
/// - **Extended size**: +8 bytes when size > u32::MAX.
/// - **UUID type**: +16 bytes when type is "uuid".
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoxHeader {
    size: BoxSize,
    type_: BoxType,
}

impl fmt::Debug for BoxHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BoxHeader")
            .field("size", &self.size)
            .field("type", &self.type_)
            .finish()
    }
}

impl fmt::Display for BoxHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (size: {})", self.type_, self.size)
    }
}

impl BoxHeader {
    /// Base size of a box header (size + type fields).
    pub const BASE_SIZE: usize = 8;

    /// Creates a new box header.
    pub fn new(type_: BoxType, payload_len: u64) -> Self {
        // Calculate header length including UUID if applicable
        let mut header_len = Self::BASE_SIZE as u64;
        if type_.is_uuid() {
            header_len += 16;
        }

        let Some(total_len) = header_len.checked_add(payload_len) else {
            panic!("Box size overflow when creating BoxHeader");
        };

        let size = BoxSize::new(total_len);
        Self { size, type_ }
    }

    /// Returns the total length of the box header in bytes.
    #[inline]
    pub const fn header_len(&self) -> usize {
        let mut len = Self::BASE_SIZE;

        if self.size.is_extended() {
            len += 8; // Additional 8 bytes for extended size
        }

        if self.type_.is_uuid() {
            len += 16; // Additional 16 bytes for UUID
        }

        len
    }

    /// Returns the box size.
    pub fn boxsize(&self) -> BoxSize {
        self.size
    }

    /// Returns the box type.
    pub fn boxtype(&self) -> BoxType {
        self.type_
    }

    /// Returns the total size of the box, including header and payload.
    pub fn total_size(&self) -> u64 {
        self.size.value().unwrap_or(0)
    }

    /// Calculates the number of additional header bytes based on the base header.
    pub fn addintional_header_bytes(base: &[u8; Self::BASE_SIZE]) -> usize {
        let size = u32::from_be_bytes([base[0], base[1], base[2], base[3]]);
        let fourcc = FourCC::from([base[4], base[5], base[6], base[7]]);

        let mut additional_bytes = 0;

        if size == BoxSize::MARKER_EXTENDED_SIZE {
            additional_bytes += 8; // Extended size field
        }

        if fourcc == boxtype::UUID {
            additional_bytes += 16; // UUID field
        }

        additional_bytes
    }

    /// Parses a box header from the given byte slice.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let size = cur.read_u32_be()?;
        let fourcc = FourCC::from(cur.read_array::<4>()?);

        let size = match size {
            0 => BoxSize::eof(),
            1 => {
                let largesize = cur.read_u64_be()?;
                BoxSize::new_extended(largesize)
            }
            _ => BoxSize::new_compact(size),
        };

        let type_ = if fourcc == boxtype::UUID {
            let user_type = UserType::from(cur.read_array::<16>()?);
            BoxType::with_usertype(user_type)
        } else {
            BoxType::new(fourcc)
        };

        Ok(BoxHeader { size, type_ })
    }

    /// Writes the box header into the given byte slice.
    pub fn write(&self, bytes: &mut [u8]) -> Result<()> {
        let mut cur = WriteCursor::new(bytes);

        if self.size.is_extended() {
            // Write extended size
            cur.write_u32_be(BoxSize::MARKER_EXTENDED_SIZE)?; // Indicate extended size
        } else if self.size.is_eof() {
            // Write EOF marker
            cur.write_u32_be(BoxSize::MARKER_EOF)?; // Indicate box extends to end of file
        } else {
            // Write compact size
            cur.write_u32_be(self.size.value().unwrap() as u32)?; // Write size
        }

        cur.write_array(self.type_.type_field().as_bytes())?; // Write type field

        if self.size.is_extended() {
            // Write extended size
            cur.write_u64_be(self.size.value().unwrap())?;
        }

        if self.type_.is_uuid() {
            // Write user type (UUID)
            cur.write_array(self.type_.user_type().unwrap().as_bytes())?;
        }

        Ok(())
    }
}
