use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;

use super::FullBoxFlags;

/// A reference to a Video Media Header Box (`vmhd`).
#[derive(Debug, Clone, Copy)]
pub struct VmhdBox {
    /// Box version
    pub version: u8,
    /// Box flags
    pub flags: VmhdFlags,
    /// Graphics mode
    pub graphicsmode: u16,
    /// Opcolor
    pub opcolor: [u16; 3],
}

impl Default for VmhdBox {
    fn default() -> Self {
        VmhdBox {
            version: 0,
            flags: VmhdFlags::new(1),
            graphicsmode: 0,
            opcolor: [0, 0, 0],
        }
    }
}

impl VmhdBox {
    pub(crate) fn parse_in(cur: &mut ReadCursor<'_>) -> Result<VmhdBox> {
        let version = cur.read_u8()?;
        let flags = VmhdFlags::from_bytes(cur.read_array()?);

        let graphicsmode = cur.read_u16_be()?;

        let mut opcolor = [0u16; 3];
        for color in &mut opcolor {
            *color = cur.read_u16_be()?;
        }

        if !cur.is_empty() {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Extra data after parsing vmhd",
                    got: cur.remaining() as u64,
                },
                BoxType::VMHD,
            ));
        }

        Ok(VmhdBox {
            version,
            flags,
            graphicsmode,
            opcolor,
        })
    }

    /// Parses a `VmhdBox` from the given payload.
    pub fn parse(payload: &[u8]) -> Result<VmhdBox> {
        let mut cursor = ReadCursor::new(payload);
        VmhdBox::parse_in(&mut cursor)
    }

    /// Returns the size of the payload in bytes.
    pub fn size(&self) -> usize {
        // version(1) + flags(3) + graphicsmode(2) + opcolor(6) = 12
        12
    }

    pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
        // Write version (1 byte)
        cur.write_u8(self.version)?;

        // Write flags (3 bytes)
        cur.write_array(&self.flags.to_bytes())?;

        // Write graphicsmode (2 bytes)
        cur.write_u16_be(self.graphicsmode)?;

        // Write opcolor (6 bytes = 3 x u16)
        for color in &self.opcolor {
            cur.write_u16_be(*color)?;
        }

        if !cur.is_empty() {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Buffer larger than expected",
                    got: cur.remaining() as u64,
                },
                BoxType::VMHD,
            ));
        }

        Ok(())
    }

    /// Writes this `VmhdBox` into the given payload.
    pub fn write(&self, payload: &mut [u8]) -> Result<()> {
        let mut cursor = WriteCursor::new(payload);
        self.write_in(&mut cursor)
    }
}

impl TryFrom<&[u8]> for VmhdBox {
    type Error = Error;

    fn try_from(payload: &[u8]) -> Result<Self> {
        VmhdBox::parse(payload)
    }
}

impl BoxCodec for VmhdBox {
    fn boxtype(&self) -> BoxType {
        BoxType::VMHD
    }
}

impl BoxDecode<'_> for VmhdBox {
    fn decode(bytes: &[u8]) -> Result<Self> {
        VmhdBox::parse(bytes)
    }
}

impl BoxEncode for VmhdBox {
    fn encode(&self, bytes: &mut [u8]) -> Result<usize> {
        self.write(bytes)?;
        Ok(self.size())
    }
}

/// Specification for Video Media Header Box (`vmhd`).
pub struct VmhdSpec;

/// Flags for Video Media Header Box (`vmhd`).
pub type VmhdFlags = FullBoxFlags<VmhdSpec>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let original = VmhdBox {
            version: 0,
            flags: VmhdFlags::new(1),
            graphicsmode: 0x1234,
            opcolor: [0x1111, 0x2222, 0x3333],
        };

        let mut buf = vec![0u8; original.size()];
        original.write(&mut buf).unwrap();

        let reparsed = VmhdBox::parse(&buf).unwrap();
        assert_eq!(reparsed.version, original.version);
        assert_eq!(reparsed.flags.get(), original.flags.get());
        assert_eq!(reparsed.graphicsmode, original.graphicsmode);
        assert_eq!(reparsed.opcolor, original.opcolor);
    }
}
