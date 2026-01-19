//! BMFF box header types and parsing.

use core::fmt;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;
use crate::types::FourCC;

use crate::error::*;

pub mod boxsize;
pub mod boxtype;
pub mod fullbox;

pub use boxsize::BoxSize;
pub use boxtype::BoxType;
pub use fullbox::FullBoxFlags;

/// Represents the header of a BMFF box, including its size and type.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoxHeader {
    size: BoxSize,
    type_: BoxType,
}

impl BoxHeader {
    /// Base size of a box header (size + type fields).
    const BASE_SIZE: u64 = 8;

    /// Creates a new box header.
    pub const fn new(size: BoxSize, type_: BoxType) -> Self {
        Self { size, type_ }
    }

    /// Returns the size of the box header in bytes.
    pub fn header_size(&self) -> u64 {
        let base = Self::BASE_SIZE;

        // Add largesize field size if needed
        let extened = if self.size.is_extended() { 8 } else { 0 };

        // Add UUID field size if needed
        let uuid = if self.type_.is_uuid() { 16 } else { 0 };

        base + extened + uuid
    }

    /// Returns the payload size of the box, or None if the size extends to the end of the file.
    pub fn payload_size(&self) -> Option<u64> {
        self.size.value()?.checked_sub(self.header_size())
    }

    /// Returns the box size.
    pub fn boxsize(&self) -> BoxSize {
        self.size
    }

    /// Returns the box type.
    pub fn boxtype(&self) -> BoxType {
        self.type_
    }

    /// Parses a `BoxHeader` from the given `ReadCursor`.
    pub fn parse(cur: &mut ReadCursor<'_>) -> Result<Self> {
        if cur.remaining() < Self::BASE_SIZE as usize {
            return Err(Error::at(
                ErrorKind::NotEnoughBytes {
                    expected: Self::BASE_SIZE as usize,
                    remaining: cur.remaining(),
                },
                cur.position() as u64,
            ));
        }

        // Read size (4 bytes)
        let size = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let type_bytes = cur
            .read_array::<4>()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let boxtype = FourCC::from(type_bytes);

        let boxsize = match size {
            0 => BoxSize::eof(),
            1 => {
                // Read largesize (8 bytes)
                let largesize = cur
                    .read_u64_be()
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
                BoxSize::from_u64(largesize)
                    .map_err(|e| Error::at(e.into(), cur.position() as u64))?
            }
            _ => BoxSize::from_u32(size).map_err(|e| Error::at(e.into(), cur.position() as u64))?,
        };

        let boxtype = if boxtype == boxtype::UUID {
            // Read usertype (16 bytes)
            let usertype_bytes = cur
                .read_array::<16>()
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
            let usertype = boxtype::UserType::new(usertype_bytes);
            BoxType::from_uuid(usertype)
        } else {
            BoxType::from_fourcc(boxtype).map_err(|e| Error::at(e.into(), cur.position() as u64))?
        };

        Ok(BoxHeader {
            size: boxsize,
            type_: boxtype,
        })
    }

    /// Writes the `BoxHeader` to the given `WriteCursor`.
    pub fn write(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
        if self.size.is_eof() {
            // Write size field for EOF (4 bytes)
            cur.write_u32_be(BoxSize::MARKER_EOF)
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
            // Write type field (4 bytes)
            cur.write_array(self.boxtype().type_field().as_bytes())
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        } else if self.size.is_extended() {
            // Write size field (4 bytes)
            cur.write_u32_be(BoxSize::MARKER_EXTENDED_SIZE)
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
            // Write type field (4 bytes)
            cur.write_array(self.boxtype().type_field().as_bytes())
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
            // Write largesize field (8 bytes)
            cur.write_u64_be(self.boxsize().value().expect("extended size has value"))
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        } else {
            // Write size field (4 bytes) - compact size
            cur.write_u32_be(self.boxsize().value().expect("compact size has value") as u32)
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
            // Write type field (4 bytes)
            cur.write_array(self.boxtype().type_field().as_bytes())
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        }

        // Write usertype if UUID type (16 bytes)
        if self.boxtype().is_uuid() {
            cur.write_array(
                self.boxtype()
                    .user_type()
                    .expect("BoxType UserType")
                    .as_bytes(),
            )
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        }

        Ok(())
    }
}

impl fmt::Debug for BoxHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BoxHeader")
            .field("size", &self.size)
            .field("type", &self.type_)
            .finish()
    }
}

impl fmt::Display for BoxHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (size: {})", self.type_, self.size)
    }
}

/// Represents the header of a FullBox, including version and flags.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FullBoxHeader<B> {
    version: u8,
    flags: FullBoxFlags<B>,
}

impl<B> FullBoxHeader<B> {
    /// Creates a new `FullBoxHeader` with the given version and flags.
    pub const fn new(version: u8, flags: FullBoxFlags<B>) -> Self {
        Self { version, flags }
    }

    /// Returns the version of the FullBox.
    pub fn version(&self) -> u8 {
        self.version
    }

    /// Returns the flags of the FullBox.
    pub fn flags(&self) -> FullBoxFlags<B> {
        self.flags
    }

