//! Zero-copy frame views into BMFF boxes.

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

use crate::error::*;
use crate::header::BoxSize;
use crate::header::BoxType;
use crate::header::boxtype;
use crate::types::FourCC;
use crate::types::Uuid;

/// A zero-copy frame view into a BMFF box.
///
/// This structure holds a reference to the raw byte slice of a box,
/// validating only the header structure on construction.
/// The type and size are computed on demand from the underlying bytes.
#[derive(Debug, Clone, Copy)]
pub struct BoxFrame<'a> {
    inner: BoxFrameInner<&'a [u8]>,
}

impl<'a> BoxFrame<'a> {
    /// Creates a new `BoxFrame` from a byte slice.
    ///
    /// Validates that the header can be read and truncates the slice
    /// to match the declared box size.
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);
        Self::parse_in(&mut cur)
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<Self> {
        let (header_len, frame_size) = validate_header(cur.remaining_slice())?;
        let data = cur.take(frame_size)?;
        let inner = BoxFrameInner::new(data, header_len);
        Ok(Self { inner })
    }

    /// Returns the header length in bytes.
    #[inline]
    pub fn header_len(&self) -> usize {
        self.inner.header_len()
    }

    /// Returns the box size.
    #[inline]
    pub fn boxsize(&self) -> BoxSize {
        self.inner.boxsize()
    }

    /// Returns the box type.
    #[inline]
    pub fn boxtype(&self) -> BoxType {
        self.inner.boxtype()
    }

    /// Returns the payload (data after the header).
    #[inline]
    pub fn payload(&self) -> &'a [u8] {
        &self.inner.data[self.inner.header_len..]
    }

    /// Returns the total length of the box.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns true if the box has no payload.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the raw data slice.
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.inner.data
    }
}

/// A mutable zero-copy frame view into a BMFF box.
///
/// This structure allows in-place modification of box fields,
/// particularly useful for updating the size field after writing payload.
#[derive(Debug)]
pub struct BoxFrameMut<'a> {
    inner: BoxFrameInner<&'a mut [u8]>,
}

impl<'a> BoxFrameMut<'a> {
    /// Calculates the required length of a box with the given type and payload length.
    #[inline]
    pub fn required_len(boxtype: BoxType, payload_len: usize) -> usize {
        let mut header_len = BASE_HEADER_SIZE;

        if payload_len > u32::MAX as usize {
            header_len += 8; // Extended size
        }

        if boxtype.is_uuid() {
            header_len += 16; // UUID user type
        }

        header_len + payload_len
    }

    /// Creates a new `BoxFrameMut` from a mutable byte slice and header length.
    pub fn new(bytes: &'a mut [u8], boxtype: BoxType, payload_len: usize) -> Result<Self> {
        let bytes_len = bytes.len();
        let required_len = Self::required_len(boxtype, payload_len);

        if bytes_len < required_len {
            return Err(Error::new(ErrorKind::NotEnoughBytes {
                expected: required_len,
                remaining: bytes_len,
            }));
        }

        let mut cur = WriteCursor::new(bytes);
        let is_extended = payload_len > u32::MAX as usize;
        let header_len = required_len - payload_len;

        if is_extended {
            // Write extended size
            cur.write_u32_be(1)?; // Indicate extended size
        } else {
            cur.write_u32_be(bytes_len as u32)?; // Write size
        }

        cur.write_array(&boxtype.type_field().as_bytes())?;

        if is_extended {
            cur.write_u64_be(bytes_len as u64)?; // Write largesize
        }

        if boxtype.is_uuid() {
            cur.write_array(boxtype.user_type().unwrap().as_bytes())?;
        }

        let inner = BoxFrameInner::new(bytes, header_len);
        Ok(Self { inner })
    }

    /// Returns the header length in bytes.
    #[inline]
    pub fn header_len(&self) -> usize {
        self.inner.header_len()
    }

    /// Returns the box size.
    #[inline]
    pub fn boxsize(&self) -> BoxSize {
        self.inner.boxsize()
    }

    /// Returns the box type.
    #[inline]
    pub fn boxtype(&self) -> BoxType {
        self.inner.boxtype()
    }

