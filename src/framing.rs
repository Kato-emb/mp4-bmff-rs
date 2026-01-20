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

        cur.write_array(boxtype.type_field().as_bytes())?;

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

pub(crate) fn write_box_in<F>(
    cur: &mut WriteCursor<'_>,
    boxtype: BoxType,
    payload_len: usize,
    write_payload: F,
) -> Result<()>
where
    F: FnOnce(&mut [u8]) -> Result<()>,
{
    let required_len = BoxFrameMut::required_len(boxtype, payload_len);
    let buf = cur.take_mut(required_len)?;
    let mut frame = BoxFrameMut::new(buf, boxtype, payload_len)?;
    write_payload(frame.payload_mut())?;
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // BoxFrame tests
    // ========================================================================

    #[test]
    fn box_frame_parse_basic() {
        // ftyp box: size=20, type="ftyp", payload=12 bytes
        let data: [u8; 20] = [
            0x00, 0x00, 0x00, 0x14, // size = 20
            b'f', b't', b'y', b'p', // type = "ftyp"
            b'i', b's', b'o', b'm', // payload: major_brand
            0x00, 0x00, 0x02, 0x00, // payload: minor_version
            b'i', b's', b'o', b'm', // payload: compatible_brand
        ];

        let frame = BoxFrame::parse(&data).unwrap();

        assert_eq!(frame.header_len(), 8);
        assert_eq!(frame.boxsize().value(), Some(20));
        assert_eq!(frame.boxtype(), BoxType::FTYP);
        assert_eq!(frame.payload().len(), 12);
        assert_eq!(frame.len(), 20);
        assert!(!frame.is_empty());
        assert_eq!(frame.as_bytes(), &data);
    }

    #[test]
    fn box_frame_parse_empty_payload() {
        // free box with no payload: size=8
        let data: [u8; 8] = [
            0x00, 0x00, 0x00, 0x08, // size = 8
            b'f', b'r', b'e', b'e', // type = "free"
        ];

        let frame = BoxFrame::parse(&data).unwrap();

        assert_eq!(frame.header_len(), 8);
        assert_eq!(frame.payload().len(), 0);
        assert!(frame.is_empty());
    }

    #[test]
    fn box_frame_parse_extended_size() {
        // Box with extended size (size=1, largesize=24)
        let mut data = vec![0u8; 24];
        data[0..4].copy_from_slice(&1u32.to_be_bytes()); // size = 1 (extended marker)
        data[4..8].copy_from_slice(b"mdat"); // type = "mdat"
        data[8..16].copy_from_slice(&24u64.to_be_bytes()); // largesize = 24
        data[16..24].copy_from_slice(b"testdata"); // payload

        let frame = BoxFrame::parse(&data).unwrap();

        assert_eq!(frame.header_len(), 16);
        assert_eq!(frame.boxsize().value(), Some(24));
        assert_eq!(frame.boxtype(), BoxType::MDAT);
        assert_eq!(frame.payload(), b"testdata");
    }

    #[test]
    fn box_frame_parse_eof_box() {
        // EOF box: size=0 means extends to end of file
        let data: [u8; 16] = [
            0x00, 0x00, 0x00, 0x00, // size = 0 (EOF)
            b'm', b'd', b'a', b't', // type = "mdat"
            b't', b'e', b's', b't', // payload
            b'd', b'a', b't', b'a',
        ];

        let frame = BoxFrame::parse(&data).unwrap();

        assert_eq!(frame.header_len(), 8);
        assert!(frame.boxsize().is_eof());
        assert_eq!(frame.boxtype(), BoxType::MDAT);
        assert_eq!(frame.payload(), b"testdata");
        assert_eq!(frame.len(), 16); // entire remaining data
    }

    #[test]
    fn box_frame_parse_uuid_box() {
        // UUID box: type="uuid" + 16-byte usertype
        let uuid_bytes: [u8; 16] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10,
        ];
        let mut data = vec![0u8; 28];
        data[0..4].copy_from_slice(&28u32.to_be_bytes()); // size = 28
        data[4..8].copy_from_slice(b"uuid"); // type = "uuid"
        data[8..24].copy_from_slice(&uuid_bytes); // usertype
        data[24..28].copy_from_slice(b"test"); // payload

        let frame = BoxFrame::parse(&data).unwrap();

        assert_eq!(frame.header_len(), 24);
        assert_eq!(frame.boxsize().value(), Some(28));
        assert!(frame.boxtype().is_uuid());
        assert_eq!(frame.boxtype().user_type().unwrap().as_bytes(), &uuid_bytes);
        assert_eq!(frame.payload(), b"test");
    }

    #[test]
    fn box_frame_parse_truncates_extra_data() {
        // Data has extra bytes beyond declared size
        let data: [u8; 16] = [
            0x00, 0x00, 0x00, 0x0c, // size = 12
            b'f', b'r', b'e', b'e', // type = "free"
            b't', b'e', b's', b't', // payload (4 bytes)
            0xDE, 0xAD, 0xBE, 0xEF, // extra data (not part of box)
        ];

        let frame = BoxFrame::parse(&data).unwrap();

        assert_eq!(frame.len(), 12);
        assert_eq!(frame.as_bytes().len(), 12);
        assert_eq!(frame.payload(), b"test");
    }

    #[test]
    fn box_frame_parse_error_insufficient_header() {
        let data: [u8; 4] = [0x00, 0x00, 0x00, 0x10]; // Only 4 bytes, need at least 8

        let result = BoxFrame::parse(&data);
        assert!(result.is_err());
    }

    #[test]
    fn box_frame_parse_error_size_exceeds_data() {
        let data: [u8; 8] = [
            0x00, 0x00, 0x00, 0x20, // size = 32 (but only 8 bytes available)
            b'f', b'r', b'e', b'e',
        ];

        let result = BoxFrame::parse(&data);
        assert!(result.is_err());
    }

    // ========================================================================
    // BoxFrameMut tests
    // ========================================================================

    #[test]
    fn box_frame_mut_new_basic() {
        let mut buf = vec![0u8; 20];
        let payload_len = 12;

        let frame = BoxFrameMut::new(&mut buf, BoxType::FTYP, payload_len).unwrap();

        assert_eq!(frame.header_len(), 8);
        assert_eq!(frame.boxsize().value(), Some(20));
        assert_eq!(frame.boxtype(), BoxType::FTYP);
        assert_eq!(frame.payload().len(), 12);
        assert_eq!(frame.len(), 20);

        // Verify header bytes
        assert_eq!(&buf[0..4], &20u32.to_be_bytes()); // size
        assert_eq!(&buf[4..8], b"ftyp"); // type
    }

    #[test]
    fn box_frame_mut_new_uuid() {
        let uuid = Uuid::from([
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10,
        ]);
        let boxtype = BoxType::from_uuid(uuid);
        let payload_len = 4;
        let mut buf = vec![0u8; BoxFrameMut::required_len(boxtype, payload_len)];

        let frame = BoxFrameMut::new(&mut buf, boxtype, payload_len).unwrap();

        assert_eq!(frame.header_len(), 24); // 8 + 16 (uuid)
        assert_eq!(frame.boxtype(), boxtype);
        assert!(frame.boxtype().is_uuid());
        assert_eq!(frame.payload().len(), 4);

        // Verify header bytes
        assert_eq!(&buf[4..8], b"uuid");
        assert_eq!(&buf[8..24], uuid.as_bytes());
    }

    #[test]
    fn box_frame_mut_write_payload() {
        let mut buf = vec![0u8; 16];
        let payload_len = 8;

        let mut frame = BoxFrameMut::new(&mut buf, BoxType::FREE, payload_len).unwrap();

        // Write to payload
        frame.payload_mut().copy_from_slice(b"testdata");

        assert_eq!(frame.payload(), b"testdata");
        assert_eq!(&buf[8..16], b"testdata");
    }

    #[test]
    fn box_frame_mut_error_buffer_too_small() {
        let mut buf = vec![0u8; 10];
        let payload_len = 12; // requires 20 bytes total

        let result = BoxFrameMut::new(&mut buf, BoxType::FTYP, payload_len);
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(
                e.kind(),
                ErrorKind::NotEnoughBytes {
                    expected: 20,
                    remaining: 10
                }
            ));
        }
    }

    #[test]
    fn box_frame_mut_required_len_basic() {
        assert_eq!(BoxFrameMut::required_len(BoxType::FTYP, 0), 8);
        assert_eq!(BoxFrameMut::required_len(BoxType::FTYP, 12), 20);
        assert_eq!(BoxFrameMut::required_len(BoxType::MDAT, 100), 108);
    }

    #[test]
    fn box_frame_mut_required_len_uuid() {
        let uuid = Uuid::from([0u8; 16]);
        let boxtype = BoxType::from_uuid(uuid);

        assert_eq!(BoxFrameMut::required_len(boxtype, 0), 24); // 8 + 16
        assert_eq!(BoxFrameMut::required_len(boxtype, 10), 34); // 8 + 16 + 10
    }

    // ========================================================================
    // Round-trip tests
    // ========================================================================

    #[test]
    fn box_frame_round_trip() {
        // Create with BoxFrameMut
        let mut buf = vec![0u8; 20];
        {
            let mut frame = BoxFrameMut::new(&mut buf, BoxType::FTYP, 12).unwrap();
            frame.payload_mut()[0..4].copy_from_slice(b"isom");
            frame.payload_mut()[4..8].copy_from_slice(&512u32.to_be_bytes());
            frame.payload_mut()[8..12].copy_from_slice(b"iso2");
        }

        // Parse with BoxFrame
        let frame = BoxFrame::parse(&buf).unwrap();

        assert_eq!(frame.boxtype(), BoxType::FTYP);
        assert_eq!(frame.payload().len(), 12);
        assert_eq!(&frame.payload()[0..4], b"isom");
        assert_eq!(&frame.payload()[8..12], b"iso2");
    }

    #[test]
    fn box_frame_round_trip_uuid() {
        let uuid = Uuid::from([
            0xA1, 0xB2, 0xC3, 0xD4, 0xE5, 0xF6, 0x07, 0x18, 0x29, 0x3A, 0x4B, 0x5C, 0x6D, 0x7E,
            0x8F, 0x90,
        ]);
        let boxtype = BoxType::from_uuid(uuid);

        // Create with BoxFrameMut
        let mut buf = vec![0u8; BoxFrameMut::required_len(boxtype, 8)];
        {
            let mut frame = BoxFrameMut::new(&mut buf, boxtype, 8).unwrap();
            frame.payload_mut().copy_from_slice(b"uuiddata");
        }

        // Parse with BoxFrame
        let frame = BoxFrame::parse(&buf).unwrap();

        assert!(frame.boxtype().is_uuid());
        assert_eq!(frame.boxtype().user_type().unwrap(), uuid);
        assert_eq!(frame.payload(), b"uuiddata");
    }
}
