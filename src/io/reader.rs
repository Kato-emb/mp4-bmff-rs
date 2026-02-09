//! BMFF box reader for stream-based I/O.
//!
//! This module provides [`BoxReader`] for reading BMFF boxes from any
//! type implementing [`std::io::Read`].

use std::io::*;

use crate::error::Result;
use crate::{
    BoxHeader, //
    RawBoxOwned,
};

/// A reader for BMFF boxes from a stream.
///
/// `BoxReader` wraps any [`Read`] implementor and provides
/// methods to read boxes sequentially. It handles compact boxes, extended
/// size boxes, and EOF boxes (size = 0) automatically.
///
/// # Example
///
/// ```
/// use std::io::Cursor;
/// use mp4_bmff::io::BoxReader;
///
/// // Create a minimal BMFF stream with one box
/// let data = vec![
///     0x00, 0x00, 0x00, 0x0C, // size = 12
///     b'f', b't', b'y', b'p', // type = "ftyp"
///     0x69, 0x73, 0x6F, 0x6D, // payload: "isom"
/// ];
///
/// let mut reader = BoxReader::new(Cursor::new(data));
/// let raw_box = reader.read_box().unwrap();
///
/// assert_eq!(raw_box.payload(), b"isom");
/// ```
pub struct BoxReader<R> {
    inner: R,
    pending: Option<BoxHeader>,
}

impl<R> BoxReader<R> {
    /// Creates a new `BoxReader` wrapping the given reader.
    ///
    /// # Arguments
    ///
    /// * `inner` - Any type implementing [`Read`]
    pub fn new(inner: R) -> Self {
        BoxReader {
            inner,
            pending: None,
        }
    }

    /// Returns a reference to the underlying reader.
    #[inline]
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Returns a mutable reference to the underlying reader.
    ///
    /// Use this to access reader-specific functionality like seeking.
    #[inline]
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Consumes the `BoxReader`, returning the underlying reader.
    #[inline]
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R: Read> BoxReader<R> {
    /// Peeks at the next box header without consuming the payload.                                                                                                         
    pub fn peek_header(&mut self) -> Result<&BoxHeader> {
        if self.pending.is_none() {
            self.pending = Some(self.read_header_in()?);
        }

        // SAFETY: We just ensured pending is Some
        Ok(self.pending.as_ref().unwrap())
    }

    /// Reads the next box from the stream.
    ///
    /// This method reads the box header first to determine the box size,
    /// then reads the payload. For EOF boxes (size = 0), it reads all
    /// remaining data from the stream.
    ///
    /// # Returns
    ///
    /// The parsed box as a [`RawBoxOwned`] on success.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The stream doesn't contain enough data for the header
    /// - The stream doesn't contain enough data for the declared payload size
    /// - An I/O error occurs
    pub fn read_box(&mut self) -> Result<RawBoxOwned> {
        let header = self.next_header_in()?;

        let payload = if header.boxsize().is_eof() {
            let mut payload = Vec::new();
            self.inner.read_to_end(&mut payload)?;
            payload
        } else {
            let payload_len = header.total_size() - header.header_len() as u64;
            let mut payload = vec![0u8; payload_len as usize];
            self.inner.read_exact(&mut payload)?;
            payload
        };

        Ok(RawBoxOwned::from_parts(header, payload))
    }

    fn read_header_in(&mut self) -> Result<BoxHeader> {
        let mut buf = [0u8; BoxHeader::MAX_HEADER_SIZE];
        self.inner.read_exact(&mut buf[..BoxHeader::BASE_SIZE])?;

        let additional =
            BoxHeader::additional_header_bytes(&buf[..BoxHeader::BASE_SIZE].try_into().unwrap());
        let header_len = BoxHeader::BASE_SIZE + additional;

        if additional > 0 {
            self.inner
                .read_exact(&mut buf[BoxHeader::BASE_SIZE..header_len])?;
        }

        BoxHeader::parse(&buf[..header_len])
    }

    fn next_header_in(&mut self) -> Result<BoxHeader> {
        if let Some(header) = self.pending.take() {
            Ok(header)
        } else {
            self.read_header_in()
        }
    }
}

impl<R: Read + Seek> BoxReader<R> {
    /// Skips the next box in the stream.
    ///
    /// This method reads the box header to determine the box size,
    /// then seeks past the payload. For EOF boxes (size = 0),
    /// it seeks to the end of the stream.
    ///
    /// # Returns
    ///
    /// The skipped box header on success.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The stream doesn't contain enough data for the header
    /// - An I/O error occurs
    ///
    /// # Example
    /// ```
    /// use std::io::Cursor;
    /// use mp4_bmff::io::BoxReader;
    ///
    /// let data = vec![
    ///     0x00, 0x00, 0x00, 0x0C, // size = 12
    ///     b'f', b't', b'y', b'p', // type = "ftyp"
    ///     0x69, 0x73, 0x6F, 0x6D, // payload: "isom"
    ///     0x00, 0x00, 0x00, 0x08, // size = 8
    ///     b'f', b'r', b'e', b'e', // type = "free"
    /// ];
    ///
    /// let mut reader = BoxReader::new(Cursor::new(data));
    /// reader.skip_box().unwrap(); // Skip 'ftyp' box
    /// let raw_box = reader.read_box().unwrap(); // Read 'free' box
    ///
    /// assert_eq!(raw_box.boxtype().type_field().to_string(), "free");
    /// ```
    pub fn skip_box(&mut self) -> Result<BoxHeader> {
        let header = self.next_header_in()?;

        if header.boxsize().is_eof() {
            // EOF box: seek to end
            self.inner.seek(SeekFrom::End(0))?;
        } else {
            let payload_len = header.total_size() - header.header_len() as u64;
            self.inner.seek(SeekFrom::Current(payload_len as i64))?;
        }

        Ok(header)
    }

