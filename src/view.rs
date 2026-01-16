//!

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

use crate::error::*;
use crate::header::BoxHeader;
use crate::header::BoxSize;
use crate::header::BoxType;

/// A view into a BMFF box, containing its header and payload.
#[derive(Clone, Copy, Debug)]
pub struct BoxView<'a> {
    /// The box header.
    pub header: BoxHeader,
    /// The box payload.
    pub payload: &'a [u8],
}

impl<'a> BoxView<'a> {
    /// Parses a `BoxView` from the given `ReadCursor`.
    pub fn parse(cur: &mut ReadCursor<'a>) -> Result<Self> {
        let header = BoxHeader::parse(cur)?;

        let payload_size = match header.payload_size() {
            Some(size) => {
                if cur.remaining() < size as usize {
                    return Err(Error::new(ErrorKind::MismatchedBoxSize {
                        expected: size,
                        found: cur.remaining() as u64,
                    })
                    .at(cur.position() as u64));
                }

                size as usize
            }
            None => cur.remaining(),
        };

        let payload = cur
            .take(payload_size)
            .map_err(|e| Error::new(e.into()).at(cur.position() as u64))?;

        Ok(Self { header, payload })
    }

    /// Writes the `BoxView` to the given `WriteCursor`.
    pub fn write(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
        self.header
            .write(cur)
            .map_err(|e| e.at(cur.position() as u64))?;

        cur.write_slice(self.payload)
            .map_err(|e| Error::new(e.into()).at(cur.position() as u64))?;

        Ok(())
    }
}

/// Writes a BMFF box with the given header and payload to the `WriteCursor`.
pub fn write_box(cur: &mut WriteCursor<'_>, type_: BoxType, payload: &[u8]) -> Result<()> {
    let box_type_size = if type_.is_uuid() { 16 } else { 4 };
    let box_size = 4 + box_type_size + payload.len(); // size (4) + type (4 or 16) + payload

    let size = if box_size > u32::MAX as usize {
        BoxSize::from_u64(payload.len() as u64 + 16)
    } else {
        BoxSize::from_u64(payload.len() as u64 + 8)
    }
    .map_err(|e| Error::new(e.into()).with_box_type(type_))?;

    let header = BoxHeader::new(size, type_);
    header.write(cur).map_err(|e| e.at(cur.position() as u64))?;

    cur.write_slice(payload)
        .map_err(|e| Error::new(e.into()).at(cur.position() as u64))?;

    Ok(())
}

