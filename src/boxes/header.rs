//!

use core::fmt;

use super::boxsize::BoxSize;
use super::boxtype::BoxType;

/// Represents the header of a BMFF box, including its size and type.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoxHeader {
    size: BoxSize,
    type_: BoxType,
}

impl BoxHeader {
    /// Base size of a box header (size + type fields).
    const BASE_SIZE: u64 = 8;

    /// Creates a new box header.
    pub const fn new(size: BoxSize, type_: BoxType) -> Self {
        Self { size, type_ }
    }

    /// Returns the size of the box header in bytes.
    pub fn header_size(&self) -> u64 {
        let base = Self::BASE_SIZE;

        // Add largesize field size if needed
        let extened = match self.size {
            BoxSize::Size64(_) => 8,
            _ => 0,
        };

        // Add UUID field size if needed
        let uuid = if self.type_.is_uuid() { 16 } else { 0 };

        base + extened + uuid
    }

    /// Returns the payload size of the box, or None if the size extends to the end of the file.
    pub fn payload_size(&self) -> Option<u64> {
        self.size
            .as_u64()
            .and_then(|total| total.checked_sub(self.header_size()))
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FourCC;
    use crate::types::Uuid;

    #[test]
    fn header_size_calculation() {
        let header = BoxHeader::new(
            BoxSize::Size32(20),
            BoxType::from_fourcc(FourCC::from(*b"moov")).unwrap(),
        );
        assert_eq!(header.header_size(), 8);
        assert_eq!(header.payload_size(), Some(12));

        let header = BoxHeader::new(
            BoxSize::Size64(40),
            BoxType::from_uuid(Uuid::new([0x01; 16])),
        );
        assert_eq!(header.header_size(), 32);
        assert_eq!(header.payload_size(), Some(8));
    }
}
