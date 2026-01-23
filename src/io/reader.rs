//!

use std::error::Error;
use std::io::Read;

use crate::BoxHeader;
use crate::base::rawbox::RawBoxOwned;
use crate::error::Result;

/// I/O utilities for BMFF parsing and writing.
pub struct BoxReader<R> {
    inner: R,
}

impl<R> BoxReader<R> {
    /// Creates a new `BoxReader` from the given reader.
    pub fn new(inner: R) -> Self {
        BoxReader { inner }
    }

    /// Returns a reference to the inner reader.
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Returns a mutable reference to the inner reader.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Consumes the `BoxReader`, returning the inner reader.
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R: Read> BoxReader<R> {
    fn read_header(&mut self) -> Result<BoxHeader> {
        let mut buf = [0u8; BoxHeader::BASE_SIZE];
        self.inner.read_exact(&mut buf)?;

        let additional = BoxHeader::addintional_header_bytes(&buf);

        let header_len = BoxHeader::BASE_SIZE + additional;
        let mut full_buf = vec![0u8; header_len];
        full_buf[..BoxHeader::BASE_SIZE].copy_from_slice(&buf);
        if additional > 0 {
            self.inner
                .read_exact(&mut full_buf[BoxHeader::BASE_SIZE..])?;
        }

        BoxHeader::parse(&full_buf)
    }

    /// Reads a `RawBoxOwned` from the inner reader.
    pub fn read_box(&mut self) -> Result<RawBoxOwned> {
        let header = self.read_header()?;

        if header.boxsize().is_eof() {
            let mut payload = Vec::new();
            self.inner.read_to_end(&mut payload)?;
            Ok(RawBoxOwned::from_parts(header, payload))
        } else {
            let payload_len = header.total_size() - header.header_len() as u64;
            let mut payload = vec![0u8; payload_len as usize];
            self.inner.read_exact(&mut payload)?;
            Ok(RawBoxOwned::from_parts(header, payload))
        }
    }
}

impl<R: Read> Iterator for BoxReader<R> {
    type Item = Result<RawBoxOwned>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.read_box() {
            Ok(boxed) => Some(Ok(boxed)),
            Err(e) => {
                // If we reached EOF, return None to end the iteration.
                if let crate::error::ErrorKind::Io = e.kind() {
                    if let Some(source) = e
                        .source()
                        .and_then(|src| src.downcast_ref::<std::io::Error>())
                    {
                        if source.kind() == std::io::ErrorKind::UnexpectedEof {
                            return None;
                        }
                    }
                }
                Some(Err(e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    use crate::BoxType;
    use crate::types::FourCC;

    #[test]
    fn accessors() {
        let data = vec![1u8, 2, 3];
        let cursor = Cursor::new(data.clone());
        let mut reader = BoxReader::new(cursor);

        assert_eq!(reader.get_ref().position(), 0);
        reader.get_mut().set_position(1);
        assert_eq!(reader.get_ref().position(), 1);

        let inner = reader.into_inner();
        assert_eq!(inner.into_inner(), data);
    }

    #[test]
    fn read_compact_box() {
        // size=12 (header 8 + payload 4)
        let data = vec![
            0x00, 0x00, 0x00, 0x0C, // size = 12
            b'f', b't', b'y', b'p', // type = 'ftyp'
            0x01, 0x02, 0x03, 0x04, // payload
        ];

        let mut reader = BoxReader::new(Cursor::new(data));
        let raw = reader.read_box().unwrap();

        assert_eq!(raw.boxtype().type_field(), FourCC::from(*b"ftyp"));
        assert_eq!(raw.payload(), &[0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn read_eof_box() {
        // size=0 means EOF (consume all remaining)
        let data = vec![
            0x00, 0x00, 0x00, 0x00, // size = 0 (EOF)
            b'm', b'd', b'a', b't', // type = 'mdat'
            0xDE, 0xAD, 0xBE, 0xEF, // payload
        ];

        let mut reader = BoxReader::new(Cursor::new(data));
        let raw = reader.read_box().unwrap();

        assert!(raw.boxsize().is_eof());
        assert_eq!(raw.payload(), &[0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn read_extended_size_box() {
        // size=1 signals extended size, largesize=24 (header 16 + payload 8)
        let data = vec![
            0x00, 0x00, 0x00, 0x01, // size = 1 (extended size)
            b'm', b'd', b'a', b't', // type = 'mdat'
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, // largesize = 24
            0x01, 0x02, 0x03, 0x04, // payload
            0x05, 0x06, 0x07, 0x08,
        ];

        let mut reader = BoxReader::new(Cursor::new(data));
        let raw = reader.read_box().unwrap();

        assert!(raw.boxsize().is_extended());
        assert_eq!(raw.payload().len(), 8);
    }

    #[test]
    fn iterator_reads_multiple_boxes() {
        // Two boxes: ftyp (12 bytes) + free (8 bytes)
        let data = vec![
            // Box 1: ftyp
            0x00, 0x00, 0x00, 0x0C, // size = 12
            b'f', b't', b'y', b'p', // type
            0x01, 0x02, 0x03, 0x04, // payload
            // Box 2: free (header only)
            0x00, 0x00, 0x00, 0x08, // size = 8
            b'f', b'r', b'e', b'e', // type
        ];

        let reader = BoxReader::new(Cursor::new(data));
        let boxes: Vec<_> = reader.collect();

        assert_eq!(boxes.len(), 2);
        assert_eq!(boxes[0].as_ref().unwrap().boxtype(), BoxType::FTYP);
        assert_eq!(boxes[1].as_ref().unwrap().boxtype(), BoxType::FREE);
    }

    #[test]
    fn iterator_ends_on_eof() {
        let data = vec![];
        let reader = BoxReader::new(Cursor::new(data));
        let boxes: Vec<_> = reader.collect();

        assert!(boxes.is_empty());
    }
}