    /// Returns the payload (data after the header).
    #[inline]
    pub fn payload(&self) -> &[u8] {
        self.inner.payload()
    }

    /// Returns a mutable reference to the payload.
    #[inline]
    pub fn payload_mut(&mut self) -> &mut [u8] {
        self.inner.payload_mut()
    }

    /// Returns the total length of the box.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns true if the box has no payload.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the raw data slice.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.inner.as_bytes()
    }
}

const BASE_HEADER_SIZE: usize = 8;

/// Validates the header and returns (header_len, frame_size).
fn validate_header(bytes: &[u8]) -> Result<(usize, usize)> {
    let mut cur = ReadCursor::new(bytes);

    let size = cur.read_u32_be()?;
    let fourcc = FourCC::new(cur.read_array()?);
    let mut header_len = BASE_HEADER_SIZE;

    if size == BoxSize::MARKER_EXTENDED_SIZE {
        header_len += 8;
    }

    if fourcc == boxtype::UUID {
        header_len += 16;
    }

    if bytes.len() < header_len {
        return Err(Error::new(ErrorKind::NotEnoughBytes {
            expected: header_len,
            remaining: bytes.len(),
        }));
    }

    let boxsize = match size {
        0 => BoxSize::eof(),
        1 => {
            let largesize = cur.read_u64_be()?;
            BoxSize::from_u64(largesize).map_err(|e| Error::at(e.into(), 8))?
        }
        _ => BoxSize::from_u32(size).map_err(|e| Error::at(e.into(), 0))?,
    };

    let frame_size = match boxsize.value() {
        Some(s) => s.try_into().map_err(|_| {
            Error::new(ErrorKind::Other {
                description: "box size too large",
            })
        })?,
        None => bytes.len(),
    };

    if frame_size > bytes.len() {
        return Err(Error::new(ErrorKind::NotEnoughBytes {
            expected: frame_size,
            remaining: bytes.len(),
        }));
    }

    Ok((header_len, frame_size))
}

#[derive(Debug, Clone, Copy)]
struct BoxFrameInner<D> {
    data: D,
    header_len: usize,
}

impl<D> BoxFrameInner<D> {
    fn new(data: D, header_len: usize) -> Self {
        Self { data, header_len }
    }
}

impl<D: AsRef<[u8]>> BoxFrameInner<D> {
    #[inline]
    fn header_len(&self) -> usize {
        self.header_len
    }

    fn boxsize(&self) -> BoxSize {
        let bytes = self.data.as_ref();
        let size = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);

        match size {
            0 => BoxSize::eof(),
            1 => {
                let largesize = u64::from_be_bytes([
                    bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                    bytes[15],
                ]);
                BoxSize::from_u64(largesize).expect("valid extended size")
            }
            _ => BoxSize::from_u32(size).expect("valid extended size"),
        }
    }

    fn boxtype(&self) -> BoxType {
        let bytes = self.data.as_ref();
        let fourcc = FourCC::new([bytes[4], bytes[5], bytes[6], bytes[7]]);

        if fourcc == boxtype::UUID {
            let uuid_offset = if self.header_len >= 24 {
                if u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) == 1 {
                    16 // After extended size
                } else {
                    8 // After basic header
                }
            } else {
                8
            };

            let uuid_bytes: [u8; 16] = bytes[uuid_offset..uuid_offset + 16]
                .try_into()
                .expect("uuid bytes");
            let usertype = Uuid::from(uuid_bytes);
            BoxType::from_uuid(usertype)
        } else {
            BoxType::from_fourcc(fourcc).expect("valid fourcc")
        }
    }

    #[inline]
    fn payload(&self) -> &[u8] {
        &self.data.as_ref()[self.header_len..]
    }

    fn len(&self) -> usize {
        match self.boxsize() {
            size if size.is_eof() => self.data.as_ref().len(),
            size => size.value().unwrap() as usize,
        }
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.payload().is_empty()
    }

    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.data.as_ref()
    }
}

impl<D: AsMut<[u8]>> BoxFrameInner<D> {
    #[inline]
    fn payload_mut(&mut self) -> &mut [u8] {
        let header_len = self.header_len;
        &mut self.data.as_mut()[header_len..]
    }
}