/// Writes a BMFF box to the `WriteCursor`, using the provided closure to write the payload.
pub fn write_box_with<F>(w: &mut WriteCursor<'_>, type_: BoxType, f: F) -> Result<()>
where
    F: FnOnce(&mut WriteCursor<'_>) -> Result<()>,
{
    let start_pos = w.position();

    // Reserve space for header
    let box_type_size = if type_.is_uuid() { 16 } else { 4 };
    let header_size = 4 + box_type_size; // size (4) + type (4 or 16)
    w.reserve_zeros(header_size)
        .map_err(|e| Error::new(e.into()).at(w.position() as u64))?;

    // Write payload using the provided closure
    f(w)?;

    let end_pos = w.position();
    let payload_size = (end_pos - start_pos - header_size) as u64;

    // Go back and write the header
    let box_size = if end_pos - start_pos > u32::MAX as usize {
        BoxSize::from_u64(payload_size + 16)
    } else {
        BoxSize::from_u64(payload_size + 8)
    }
    .map_err(|e| Error::new(e.into()).with_box_type(type_))?;

    let header = BoxHeader::new(box_size, type_);

    let current_pos = w.position();
    w.set_position(start_pos);
    header.write(w).map_err(|e| e.at(w.position() as u64))?;
    w.set_position(current_pos);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::BoxSize;
    use crate::header::BoxType;
    use crate::types::FourCC;
    use crate::types::Uuid;

    #[test]
    fn parse_normal_box() {
        // size=20, type="ftyp", payload=12 bytes
        let data = [
            0x00, 0x00, 0x00, 0x14, // size: 20
            b'f', b't', b'y', b'p', // type: ftyp
            b'i', b's', b'o', b'm', // payload: "isom" + 8 bytes
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut cur = ReadCursor::new(&data);
        let view = BoxView::parse(&mut cur).unwrap();

        assert_eq!(view.header.boxsize().value(), Some(20));
        assert!(!view.header.boxsize().is_extended());
        assert_eq!(view.header.boxtype().type_field(), FourCC::from(*b"ftyp"));
        assert_eq!(view.payload.len(), 12);
        assert_eq!(&view.payload[0..4], b"isom");
        assert_eq!(cur.position(), 20);
        assert!(cur.is_empty());
    }

    #[test]
    fn parse_eof_box() {
        // size=0 means box extends to EOF
        let data = [
            0x00, 0x00, 0x00, 0x00, // size: 0 (to end of file)
            b'f', b'r', b'e', b'e', // type: free
            0xDE, 0xAD, 0xBE, 0xEF, // remaining data as payload
            0xCA, 0xFE,
        ];

        let mut cur = ReadCursor::new(&data);
        let view = BoxView::parse(&mut cur).unwrap();

        assert!(view.header.boxsize().is_eof());
        assert_eq!(view.header.boxtype().type_field(), FourCC::from(*b"free"));
        assert_eq!(view.payload.len(), 6);
        assert_eq!(view.payload, &[0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE]);
        assert!(cur.is_empty());
    }

    #[test]
    fn parse_extended_size_box() {
        // Extended size box using size=1 marker.
        // largesize=24, header=16 bytes (4+4+8), payload=8 bytes
        let mut data = vec![
            0x00, 0x00, 0x00, 0x01, // size: 1 (extended size marker)
            b'm', b'd', b'a', b't', // type: mdat
            0x00, 0x00, 0x00, 0x00, // largesize high 32 bits
            0x00, 0x00, 0x00, 0x18, // largesize low 32 bits: 24
        ];
        data.extend_from_slice(&[0xAB; 8]); // 8 bytes payload (24 - 16 = 8)

        let mut cur = ReadCursor::new(&data);
        let view = BoxView::parse(&mut cur).unwrap();

        assert_eq!(view.header.boxsize().value(), Some(24));
        assert!(view.header.boxsize().is_extended()); // Now correctly true
        assert_eq!(view.header.header_size(), 16);
        assert_eq!(view.payload.len(), 8);
        assert_eq!(view.payload, &[0xAB; 8]);
        assert_eq!(cur.position(), 24); // 16 (header) + 8 (payload)
    }

    #[test]
    fn parse_uuid_box() {
        let uuid_bytes: [u8; 16] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
            0x0F, 0x10,
        ];
        let mut data = vec![
            0x00, 0x00, 0x00, 0x20, // size: 32
            b'u', b'u', b'i', b'd', // type: uuid
        ];
        data.extend_from_slice(&uuid_bytes);
        data.extend_from_slice(&[0xCD; 8]); // payload (8 bytes)

        let mut cur = ReadCursor::new(&data);
        let view = BoxView::parse(&mut cur).unwrap();

        assert!(view.header.boxtype().is_uuid());
        assert_eq!(
            view.header.boxtype().user_type(),
            Some(Uuid::new(uuid_bytes))
        );
        assert_eq!(view.payload.len(), 8);
        assert_eq!(view.payload, &[0xCD; 8]);
    }

    #[test]
    fn parse_insufficient_payload() {
        // Header says size=20, but only 10 bytes total available
        let data = [
            0x00, 0x00, 0x00, 0x14, // size: 20
            b'f', b't', b'y', b'p', // type: ftyp
            0x00, 0x00, // only 2 bytes of payload (need 12)
        ];

        let mut cur = ReadCursor::new(&data);
        let result = BoxView::parse(&mut cur);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err.kind(),
            ErrorKind::MismatchedBoxSize {
                expected: 12,
                found: 2
            }
        ));
    }

    #[test]
    fn parse_multiple_boxes() {
        // Two consecutive boxes
        let data = [
            // First box: size=12, type="ftyp", payload=4 bytes
            0x00, 0x00, 0x00, 0x0C, b'f', b't', b'y', b'p', 0x01, 0x02, 0x03, 0x04,
            // Second box: size=16, type="moov", payload=8 bytes
            0x00, 0x00, 0x00, 0x10, b'm', b'o', b'o', b'v', 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A,
            0x0B, 0x0C,
        ];

        let mut cur = ReadCursor::new(&data);

        let view1 = BoxView::parse(&mut cur).unwrap();
        assert_eq!(view1.header.boxtype().type_field(), FourCC::from(*b"ftyp"));
        assert_eq!(view1.payload, &[0x01, 0x02, 0x03, 0x04]);

        let view2 = BoxView::parse(&mut cur).unwrap();
        assert_eq!(view2.header.boxtype().type_field(), FourCC::from(*b"moov"));
        assert_eq!(
            view2.payload,
            &[0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C]
        );

        assert!(cur.is_empty());
    }

    #[test]
    fn write_roundtrip() {
        let header = BoxHeader::new(
            BoxSize::from_u32(20).unwrap(),
            BoxType::from_fourcc(FourCC::from(*b"test")).unwrap(),
        );
        let payload = b"hello world!";
        let original = BoxView { header, payload };

        let mut buf = [0u8; 20];
        let mut write_cur = WriteCursor::new(&mut buf);
        original.write(&mut write_cur).unwrap();

        let mut read_cur = ReadCursor::new(&buf);
        let parsed = BoxView::parse(&mut read_cur).unwrap();

        assert_eq!(original.header, parsed.header);
        assert_eq!(original.payload, parsed.payload);
    }

    #[test]
    fn write_roundtrip_uuid() {
        let uuid = Uuid::new([0x99; 16]);
        let header = BoxHeader::new(BoxSize::from_u32(32).unwrap(), BoxType::from_uuid(uuid));
        let payload = b"payload!";
        let original = BoxView { header, payload };

        let mut buf = [0u8; 32];
        let mut write_cur = WriteCursor::new(&mut buf);
        original.write(&mut write_cur).unwrap();

        let mut read_cur = ReadCursor::new(&buf);
        let parsed = BoxView::parse(&mut read_cur).unwrap();

        assert_eq!(original.header, parsed.header);
        assert_eq!(original.payload, parsed.payload);
    }

    #[test]
    fn write_buffer_too_small() {
        let header = BoxHeader::new(
            BoxSize::from_u32(20).unwrap(),
            BoxType::from_fourcc(FourCC::from(*b"test")).unwrap(),
        );
        let payload = b"hello world!";
        let view = BoxView { header, payload };

        let mut buf = [0u8; 10]; // Too small
        let mut write_cur = WriteCursor::new(&mut buf);
        let result = view.write(&mut write_cur);

        assert!(result.is_err());
    }
}
