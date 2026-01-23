use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;

use super::FullBoxFlags;

/// A Null Media Header Box (`nmhd`).
///
/// This box is used for tracks that do not otherwise have a specific media header
/// (neither `vmhd`, `smhd`, nor `hmhd`). This includes some metadata tracks.
#[derive(Debug, Clone, Copy)]
pub struct NmhdBox {
    /// Box version (should be 0).
    pub version: u8,
    /// Box flags (should be 0).
    pub flags: NmhdFlags,
}

impl Default for NmhdBox {
    fn default() -> Self {
        NmhdBox {
            version: 0,
            flags: NmhdFlags::empty(),
        }
    }
}

/// Specification for Null Media Header Box (`nmhd`).
pub struct NmhdSpec;

/// Flags for Null Media Header Box (`nmhd`).
pub type NmhdFlags = FullBoxFlags<NmhdSpec>;

impl BoxCodec for NmhdBox {
    fn boxtype(&self) -> BoxType {
        BoxType::NMHD
    }
}

impl BoxDecode<'_> for NmhdBox {
    fn decode(bytes: &[u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = NmhdFlags::from_bytes(cur.read_array()?);

        Ok(NmhdBox { version, flags })
    }
}

impl TryFrom<&[u8]> for NmhdBox {
    type Error = Error;

    fn try_from(payload: &[u8]) -> Result<Self> {
        NmhdBox::decode(payload)
    }
}

impl BoxEncode for NmhdBox {
    #[inline]
    fn encoded_len(&self) -> usize {
        4 // version(1) + flags(3)
    }

    fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

        cur.write_u8(self.version)?;
        cur.write_array(&self.flags.to_bytes())?;

        Ok(cur.position())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let original = NmhdBox {
            version: 0,
            flags: NmhdFlags::new(1),
        };

        let mut buf = vec![0u8; 4];
        original.encode_into(&mut buf).unwrap();

        let reparsed = NmhdBox::decode(&buf).unwrap();
        assert_eq!(reparsed.version, original.version);
        assert_eq!(reparsed.flags.get(), original.flags.get());
    }
}
