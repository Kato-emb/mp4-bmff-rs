use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
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

/// Specification for the Movie Fragment Random Access Offset Box (`mfro`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MfroSpec;

/// Flags for the Movie Fragment Random Access Offset Box (`mfro`).
pub type MfroFlags = FullBoxFlags<MfroSpec>;

impl BoxCodec for MfroBox {
    fn boxtype(&self) -> BoxType {
        BoxType::MFRO
    }
}

impl BoxDecode<'_> for MfroBox {
    fn decode(bytes: &[u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

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

        Ok(MfroBox {
            version,
            flags,
            size: mfra_size,
        })
    }
}

impl TryFrom<&[u8]> for MfroBox {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self> {
        MfroBox::decode(value)
    }
}

impl BoxEncode for MfroBox {
    fn encode(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

        cur.write_u8(self.version)?;
        cur.write_array(&self.flags.to_bytes())?;

        cur.write_u32_be(self.size)?;

        Ok(cur.position())
    }
}

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
        let mfro = MfroBox::decode(&payload).unwrap();

        assert_eq!(mfro.version, 0);
        assert_eq!(mfro.size, 1024);
    }

    #[test]
    fn parse_mfro_invalid_version() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(1, 0)); // version 1 is invalid
        payload.extend_from_slice(&2048u32.to_be_bytes());

        let result = MfroBox::decode(&payload);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err.kind(), ErrorKind::InvalidBoxVersion { .. }));
        }
    }

    #[test]
    fn parse_mfro_truncated() {
        let payload = make_full_box_header(0, 0);
        // Missing mfra_size

        let result = MfroBox::decode(&payload);
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
