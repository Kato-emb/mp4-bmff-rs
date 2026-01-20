use crate::cursor::ReadCursor;

use crate::BoxType;
use crate::BoxFrame;
use crate::FullBoxFlags;
use crate::error::*;

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
        let version = cur
            .read_u8()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let flags = VmhdFlags::from_bytes(
            cur.read_array()
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
        );

        let graphicsmode = cur
            .read_u16_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        let mut opcolor = [0u16; 3];
        for color in &mut opcolor {
            *color = cur
                .read_u16_be()
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
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
}

impl TryFrom<&[u8]> for VmhdBox {
    type Error = Error;

    fn try_from(payload: &[u8]) -> Result<Self> {
        VmhdBox::parse(payload)
    }
}

impl TryFrom<BoxFrame<'_>> for VmhdBox {
    type Error = Error;

    fn try_from(value: BoxFrame<'_>) -> Result<Self> {
        if value.boxtype() != BoxType::VMHD {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::VMHD,
                found: value.boxtype(),
            }));
        }

        VmhdBox::parse(value.payload())
    }
}

/// Specification for Video Media Header Box (`vmhd`).
pub struct VmhdSpec;

/// Flags for Video Media Header Box (`vmhd`).
pub type VmhdFlags = FullBoxFlags<VmhdSpec>;
