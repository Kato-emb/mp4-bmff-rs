use core::mem;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;
use crate::types::I8F8;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
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
}

impl BoxCodec for SmhdBox {
    fn boxtype(&self) -> BoxType {
        BoxType::SMHD
    }
}

impl BoxDecode<'_> for SmhdBox {
    fn decode(bytes: &[u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = SmhdFlags::from_bytes(cur.read_array()?);

        let balance = cur.read_i16_be()?;
        let balance = I8F8::from_raw(balance);

        // Skip reserved (2 bytes)
        cur.advance(Self::RESERVED_SIZE)?;

        Ok(SmhdBox {
            version,
            flags,
            balance,
        })
    }
}

impl TryFrom<&[u8]> for SmhdBox {
    type Error = Error;

    fn try_from(payload: &[u8]) -> Result<Self> {
        SmhdBox::decode(payload)
    }
}

impl BoxEncode for SmhdBox {
    #[inline]
    fn encoded_len(&self) -> usize {
        1 // version
            + 3 // flags
            + 2 // balance
            + Self::RESERVED_SIZE // reserved
    }

    fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

        // Write version (1 byte)
        cur.write_u8(self.version)?;

        // Write flags (3 bytes)
        cur.write_array(&self.flags.to_bytes())?;

        // Write balance (2 bytes)
        cur.write_i16_be(self.balance.to_raw())?;

        // Write reserved (2 bytes)
        cur.reserve_zeros(Self::RESERVED_SIZE)?;

        Ok(cur.position())
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

        let mut buf = vec![0u8; 32];
        let written = original.encode_into(&mut buf).unwrap();

        let reparsed = SmhdBox::decode(&buf[..written]).unwrap();
        assert_eq!(reparsed.version, original.version);
        assert_eq!(reparsed.flags.get(), original.flags.get());
        assert_eq!(reparsed.balance.to_raw(), original.balance.to_raw());
    }
}
