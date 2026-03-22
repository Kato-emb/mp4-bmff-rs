//! BMFF box writer for stream-based I/O.
//!
//! This module provides [`BoxWriter`] for writing BMFF boxes to any
//! type implementing [`std::io::Write`].

use std::io::{
    self, //
    Write,
};

use mp4_bmff::BoxCodec;
use mp4_bmff::BoxEncode;
use mp4_bmff::BoxHeader;
use mp4_bmff::base::rawbox::RawBox;

/// A writer for BMFF boxes to a stream.
///
/// `BoxWriter` wraps any [`Write`] implementor and provides
/// methods to write encoded boxes. For typed boxes, it maintains an internal
/// payload buffer that is reused across writes to minimize allocations.
///
/// # Writing Methods
///
/// - [`write_box`](Self::write_box): Write an encoded box (requires `BoxCodec + BoxEncode`)
/// - [`write_raw_box`](Self::write_raw_box): Write a raw box directly
///
/// # Example
///
/// ```
/// use mp4_bmff_util::io::BoxWriter;
/// use mp4_bmff::BoxType;
/// use mp4_bmff::RawBox;
///
/// let mut output = Vec::new();
/// let mut writer = BoxWriter::new(&mut output);
///
/// // Write a raw box
/// let raw = RawBox::new(BoxType::FREE, vec![]);
/// writer.write_raw_box(&raw).unwrap();
///
/// // The output now contains the serialized box
/// assert_eq!(output.len(), 8); // header only (no payload)
/// ```
#[derive(Debug)]
pub struct BoxWriter<W> {
    inner: W,
    buf: Vec<u8>,
}

impl<W> BoxWriter<W> {
    /// Creates a new `BoxWriter` wrapping the given writer.
    ///
    /// # Arguments
    ///
    /// * `inner` - Any type implementing [`Write`]
    pub fn new(inner: W) -> Self {
        BoxWriter {
            inner,
            buf: Vec::new(),
        }
    }

    /// Returns a reference to the underlying writer.
    #[inline]
    pub fn get_ref(&self) -> &W {
        &self.inner
    }

    /// Returns a mutable reference to the underlying writer.
    ///
    /// Use this to access writer-specific functionality like flushing.
    #[inline]
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Consumes the `BoxWriter`, returning the underlying writer.
    #[inline]
    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<W: Write> BoxWriter<W> {
    /// Writes an encoded BMFF box to the stream.
    ///
    /// This method encodes the box using its [`BoxEncode`] implementation,
    /// prepends the appropriate header, and writes the complete box to
    /// the underlying writer.
    ///
    /// # Type Parameters
    ///
    /// * `B` - A type implementing both [`BoxCodec`] and [`BoxEncode`]
    ///
    /// # Errors
    ///
    /// Returns an error if encoding fails or an I/O error occurs.
    pub fn write_box<B>(&mut self, boxed: &B) -> io::Result<()>
    where
        B: BoxCodec + BoxEncode,
    {
        let payload_len = boxed.encoded_len();
        let header = BoxHeader::new(boxed.boxtype(), payload_len as u64);
        let header_len = header.header_len();

        // Write header from a stack-allocated buffer
        let mut header_buf = [0u8; BoxHeader::MAX_HEADER_SIZE];
        header
            .write(&mut header_buf[..header_len])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        self.inner.write_all(&header_buf[..header_len])?;

        // Encode payload into the reusable buffer
        self.buf.resize(payload_len, 0);
        boxed
            .encode(&mut self.buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        self.inner.write_all(&self.buf[..payload_len])?;

        Ok(())
    }

    /// Writes a raw BMFF box to the stream.
    ///
    /// This method writes a [`RawBox`] directly without additional encoding.
    /// Use this when you have a box that doesn't need further processing
    /// or when copying boxes between files.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The payload storage type (e.g., `&[u8]`, `Vec<u8>`)
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing.
    pub fn write_raw_box<T: AsRef<[u8]>>(&mut self, raw: &RawBox<T>) -> io::Result<()> {
        let header = raw.header();
        let header_len = header.header_len();

        // Write header from a stack-allocated buffer
        let mut header_buf = [0u8; BoxHeader::MAX_HEADER_SIZE];
        header
            .write(&mut header_buf[..header_len])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        self.inner.write_all(&header_buf[..header_len])?;

        // Write payload directly — no intermediate buffer needed
        self.inner.write_all(raw.payload())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mp4_bmff::BoxType;

    #[test]
    fn accessors() {
        let buf = Vec::new();
        let mut writer = BoxWriter::new(buf);

        writer.get_mut().push(0xFF);
        assert_eq!(writer.get_ref().len(), 1);

        let inner = writer.into_inner();
        assert_eq!(inner, vec![0xFF]);
    }

    #[test]
    fn write_raw_box_roundtrip() {
        let payload = vec![0x01, 0x02, 0x03, 0x04];
        let raw = RawBox::new(BoxType::FTYP, payload.clone());

        let mut output = Vec::new();
        let mut writer = BoxWriter::new(&mut output);
        writer.write_raw_box(&raw).unwrap();

        // Verify output: header (8 bytes) + payload (4 bytes) = 12 bytes
        assert_eq!(output.len(), 12);

        // Verify size field
        assert_eq!(&output[0..4], &[0x00, 0x00, 0x00, 0x0C]);

        // Verify type field
        assert_eq!(&output[4..8], b"ftyp");

        // Verify payload
        assert_eq!(&output[8..12], &payload[..]);
    }

    #[test]
    fn write_multiple_boxes() {
        let mut output = Vec::new();
        let mut writer = BoxWriter::new(&mut output);

        let box1 = RawBox::new(BoxType::FTYP, vec![0x01, 0x02]);
        let box2 = RawBox::new(BoxType::FREE, vec![]);

        writer.write_raw_box(&box1).unwrap();
        writer.write_raw_box(&box2).unwrap();

        // box1: 8 + 2 = 10 bytes, box2: 8 + 0 = 8 bytes
        assert_eq!(output.len(), 18);

        // Verify first box size
        assert_eq!(&output[0..4], &[0x00, 0x00, 0x00, 0x0A]);

        // Verify second box offset and size
        assert_eq!(&output[10..14], &[0x00, 0x00, 0x00, 0x08]);
    }
}
