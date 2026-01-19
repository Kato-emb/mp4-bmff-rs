use core::mem;

use crate::cursor::ReadCursor;
use crate::types::I8F8;

use crate::BoxType;
use crate::BoxView;
use crate::FullBoxFlags;
use crate::FullBoxHeader;
use crate::error::*;

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
        let full_box_header = FullBoxHeader::<SmhdSpec>::parse_in(cur)?;

        let balance = cur
            .read_i16_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let balance = I8F8::from_raw(balance);

        // Skip reserved (2 bytes)
        cur.advance(Self::RESERVED_SIZE)
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        if !cur.is_empty() {
            return Err(Error::new(ErrorKind::InvalidBoxSize {
                reason: "Extra data after parsing smhd",
                got: cur.remaining() as u64,
            }));
        }

        Ok(SmhdBox {
            version: full_box_header.version(),
            flags: full_box_header.flags(),
            balance,
        })
    }

    /// Parses a `SmhdBox` from the given payload.
    pub fn parse(payload: &[u8]) -> Result<SmhdBox> {
        let mut cursor = ReadCursor::new(payload);
        SmhdBox::parse_in(&mut cursor)
    }
}

impl TryFrom<&[u8]> for SmhdBox {
    type Error = Error;

    fn try_from(payload: &[u8]) -> Result<Self> {
        SmhdBox::parse(payload)
    }
}

impl TryFrom<&BoxView<'_>> for SmhdBox {
    type Error = Error;

    fn try_from(box_view: &BoxView<'_>) -> Result<Self> {
        if box_view.header.boxtype() != BoxType::SMHD {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::SMHD,
                found: box_view.header.boxtype(),
            }));
        }

        SmhdBox::parse(box_view.payload)
    }
}

/// Specification for Sound Media Header Box (`smhd`).
pub struct SmhdSpec;

/// Flags for Sound Media Header Box (`smhd`).
pub type SmhdFlags = FullBoxFlags<SmhdSpec>;

#[cfg(test)]
mod tests {
    use super::*;

    fn make_smhd_payload(version: u8, flags: u32, balance: i16) -> Vec<u8> {
        let mut data = Vec::new();

        // FullBoxHeader: version (1 byte) + flags (3 bytes)
        data.push(version);
        data.extend_from_slice(&flags.to_be_bytes()[1..4]);

        // balance (2 bytes)
        data.extend_from_slice(&balance.to_be_bytes());

        // reserved (2 bytes)
        data.extend_from_slice(&[0u8; 2]);

        data
    }

    #[test]
    fn parse_smhd_default() {
        let payload = make_smhd_payload(0, 0, 0);
        let smhd = SmhdBox::parse(&payload).unwrap();

        assert_eq!(smhd.version, 0);
        assert_eq!(smhd.flags.get(), 0);
        assert_eq!(smhd.balance.to_raw(), 0);
    }

    #[test]
    fn parse_smhd_with_balance() {
        // balance = 0x0080 = 0.5 (slightly right)
        let payload = make_smhd_payload(0, 0, 0x0080);
        let smhd = SmhdBox::parse(&payload).unwrap();

        assert_eq!(smhd.balance.to_raw(), 0x0080);
    }

    #[test]
    fn parse_smhd_negative_balance() {
        // balance = -256 (0xFF00) = -1.0 (full left)
        let payload = make_smhd_payload(0, 0, -256);
        let smhd = SmhdBox::parse(&payload).unwrap();

        assert_eq!(smhd.balance.to_raw(), -256);
    }

    #[test]
    fn parse_smhd_extra_data() {
        let mut payload = make_smhd_payload(0, 0, 0);
        payload.extend_from_slice(&[0xFF, 0xFF]); // Extra data

        let result = SmhdBox::parse(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn parse_smhd_truncated() {
        let payload = make_smhd_payload(0, 0, 0);
        let truncated = &payload[..payload.len() - 1];

        let result = SmhdBox::parse(truncated);
        assert!(result.is_err());
    }

    #[test]
    fn smhd_default() {
        let smhd = SmhdBox::default();

        assert_eq!(smhd.version, 0);
        assert_eq!(smhd.flags.get(), 0);
        assert_eq!(smhd.balance.to_raw(), 0);
    }
}
