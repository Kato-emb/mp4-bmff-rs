use core::mem;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;
use crate::types::I8F8;

use crate::BoxFrame;
use crate::BoxType;
use crate::error::*;

use super::FullBoxFlags;

/// A Sound Media Header Box (`smhd`).
///
/// This box contains general presentation information, independent of the coding,
/// for audio media. This header is used for all tracks containing audio.
#[derive(Debug, Clone, Copy)]
pub struct SmhdBox {
    /// Box version (should be 0).
    pub version: u8,
    /// Box flags (should be 0).
    pub flags: SmhdFlags,
    /// Stereo balance, generally 0. -1.0 = full left, 1.0 = full right.
    pub balance: I8F8,
}

impl Default for SmhdBox {
    fn default() -> Self {
        SmhdBox {
            version: 0,
            flags: SmhdFlags::empty(),
            balance: I8F8::from_raw(0),
        }
    }
}

impl SmhdBox {
    const RESERVED_SIZE: usize = mem::size_of::<u16>();

    pub(crate) fn parse_in(cur: &mut ReadCursor<'_>) -> Result<SmhdBox> {
        let version = cur.read_u8()?;
        let flags = SmhdFlags::from_bytes(cur.read_array()?);

        let balance = cur.read_i16_be()?;
        let balance = I8F8::from_raw(balance);

        // Skip reserved (2 bytes)
        cur.advance(Self::RESERVED_SIZE)?;

        if !cur.is_empty() {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Extra data after parsing smhd",
                    got: cur.remaining() as u64,
                },
                BoxType::SMHD,
            ));
        }

        Ok(SmhdBox {
            version,
            flags,
            balance,
        })
    }

    /// Parses a `SmhdBox` from the given payload.
    pub fn parse(payload: &[u8]) -> Result<SmhdBox> {
        let mut cursor = ReadCursor::new(payload);
        SmhdBox::parse_in(&mut cursor)
    }

    /// Returns the size of the payload in bytes.
    pub fn size(&self) -> usize {
        // version(1) + flags(3) + balance(2) + reserved(2) = 8
        8
    }

    pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
        // Write version (1 byte)
        cur.write_u8(self.version)?;

        // Write flags (3 bytes)
        cur.write_array(&self.flags.to_bytes())?;

        // Write balance (2 bytes)
        cur.write_i16_be(self.balance.to_raw())?;

        // Write reserved (2 bytes)
        cur.reserve_zeros(Self::RESERVED_SIZE)?;

        if !cur.is_empty() {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Buffer larger than expected",
                    got: cur.remaining() as u64,
                },
                BoxType::SMHD,
            ));
        }

        Ok(())
    }

    /// Writes this `SmhdBox` into the given payload.
    pub fn write(&self, payload: &mut [u8]) -> Result<()> {
        let mut cursor = WriteCursor::new(payload);
        self.write_in(&mut cursor)
    }
}

impl TryFrom<&[u8]> for SmhdBox {
    type Error = Error;

    fn try_from(payload: &[u8]) -> Result<Self> {
        SmhdBox::parse(payload)
    }
}

impl TryFrom<BoxFrame<'_>> for SmhdBox {
    type Error = Error;

    fn try_from(value: BoxFrame<'_>) -> Result<Self> {
        if value.boxtype() != BoxType::SMHD {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::SMHD,
                found: value.boxtype(),
            }));
        }

        SmhdBox::parse(value.payload())
    }
}

/// Specification for Sound Media Header Box (`smhd`).
pub struct SmhdSpec;

/// Flags for Sound Media Header Box (`smhd`).
pub type SmhdFlags = FullBoxFlags<SmhdSpec>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let original = SmhdBox {
            version: 0,
            flags: SmhdFlags::empty(),
            balance: I8F8::from_raw(0x0080), // 0.5 (slightly right)
        };

        let mut buf = vec![0u8; original.size()];
        original.write(&mut buf).unwrap();

        let reparsed = SmhdBox::parse(&buf).unwrap();
        assert_eq!(reparsed.version, original.version);
        assert_eq!(reparsed.flags.get(), original.flags.get());
        assert_eq!(reparsed.balance.to_raw(), original.balance.to_raw());
    }
}
