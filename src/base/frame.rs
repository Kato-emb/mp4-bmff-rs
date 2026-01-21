//! Zero-copy frame views into BMFF boxes.

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

use crate::error::*;

use super::header::{
    BoxHeader, //
    BoxSize,
    BoxType,
};

/// A zero-copy frame view into a BMFF box.
///
/// This structure holds a reference to the raw byte slice of a box,
/// validating only the header structure on construction.
/// The type and size are computed on demand from the underlying bytes.
#[derive(Debug)]
pub struct BoxFrame<T> {
    header: BoxHeader,
    data: T,
}

/// Immutable box frame reference.
pub type BoxFrameRef<'a> = BoxFrame<&'a [u8]>;
/// Mutable box frame reference.
pub type BoxFrameMut<'a> = BoxFrame<&'a mut [u8]>;

impl<T> BoxFrame<T> {
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

    /// Decomposes the `BoxFrame` into its header and data parts.
    pub fn into_parts(self) -> (BoxHeader, T) {
        (self.header, self.data)
    }
}

impl<T: AsRef<[u8]>> BoxFrame<T> {
    /// Returns the total length of the box.
    #[inline]
    pub fn len(&self) -> usize {
        match self.boxsize() {
            size if size.is_eof() => self.data.as_ref().len(),
            size => size.value().expect("box size is valid") as usize,
        }
    }

    /// Returns true if the box has no payload.
    #[inline]
    pub fn is_empty(&self) -> bool {
        let header_len = self.header.header_len();
        self.len() == header_len
    }
}

impl<'a> BoxFrameRef<'a> {
    /// Returns the payload (data after the header).
    #[inline]
    pub fn payload(&self) -> &'a [u8] {
        let header_len = self.header.header_len();
        &self.data.as_ref()[header_len..]
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<Self> {
        let start_pos = cur.position();
        let header = BoxHeader::parse_in(cur)?;

        let total_len = if let Some(size) = header.boxsize().value() {
            size as usize
        } else {
            header.header_len() + cur.remaining() // EOF box
        };

        let payload_len = total_len - header.header_len();
        cur.advance(payload_len)?;

        let data = &cur.inner()[start_pos..start_pos + total_len];
        Ok(Self { header, data })
    }

    /// Parses a box frame from the given byte slice.
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);
        Self::parse_in(&mut cur)
    }
}

impl<'a> BoxFrameMut<'a> {
    #[inline]
    pub fn payload(&self) -> &[u8] {
        let header_len = self.header.header_len();
        &self.data.as_ref()[header_len..]
    }

    /// Returns the payload (data after the header).
    #[inline]
    pub fn payload_mut(&mut self) -> &mut [u8] {
        let header_len = self.header.header_len();
        &mut self.data.as_mut()[header_len..]
    }

    pub(crate) fn new(bytes: &'a mut [u8], header: BoxHeader) -> Result<Self> {
        let bytes_len = bytes.len();
        if let Some(required_len) = header.boxsize().value() {
            if bytes_len < required_len as usize {
                return Err(Error::new(ErrorKind::NotEnoughBytes {
                    expected: required_len as usize,
                    remaining: bytes_len,
                }));
            }
        }

        let mut cur = WriteCursor::new(bytes);
        header.write_in(&mut cur)?;
        debug_assert!(cur.position() == header.header_len());

        Ok(Self {
            header,
            data: bytes,
        })
    }
}

#[cfg(feature = "alloc")]
pub use owned::*;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use super::*;

    pub type BoxFrameOwned = BoxFrame<Vec<u8>>;

    impl Clone for BoxFrameOwned {
        fn clone(&self) -> Self {
            let data = self.data.clone();
            BoxFrame {
                header: self.header,
                data,
            }
        }
    }

    impl BoxFrameOwned {
        /// Creates an immutable `BoxFrame` by borrowing the data.
        pub fn as_ref(&self) -> BoxFrameRef<'_> {
            BoxFrame {
                header: self.header,
                data: self.data.as_ref(),
            }
        }

        /// Creates a mutable `BoxFrame` by borrowing the data.
        pub fn as_mut(&mut self) -> BoxFrameMut<'_> {
            BoxFrame {
                header: self.header,
                data: self.data.as_mut(),
            }
        }
    }

    impl<T: AsRef<[u8]>> BoxFrame<T> {
        /// Creates an owned `BoxFrame` by copying the data from the given byte slice.
        pub fn to_owned(&self) -> BoxFrameOwned {
            let data = self.data.as_ref().to_vec();
            BoxFrameOwned {
                header: self.header,
                data,
            }
        }
    }
}