    /// Parses a `FullBoxHeader` from the given `ReadCursor`.
    pub(crate) fn parse_in(cur: &mut ReadCursor<'_>) -> Result<Self> {
        if cur.remaining() < 4 {
            return Err(Error::at(
                ErrorKind::NotEnoughBytes {
                    expected: 4,
                    remaining: cur.remaining(),
                },
                cur.position() as u64,
            ));
        }

        // Read version (1 byte)
        let version = cur
            .read_u8()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        // Read flags (3 bytes)
        let flags_bytes = cur
            .read_array::<3>()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let flags_value = ((flags_bytes[0] as u32) << 16)
            | ((flags_bytes[1] as u32) << 8)
            | (flags_bytes[2] as u32);
        let flags = FullBoxFlags::new(flags_value);

        Ok(Self { version, flags })
    }

    /// Writes the `FullBoxHeader` to the given `WriteCursor`.
    pub fn write(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
        cur.write_u8(self.version)
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

        let flags = self.flags.get() & 0x00FF_FFFF;
        let flags_bytes = [
            ((flags >> 16) & 0xFF) as u8,
            ((flags >> 8) & 0xFF) as u8,
            (flags & 0xFF) as u8,
        ];

        cur.write_array(&flags_bytes)
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FourCC;
    use crate::types::Uuid;

    #[test]
    fn header_size_calculation() {
        let header = BoxHeader::new(
            BoxSize::from_u32(20).unwrap(),
            BoxType::from_fourcc(FourCC::from(*b"moov")).unwrap(),
        );
        assert_eq!(header.header_size(), 8);
        assert_eq!(header.payload_size(), Some(12));

        let header = BoxHeader::new(
            BoxSize::from_u64(u64::MAX).unwrap(),
            BoxType::from_uuid(Uuid::new([0x01; 16])),
        );
        assert_eq!(header.header_size(), 32);
        assert_eq!(header.payload_size(), Some(u64::MAX - 32));

        let header = BoxHeader::new(
            BoxSize::eof(),
            BoxType::from_fourcc(FourCC::from(*b"free")).unwrap(),
        );
        assert_eq!(header.header_size(), 8);
        assert_eq!(header.payload_size(), None);
    }

    #[test]
    fn parse_32bit_size() {
        // size=20 (4 bytes) + type="ftyp" (4 bytes) + payload (12 bytes)
        let data = [
            0x00, 0x00, 0x00, 0x14, // size: 20
            b'f', b't', b'y', b'p', // type: ftyp
            0x00, 0x00, 0x00, 0x00, // payload (12 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut cur = ReadCursor::new(&data);
        let header = BoxHeader::parse(&mut cur).unwrap();

        assert_eq!(header.boxsize().value(), Some(20));
        assert!(!header.boxsize().is_extended());
        assert_eq!(header.boxtype().type_field(), FourCC::from(*b"ftyp"));
        assert!(!header.boxtype().is_uuid());
        assert_eq!(header.header_size(), 8);
        assert_eq!(header.payload_size(), Some(12));
        assert_eq!(cur.position(), 8);
    }

    #[test]
    fn parse_64bit_extended_size_large_value() {
        // largesize = 0x1_0000_0020
        let data = [
            0x00, 0x00, 0x00, 0x01, // size: 1 (extended size marker)
            b'm', b'd', b'a', b't', // type: mdat
            0x00, 0x00, 0x00, 0x01, // largesize high 32 bits: 1
            0x00, 0x00, 0x00, 0x20, // largesize low 32 bits: 32 → total: 0x1_0000_0020
        ];

        let mut cur = ReadCursor::new(&data);
        let header = BoxHeader::parse(&mut cur).unwrap();

        assert_eq!(header.boxsize().value(), Some(0x1_0000_0020));
        assert!(header.boxsize().is_extended());
        assert_eq!(header.boxtype().type_field(), FourCC::from(*b"mdat"));
        assert_eq!(header.header_size(), 16);
        assert_eq!(header.payload_size(), Some(0x1_0000_0020 - 16));
        assert_eq!(cur.position(), 16);
    }

    #[test]
    fn parse_64bit_extended_size_small_value() {
        // size=1 marker with largesize=24
        // Now is_extended() correctly returns true because we preserve the encoding format
        let data = [
            0x00, 0x00, 0x00, 0x01, // size: 1 (extended size marker)
            b'm', b'd', b'a', b't', // type: mdat
            0x00, 0x00, 0x00, 0x00, // largesize high 32 bits: 0
            0x00, 0x00, 0x00, 0x18, // largesize low 32 bits: 24
        ];

        let mut cur = ReadCursor::new(&data);
        let header = BoxHeader::parse(&mut cur).unwrap();

        assert_eq!(header.boxsize().value(), Some(24));
        assert!(header.boxsize().is_extended()); // Now correctly true
        assert_eq!(header.header_size(), 16);
        assert_eq!(header.payload_size(), Some(8)); // 24 - 16 = 8
        assert_eq!(cur.position(), 16);
    }

    #[test]
    fn parse_uuid_box() {
        // size=32 + type="uuid" + usertype (16 bytes)
        let uuid_bytes: [u8; 16] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
            0x0F, 0x10,
        ];
        let mut data = vec![
            0x00, 0x00, 0x00, 0x20, // size: 32
            b'u', b'u', b'i', b'd', // type: uuid
        ];
        data.extend_from_slice(&uuid_bytes);
        data.extend_from_slice(&[0x00; 8]); // payload (8 bytes)

        let mut cur = ReadCursor::new(&data);
        let header = BoxHeader::parse(&mut cur).unwrap();

        assert_eq!(header.boxsize().value(), Some(32));
        assert!(!header.boxsize().is_extended());
        assert!(header.boxtype().is_uuid());
        assert_eq!(header.boxtype().user_type(), Some(Uuid::new(uuid_bytes)));
        assert_eq!(header.header_size(), 24);
        assert_eq!(header.payload_size(), Some(8));
        assert_eq!(cur.position(), 24);
    }

    #[test]
    fn parse_eof_size() {
        // size=0 means box extends to EOF
        let data = [
            0x00, 0x00, 0x00, 0x00, // size: 0 (to end of file)
            b'f', b'r', b'e', b'e', // type: free
        ];

        let mut cur = ReadCursor::new(&data);
        let header = BoxHeader::parse(&mut cur).unwrap();

        assert!(header.boxsize().is_eof());
        assert_eq!(header.boxtype().type_field(), FourCC::from(*b"free"));
        assert_eq!(header.header_size(), 8);
        assert_eq!(header.payload_size(), None);
    }

    #[test]
    fn parse_insufficient_data() {
        // Only 4 bytes, need at least 8
        let data = [0x00, 0x00, 0x00, 0x08];

        let mut cur = ReadCursor::new(&data);
        let result = BoxHeader::parse(&mut cur);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err.kind(),
            ErrorKind::NotEnoughBytes {
                expected: 8,
                remaining: 4
            }
        ));
    }

    #[test]
    fn write_roundtrip_32bit() {
        let original = BoxHeader::new(
            BoxSize::from_u32(100).unwrap(),
            BoxType::from_fourcc(FourCC::from(*b"moov")).unwrap(),
        );

        let mut buf = [0u8; 8];
        let mut write_cur = WriteCursor::new(&mut buf);
        original.write(&mut write_cur).unwrap();

        let mut read_cur = ReadCursor::new(&buf);
        let parsed = BoxHeader::parse(&mut read_cur).unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn write_roundtrip_64bit() {
        let original = BoxHeader::new(
            BoxSize::from_u64(u64::MAX).unwrap(),
            BoxType::from_fourcc(FourCC::from(*b"mdat")).unwrap(),
        );

        let mut buf = [0u8; 16];
        let mut write_cur = WriteCursor::new(&mut buf);
        original.write(&mut write_cur).unwrap();

        let mut read_cur = ReadCursor::new(&buf);
        let parsed = BoxHeader::parse(&mut read_cur).unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn write_roundtrip_uuid() {
        let uuid = Uuid::new([0xAB; 16]);
        let original = BoxHeader::new(BoxSize::from_u32(100).unwrap(), BoxType::from_uuid(uuid));

        let mut buf = [0u8; 24];
        let mut write_cur = WriteCursor::new(&mut buf);
        original.write(&mut write_cur).unwrap();

        let mut read_cur = ReadCursor::new(&buf);
        let parsed = BoxHeader::parse(&mut read_cur).unwrap();

        assert_eq!(original, parsed);
    }

    // FullBoxHeader tests

    #[derive(Clone, Copy, Debug)]
    struct TestFullBox;

    #[test]
    fn fullbox_header_parse() {
        // version=1, flags=0x000102
        let data = [0x01, 0x00, 0x01, 0x02];
        let mut cur = ReadCursor::new(&data);

        let header: FullBoxHeader<TestFullBox> = FullBoxHeader::parse_in(&mut cur).unwrap();
        assert_eq!(header.version(), 1);
        assert_eq!(header.flags().get(), 0x000102);
        assert_eq!(cur.position(), 4);
    }

    #[test]
    fn fullbox_header_write_roundtrip() {
        let original: FullBoxHeader<TestFullBox> =
            FullBoxHeader::new(2, FullBoxFlags::new(0x123456));

        let mut buf = [0u8; 4];
        let mut write_cur = WriteCursor::new(&mut buf);
        original.write(&mut write_cur).unwrap();

        let mut read_cur = ReadCursor::new(&buf);
        let parsed: FullBoxHeader<TestFullBox> = FullBoxHeader::parse_in(&mut read_cur).unwrap();

        assert_eq!(parsed.version(), original.version());
        assert_eq!(parsed.flags().get(), original.flags().get());
    }

    #[test]
    fn fullbox_header_insufficient_data() {
        let data = [0x01, 0x00, 0x01]; // 3 bytes, need 4
        let mut cur = ReadCursor::new(&data);

        let result: Result<FullBoxHeader<TestFullBox>> = FullBoxHeader::parse_in(&mut cur);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().kind(),
            ErrorKind::NotEnoughBytes {
                expected: 4,
                remaining: 3
            }
        ));
    }
}
