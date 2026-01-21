//! ISOBMFF box header structures.
//!
//! Includes definitions and implementations for box size and type,
//! as well as the box header itself.

use core::fmt;

mod boxsize;
mod boxtype;

// Re-export for convenience
pub use boxsize::BoxSize;
pub use boxtype::{
    BoxType, //
    UserType,
};

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;
use crate::types::FourCC;

use crate::error::*;

/// Represents the header of a BMFF box, including its size and type.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoxHeader {
    size: BoxSize,
    type_: BoxType,
    len: usize,
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

    fn calculate_len(size: &BoxSize, type_: &BoxType) -> usize {
        let mut len = Self::BASE_SIZE;
        if size.is_extended() {
            len += 8; // Additional 8 bytes for extended size
        }

        if type_.is_uuid() {
            len += 16; // Additional 16 bytes for UUID
        }

        len
    }

    /// Creates a new box header.
    pub fn new(type_: BoxType, payload_len: usize) -> Self {
        // Calculate header length including UUID if applicable
        let mut header_len = Self::BASE_SIZE as u64;
        if type_.is_uuid() {
            header_len += 16;
        }

        let Some(total_len) = header_len.checked_add(payload_len as u64) else {
            panic!("Box size overflow when creating BoxHeader");
        };

        let size = BoxSize::new(total_len);
        let len = Self::calculate_len(&size, &type_);
        Self { size, type_, len }
    }

    /// Returns the total length of the box header in bytes.
    pub fn header_len(&self) -> usize {
        self.len
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

    pub(crate) fn parse_in(cur: &mut ReadCursor<'_>) -> Result<Self> {
        let start_pos = cur.position();
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

        let len = cur.position() - start_pos;
        Ok(BoxHeader { size, type_, len })
    }

    /// Parses a box header from the given byte slice.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);
        Self::parse_in(&mut cur)
    }

    pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
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
