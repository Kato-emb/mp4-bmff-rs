use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

use crate::RawBoxRef;
use crate::BoxType;
use crate::error::*;

use super::FullBoxFlags;

/// Movie Fragment Random Access Offset Box (`mfro`).
///
/// This box provides the offset to the containing `mfra` box,
/// allowing the `mfra` box to be located from the end of the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MfroBox {
    /// The version of the box (must be 0).
    pub version: u8,
    /// The flags of the box.
    pub flags: MfroFlags,
    /// The size of the enclosing `mfra` box (including this `mfro` box).
    pub size: u32,
}

impl MfroBox {
    pub(crate) fn parse_in(cur: &mut ReadCursor<'_>) -> Result<MfroBox> {
        let version = cur.read_u8()?;
        let flags = MfroFlags::from_bytes(cur.read_array()?);

        if version != 0 {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxVersion {
                    reason: "mfro version must be 0",
                    got: version,
                },
                BoxType::MFRO,
            ));
        }

        let mfra_size = cur.read_u32_be()?;

        if !cur.is_empty() {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Extra data after mfro fields",
                    got: cur.remaining() as u64,
                },
                BoxType::MFRO,
            ));
        }

        Ok(MfroBox {
            version,
            flags,
            size: mfra_size,
        })
    }

    /// Parses a `MfroBox` from the given payload.
    pub fn parse(payload: &[u8]) -> Result<MfroBox> {
        let mut cur = ReadCursor::new(payload);
        MfroBox::parse_in(&mut cur)
    }

    /// Returns the size of the payload in bytes.
    pub fn payload_size(&self) -> usize {
        // version(1) + flags(3) + size(4) = 8
        8
    }

    pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
        cur.write_u8(self.version)?;
        cur.write_array(&self.flags.to_bytes())?;

        cur.write_u32_be(self.size)?;

        if !cur.is_empty() {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxSize {
                    reason: "Buffer larger than expected",
                    got: cur.remaining() as u64,
                },
                BoxType::MFRO,
            ));
        }

        Ok(())
    }

    /// Writes this `MfroBox` into the given payload.
    pub fn write(&self, payload: &mut [u8]) -> Result<()> {
        let mut cursor = WriteCursor::new(payload);
        self.write_in(&mut cursor)
    }
}

impl TryFrom<&[u8]> for MfroBox {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self> {
        MfroBox::parse(value)
    }
}

impl TryFrom<RawBoxRef<'_>> for MfroBox {
    type Error = Error;

    fn try_from(value: RawBoxRef<'_>) -> Result<Self> {
        if value.boxtype() != BoxType::MFRO {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::MFRO,
                found: value.boxtype(),
            }));
        }

        MfroBox::parse(value.payload())
    }
}

/// Specification for the Movie Fragment Random Access Offset Box (`mfro`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MfroSpec;

/// Flags for the Movie Fragment Random Access Offset Box (`mfro`).
pub type MfroFlags = FullBoxFlags<MfroSpec>;

#[cfg(test)]
mod tests {
    use super::*;

    fn make_full_box_header(version: u8, flags: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(version);
        data.extend_from_slice(&flags.to_be_bytes()[1..4]);
        data
    }

    fn make_mfro_payload(mfra_size: u32) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(0, 0));
        payload.extend_from_slice(&mfra_size.to_be_bytes());
        payload
    }

    #[test]
    fn parse_mfro() {
        let payload = make_mfro_payload(1024);
        let mfro = MfroBox::parse(&payload).unwrap();

        assert_eq!(mfro.version, 0);
        assert_eq!(mfro.size, 1024);
    }

    #[test]
    fn parse_mfro_invalid_version() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(1, 0)); // version 1 is invalid
        payload.extend_from_slice(&2048u32.to_be_bytes());

        let result = MfroBox::parse(&payload);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::InvalidBoxVersion { .. }));
        }
    }

    #[test]
    fn parse_mfro_extra_data() {
        let mut payload = make_mfro_payload(512);
        payload.extend_from_slice(&[0, 0, 0, 0]); // extra data

        let result = MfroBox::parse(&payload);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::InvalidBoxSize { .. }));
        }
    }

    #[test]
    fn parse_mfro_truncated() {
        let payload = make_full_box_header(0, 0);
        // Missing mfra_size

        let result = MfroBox::parse(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn try_from_byte_slice() {
        let payload = make_mfro_payload(4096);
        let mfro = MfroBox::try_from(payload.as_slice()).unwrap();

        assert_eq!(mfro.version, 0);
        assert_eq!(mfro.size, 4096);
    }
}