    /// Returns the current stream position.
    ///
    /// # Returns
    ///
    /// The current position in the stream.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs.
    pub fn stream_position(&mut self) -> Result<u64> {
        let pos = self.inner.stream_position()?;
        Ok(pos)
    }

    /// Rewinds the stream to the beginning.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs.
    pub fn rewind(&mut self) -> Result<()> {
        self.inner.rewind()?;
        Ok(())
    }
}

impl<T> From<T> for BoxReader<T>
where
    T: Read,
{
    fn from(reader: T) -> Self {
        BoxReader::new(reader)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

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
    fn peek_header() {
        let data = vec![
            0x00, 0x00, 0x00, 0x0C, // size = 12
            b'f', b't', b'y', b'p', // type = 'ftyp'
            0x01, 0x02, 0x03, 0x04, // payload
        ];

        let mut reader = BoxReader::new(Cursor::new(data));

        // First peek
        let header1 = reader.peek_header().unwrap();
        assert_eq!(header1.boxtype().type_field(), FourCC::from(*b"ftyp"));
        assert_eq!(header1.total_size(), 12);

        // Second peek should return the same header
        let header2 = reader.peek_header().unwrap();
        assert_eq!(header2.boxtype().type_field(), FourCC::from(*b"ftyp"));

        // Reading should consume the peeked header
        let raw = reader.read_box().unwrap();
        assert_eq!(raw.boxtype().type_field(), FourCC::from(*b"ftyp"));
    }

    #[test]
    fn skip_box() {
        let data = vec![
            0x00, 0x00, 0x00, 0x0C, // size = 12
            b'f', b't', b'y', b'p', // type = 'ftyp'
            0x69, 0x73, 0x6F, 0x6D, // payload: "isom"
            0x00, 0x00, 0x00, 0x08, // size = 8
            b'f', b'r', b'e', b'e', // type = 'free'
        ];

        let mut reader = BoxReader::new(Cursor::new(data));

        // Skip first box
        let skipped = reader.skip_box().unwrap();
        assert_eq!(skipped.boxtype().type_field(), FourCC::from(*b"ftyp"));

        // Read second box
        let raw = reader.read_box().unwrap();
        assert_eq!(raw.boxtype().type_field(), FourCC::from(*b"free"));
    }

    #[test]
    fn skip_eof_box() {
        let data = vec![
            0x00, 0x00, 0x00, 0x00, // size = 0 (EOF)
            b'm', b'd', b'a', b't', // type = 'mdat'
            0xDE, 0xAD, 0xBE, 0xEF, // payload
        ];

        let mut reader = BoxReader::new(Cursor::new(data));

        let skipped = reader.skip_box().unwrap();
        assert!(skipped.boxsize().is_eof());

        // Stream should be at end
        assert_eq!(reader.stream_position().unwrap(), 12);
    }

    #[test]
    fn stream_position_and_rewind() {
        let data = vec![
            0x00, 0x00, 0x00, 0x0C, // size = 12
            b'f', b't', b'y', b'p', // type = 'ftyp'
            0x01, 0x02, 0x03, 0x04, // payload
        ];

        let mut reader = BoxReader::new(Cursor::new(data));

        assert_eq!(reader.stream_position().unwrap(), 0);

        reader.read_box().unwrap();
        assert_eq!(reader.stream_position().unwrap(), 12);

        reader.rewind().unwrap();
        assert_eq!(reader.stream_position().unwrap(), 0);

        // Can read again after rewind
        let raw = reader.read_box().unwrap();
        assert_eq!(raw.boxtype().type_field(), FourCC::from(*b"ftyp"));
    }

    #[test]
    fn from_trait() {
        let data = vec![
            0x00, 0x00, 0x00, 0x0C, // size = 12
            b'f', b't', b'y', b'p', // type = 'ftyp'
            0x01, 0x02, 0x03, 0x04, // payload
        ];

        let cursor = Cursor::new(data);
        let mut reader: BoxReader<_> = cursor.into();

        let raw = reader.read_box().unwrap();
        assert_eq!(raw.boxtype().type_field(), FourCC::from(*b"ftyp"));
    }

    #[test]
    fn read_multiple_boxes() {
        let data = vec![
            // First box: ftyp
            0x00, 0x00, 0x00, 0x0C, // size = 12
            b'f', b't', b'y', b'p', // type = 'ftyp'
            0x01, 0x02, 0x03, 0x04, // payload
            // Second box: free
            0x00, 0x00, 0x00, 0x08, // size = 8
            b'f', b'r', b'e', b'e', // type = 'free'
            // Third box: mdat
            0x00, 0x00, 0x00, 0x0A, // size = 10
            b'm', b'd', b'a', b't', // type = 'mdat'
            0xAA, 0xBB, // payload
        ];

        let mut reader = BoxReader::new(Cursor::new(data));

        let box1 = reader.read_box().unwrap();
        assert_eq!(box1.boxtype().type_field(), FourCC::from(*b"ftyp"));
        assert_eq!(box1.payload(), &[0x01, 0x02, 0x03, 0x04]);

        let box2 = reader.read_box().unwrap();
        assert_eq!(box2.boxtype().type_field(), FourCC::from(*b"free"));
        assert!(box2.payload().is_empty());

        let box3 = reader.read_box().unwrap();
        assert_eq!(box3.boxtype().type_field(), FourCC::from(*b"mdat"));
        assert_eq!(box3.payload(), &[0xAA, 0xBB]);
    }

    #[test]
    fn peek_then_skip() {
        let data = vec![
            0x00, 0x00, 0x00, 0x0C, // size = 12
            b'f', b't', b'y', b'p', // type = 'ftyp'
            0x01, 0x02, 0x03, 0x04, // payload
            0x00, 0x00, 0x00, 0x08, // size = 8
            b'f', b'r', b'e', b'e', // type = 'free'
        ];

        let mut reader = BoxReader::new(Cursor::new(data));

        // Peek first
        let header = reader.peek_header().unwrap();
        assert_eq!(header.boxtype().type_field(), FourCC::from(*b"ftyp"));

        // Skip should use the peeked header
        let skipped = reader.skip_box().unwrap();
        assert_eq!(skipped.boxtype().type_field(), FourCC::from(*b"ftyp"));

        // Next box should be 'free'
        let raw = reader.read_box().unwrap();
        assert_eq!(raw.boxtype().type_field(), FourCC::from(*b"free"));
    }

    #[test]
    fn read_header_insufficient_data() {
        // Only 4 bytes, not enough for a header
        let data = vec![0x00, 0x00, 0x00, 0x0C];

        let mut reader = BoxReader::new(Cursor::new(data));
        let result = reader.read_box();

        assert!(result.is_err());
    }

    #[test]
    fn read_payload_insufficient_data() {
        // Header says size=12, but only 2 bytes of payload
        let data = vec![
            0x00, 0x00, 0x00, 0x0C, // size = 12
            b'f', b't', b'y', b'p', // type = 'ftyp'
            0x01, 0x02, // only 2 bytes instead of 4
        ];

        let mut reader = BoxReader::new(Cursor::new(data));
        let result = reader.read_box();

        assert!(result.is_err());
    }
}
