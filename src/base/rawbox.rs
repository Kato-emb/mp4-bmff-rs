//! Raw BMFF box representation.

use crate::BoxHeader;
use crate::BoxSize;
use crate::BoxType;
use crate::error::*;

#[cfg(feature = "alloc")]
use crate::lib::Vec;

/// A raw BMFF box with its header and payload.
#[derive(Debug)]
pub struct RawBox<T> {
    header: BoxHeader,
    payload: T,
}

/// A reference to a RawBox's contents.
pub type RawBoxRef<'a> = RawBox<&'a [u8]>;

/// A owned RawBox with a `Vec<u8>` payload.
#[cfg(feature = "alloc")]
pub type RawBoxOwned = RawBox<Vec<u8>>;

impl<T: Clone> Clone for RawBox<T> {
    fn clone(&self) -> Self {
        Self {
            header: self.header,
            payload: self.payload.clone(),
        }
    }
}

impl<T> RawBox<T> {
    #[cfg(feature = "std")]
    pub(crate) fn from_parts(header: BoxHeader, payload: T) -> Self {
        Self { header, payload }
    }

    /// Returns the box header.
    #[inline]
    pub fn header(&self) -> BoxHeader {
        self.header
    }

    /// Returns the box size.
    #[inline]
    pub fn boxsize(&self) -> BoxSize {
        self.header.boxsize()
    }

    /// Returns the box type.
    #[inline]
    pub fn boxtype(&self) -> BoxType {
        self.header.boxtype()
    }

    /// Consumes the `RawBox`, returning the payload.
    pub fn into_payload(self) -> T {
        self.payload
    }
}

impl<T: AsRef<[u8]>> RawBox<T> {
    /// Creates a new `RawBox` with the given box type and payload.
    pub fn new(boxtype: BoxType, payload: T) -> Self {
        let payload_len = payload.as_ref().len() as u64;
        let header = BoxHeader::new(boxtype, payload_len);

        Self { header, payload }
    }

    /// Writes the `RawBox` into the given byte slice.
    pub fn write(&self, bytes: &mut [u8]) -> Result<()> {
        let header_len = self.header.header_len();
        let payload = self.payload.as_ref();
        let total_size = header_len + payload.len();

        if bytes.len() < total_size {
            return Err(Error::in_box(
                ErrorKind::NotEnoughBytes {
                    expected: total_size,
                    remaining: bytes.len(),
                },
                self.header.boxtype(),
            ));
        }

        // write header
        self.header.write(&mut bytes[..header_len])?;

        // write payload
        bytes[header_len..total_size].copy_from_slice(payload);

        Ok(())
    }

    /// Returns the total length of the box (header + payload).
    #[inline]
    pub fn len(&self) -> usize {
        let payload_len = self.payload.as_ref().len();
        self.header.header_len() + payload_len
    }

    /// Returns true if the box is empty (has zero length).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the box payload.
    pub fn payload(&self) -> &[u8] {
        self.payload.as_ref()
    }
}

impl<T: AsMut<[u8]>> RawBox<T> {
    /// Returns the mutable box payload.
    pub fn payload_mut(&mut self) -> &mut [u8] {
        self.payload.as_mut()
    }
}

impl<'a> RawBox<&'a [u8]> {
    /// Converts the `RawBoxRef` into an owned `RawBox` with a `Vec<u8>` payload.
    #[cfg(feature = "alloc")]
    pub fn to_owned(&self) -> RawBox<Vec<u8>> {
        RawBox {
            header: self.header,
            payload: self.payload.to_vec(),
        }
    }

    /// Parses a `RawBoxRef` from the given byte slice.
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let header = BoxHeader::parse(bytes)?;
        let header_len = header.header_len();

        let payload_size = match header.boxsize() {
            size if size.is_eof() => bytes.len().saturating_sub(header_len),
            size => {
                let box_size = size.value().expect("box size is valid");

                if box_size > usize::MAX as u64 {
                    return Err(Error::in_box(
                        ErrorKind::InvalidBoxSize {
                            reason: "Box size exceeds usize max",
                            got: box_size,
                        },
                        header.boxtype(),
                    ));
                }

                let total_size = box_size as usize;

                if total_size < header_len {
                    return Err(Error::in_box(
                        ErrorKind::InvalidBoxSize {
                            reason: "Box size is smaller than header size",
                            got: box_size,
                        },
                        header.boxtype(),
                    ));
                }

                total_size - header_len
            }
        };

        let total_needed = header_len + payload_size;
        if bytes.len() < total_needed {
            return Err(Error::in_box(
                ErrorKind::NotEnoughBytes {
                    expected: total_needed,
                    remaining: bytes.len(),
                },
                header.boxtype(),
            ));
        }

        let payload = &bytes[header_len..total_needed];
        Ok(RawBox { header, payload })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FourCC;

    #[test]
    fn parse_compact_box() {
        // size=12 (header 8 + payload 4)
        let data = [
            0x00, 0x00, 0x00, 0x0C, // size = 12
            b'f', b't', b'y', b'p', // type = 'ftyp'
            0x01, 0x02, 0x03, 0x04, // payload
        ];

        let raw = RawBoxRef::parse(&data).unwrap();
        assert_eq!(raw.boxtype().type_field(), FourCC::from(*b"ftyp"));
        assert_eq!(raw.payload(), &[0x01, 0x02, 0x03, 0x04]);
        assert_eq!(raw.len(), 12);
    }

    #[test]
    fn parse_eof_box() {
        // size=0 means EOF (consume all remaining)
        let data = [
            0x00, 0x00, 0x00, 0x00, // size = 0 (EOF)
            b'm', b'd', b'a', b't', // type = 'mdat'
            0xDE, 0xAD, 0xBE, 0xEF, // payload
        ];

        let raw = RawBoxRef::parse(&data).unwrap();
        assert_eq!(raw.boxtype().type_field(), FourCC::from(*b"mdat"));
        assert!(raw.boxsize().is_eof());
        assert_eq!(raw.payload(), &[0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn parse_extended_size_box() {
        // size=1 signals extended size, largesize=24 (header 16 + payload 8)
        let data = [
            0x00, 0x00, 0x00, 0x01, // size = 1 (extended size)
            b'm', b'd', b'a', b't', // type = 'mdat'
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, // largesize = 24
            0x01, 0x02, 0x03, 0x04, // payload
            0x05, 0x06, 0x07, 0x08,
        ];

        let raw = RawBoxRef::parse(&data).unwrap();
        assert!(raw.boxsize().is_extended());
        assert_eq!(raw.payload().len(), 8);
    }

    #[test]
    fn parse_header_only_box() {
        // size=8, no payload
        let data = [
            0x00, 0x00, 0x00, 0x08, // size = 8
            b'f', b'r', b'e', b'e', // type = 'free'
        ];

        let raw = RawBoxRef::parse(&data).unwrap();
        assert_eq!(raw.payload().len(), 0);
        assert!(!raw.is_empty()); // len() returns box size (8), not payload size
    }

    #[test]
    fn parse_truncated_payload() {
        // size=16 but only 10 bytes provided
        let data = [
            0x00, 0x00, 0x00, 0x10, // size = 16
            b'f', b't', b'y', b'p', // type = 'ftyp'
            0x01, 0x02, 0x03, 0x04, // incomplete payload
        ];

        let result = RawBoxRef::parse(&data);
        assert!(result.is_err());
    }
}
