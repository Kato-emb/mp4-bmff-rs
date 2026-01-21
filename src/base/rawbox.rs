//!

use crate::BoxHeader;
use crate::BoxSize;
use crate::BoxType;
use crate::error::*;

use crate::cursor::ReadCursor;

/// A raw BMFF box with its header and data.
#[derive(Debug)]
pub struct RawBox<T> {
    header: BoxHeader,
    data: T,
}

/// A raw BMFF box reference with its header and data as a byte slice.
pub type RawBoxRef<'a> = RawBox<&'a [u8]>;
/// A raw BMFF box mutable reference with its header and data as a mutable byte slice.
pub type RawBoxMut<'a> = RawBox<&'a mut [u8]>;

impl<T> RawBox<T> {
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

    /// Decomposes the `RawBox` into its header and data parts.
    pub fn into_parts(self) -> (BoxHeader, T) {
        (self.header, self.data)
    }
}

impl<T: AsRef<[u8]>> RawBox<T> {
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

    /// Returns the box payload as a byte slice.
    #[inline]
    pub fn payload(&self) -> &[u8] {
        let header_len = self.header.header_len();
        &self.data.as_ref()[header_len..]
    }
}

impl<T: AsMut<[u8]>> RawBox<T> {
    /// Returns the box payload as a mutable byte slice.
    #[inline]
    pub fn payload_mut(&mut self) -> &mut [u8] {
        let header_len = self.header.header_len();
        &mut self.data.as_mut()[header_len..]
    }
}

impl<'a> RawBox<&'a [u8]> {
    /// Returns the box payload as a byte slice.
    #[inline]
    pub fn into_payload(self) -> &'a [u8] {
        let header_len = self.header.header_len();
        &self.data.as_ref()[header_len..]
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<Self> {
        let start_pos = cur.position();
        let header = BoxHeader::parse_in(cur)?;

        let box_size = match header.boxsize() {
            size if size.is_eof() => cur.remaining(),
            size => {
                let value = size.value().unwrap();

                if value > usize::MAX as u64 {
                    return Err(Error::in_box(
                        ErrorKind::InvalidBoxSize {
                            reason: "Box size exceeds usize max",
                            got: value,
                        },
                        header.boxtype(),
                    ));
                }

                value as usize
            }
        };

        let consumed = cur.position() - start_pos;
        if cur.remaining() + consumed < box_size {
            return Err(Error::in_box(
                ErrorKind::NotEnoughBytes {
                    expected: box_size,
                    remaining: cur.remaining() + consumed,
                },
                header.boxtype(),
            ));
        }

        let data = cur.inner()[(start_pos as usize)..(start_pos as usize + box_size)].as_ref();
        cur.advance(box_size)?;
        Ok(RawBox { header, data })
    }

    /// Parses a `RawBoxRef` from the given byte slice.
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let mut cursor = ReadCursor::new(bytes);
        Self::parse_in(&mut cursor)
    }
}

impl<'a> TryFrom<&'a [u8]> for RawBox<&'a [u8]> {
    type Error = Error;

    fn try_from(bytes: &'a [u8]) -> Result<Self> {
        Self::parse(bytes)
    }
}

impl<'a> RawBox<&'a mut [u8]> {
    /// Returns the box payload as a mutable byte slice.
    #[inline]
    pub fn into_payload_mut(self) -> &'a mut [u8] {
        let header_len = self.header.header_len();
        &mut self.data.as_mut()[header_len..]
    }

    /// Creates a `RawBoxMut` from the given header and mutable byte slice.
    pub fn new(buf: &'a mut [u8], header: BoxHeader) -> Result<Self> {
        let box_size = match header.boxsize() {
            size if size.is_eof() => buf.len(),
            size => {
                let value = size.value().unwrap();

                if value > usize::MAX as u64 {
                    return Err(Error::in_box(
                        ErrorKind::InvalidBoxSize {
                            reason: "Box size exceeds usize max",
                            got: value,
                        },
                        header.boxtype(),
                    ));
                }

                value as usize
            }
        };

        if buf.len() < box_size {
            return Err(Error::in_box(
                ErrorKind::NotEnoughBytes {
                    expected: box_size,
                    remaining: buf.len(),
                },
                header.boxtype(),
            ));
        }

        header.write(buf)?;
        let data = &mut buf[..box_size];
        Ok(RawBox { header, data })
    }
}
