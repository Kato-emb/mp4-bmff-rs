//!

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

use crate::BoxHeader;
use crate::BoxType;
use crate::error::*;

/// Traits for BMFF boxes.
pub trait BmffBox {
    /// Returns the box type.
    fn boxtype(&self) -> BoxType;
    /// Returns the payload size of the box.
    fn payload_size(&self) -> u64;

    /// Returns the header size of the box.
    #[inline]
    fn header_size(&self) -> usize {
        let mut len = BoxHeader::BASE_SIZE;
        if self.boxtype().is_uuid() {
            len += 16;
        }

        if self.payload_size() > u32::MAX as u64 - len as u64 {
            len += 8; // extended size
        }

        len
    }

    /// Returns the total size of the box (header + payload).
    #[inline]
    fn total_size(&self) -> u64 {
        self.header_size() as u64 + self.payload_size()
    }
}

pub(crate) trait DecodeIn<'de>: Sized {
    fn decode_in(cur: &mut ReadCursor<'de>) -> Result<Self>;
}

pub(crate) trait EncodeIn {
    fn encode_in(&self, cur: &mut WriteCursor<'_>) -> Result<()>;
}

/// Traits for decoding BMFF boxes.
pub trait BoxDecode<'de>: Sized {
    /// Decodes this box from the given buffer.
    fn decode(buf: &'de [u8]) -> Result<Self>;
}

impl<'de, T> BoxDecode<'de> for T
where
    T: DecodeIn<'de>,
{
    fn decode(buf: &'de [u8]) -> Result<Self> {
        let mut cursor = ReadCursor::new(buf);
        let this = T::decode_in(&mut cursor)?;

        if !cursor.is_empty() {
            return Err(Error::new(ErrorKind::MismatchedBoxSize {
                expected: buf.len() as u64 - cursor.remaining() as u64,
                found: buf.len() as u64,
            }));
        }

        Ok(this)
    }
}

/// Traits for encoding BMFF boxes.
pub trait BoxEncode {
    /// Encodes this box into the given buffer.
    fn encode(&self, buf: &mut [u8]) -> Result<()>;
}

impl<T> BoxEncode for T
where
    T: EncodeIn + BmffBox,
{
    fn encode(&self, buf: &mut [u8]) -> Result<()> {
        let expected = self.payload_size();

        // Check for buffer size overflow
        if expected > usize::MAX as u64 {
            return Err(Error::in_box(
                ErrorKind::BufferTooLarge {
                    expected,
                    max: usize::MAX as u64,
                },
                self.boxtype(),
            ));
        }

        // Check buffer size matches expected payload size
        if buf.len() as u64 != expected {
            return Err(Error::in_box(
                ErrorKind::MismatchedBoxSize {
                    expected: self.payload_size() as u64,
                    found: buf.len() as u64,
                },
                self.boxtype(),
            ));
        }

        let mut cur = WriteCursor::new(buf);
        self.encode_in(&mut cur)?;

        // Ensure we've written all bytes
        debug_assert!(cur.is_empty());

        Ok(())
    }
}
