use crate::cursor::ReadCursor;

use crate::BoxType;
use crate::BoxView;
use crate::FullBoxFlags;
use crate::FullBoxHeader;
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
        let full_box_header = FullBoxHeader::<VmhdSpec>::parse_in(cur)?;

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
            version: full_box_header.version(),
            flags: full_box_header.flags(),
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

impl TryFrom<&BoxView<'_>> for VmhdBox {
    type Error = Error;

    fn try_from(box_view: &BoxView<'_>) -> Result<Self> {
        if box_view.header.boxtype() != BoxType::VMHD {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::VMHD,
                found: box_view.header.boxtype(),
            }));
        }

        VmhdBox::parse(box_view.payload)
    }
}

/// Specification for Video Media Header Box (`vmhd`).
pub struct VmhdSpec;

/// Flags for Video Media Header Box (`vmhd`).
pub type VmhdFlags = FullBoxFlags<VmhdSpec>;
