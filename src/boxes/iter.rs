//!

use crate::cursor::ReadCursor;

use super::error::Result;
use super::view::BoxView;

/// An iterator over BMFF boxes in a byte slice.
pub struct BoxIter<'a> {
    cur: ReadCursor<'a>,
}

impl<'a> BoxIter<'a> {
    /// Creates a new `BoxIter` from the given byte slice.
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            cur: ReadCursor::new(data),
        }
    }
}

impl<'a> Iterator for BoxIter<'a> {
    type Item = Result<BoxView<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur.is_empty() {
            return None;
        }

        Some(BoxView::parse(&mut self.cur))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FourCC;

    #[test]
    fn iterate_empty_data() {
        let data = [];
        let mut iter = BoxIter::new(&data);

        assert!(iter.next().is_none());
    }

    #[test]
    fn iterate_single_box() {
        let data = [
            0x00, 0x00, 0x00, 0x0C, // size: 12
            b'f', b't', b'y', b'p', // type: ftyp
            0x01, 0x02, 0x03, 0x04, // payload (4 bytes)
        ];

        let mut iter = BoxIter::new(&data);

        let view = iter.next().unwrap().unwrap();
        assert_eq!(view.header.boxtype().type_field(), FourCC::from(*b"ftyp"));
        assert_eq!(view.payload, &[0x01, 0x02, 0x03, 0x04]);

        assert!(iter.next().is_none());
    }

    #[test]
    fn iterate_multiple_boxes() {
        #[rustfmt::skip]
        let data = [
            // First box: size=12, type="ftyp"
            0x00, 0x00, 0x00, 0x0C,
            b'f', b't', b'y', b'p',
            0x01, 0x02, 0x03, 0x04,
            // Second box: size=16, type="moov"
            0x00, 0x00, 0x00, 0x10,
            b'm', b'o', b'o', b'v',
            0x05, 0x06, 0x07, 0x08,
            0x09, 0x0A, 0x0B, 0x0C,
            // Third box: size=8, type="free" (no payload)
            0x00, 0x00, 0x00, 0x08,
            b'f', b'r', b'e', b'e',
        ];

        let iter = BoxIter::new(&data);
        let boxes: Vec<_> = iter.collect();

        assert_eq!(boxes.len(), 3);

        let view0 = boxes[0].as_ref().unwrap();
        assert_eq!(view0.header.boxtype().type_field(), FourCC::from(*b"ftyp"));
        assert_eq!(view0.payload.len(), 4);

        let view1 = boxes[1].as_ref().unwrap();
        assert_eq!(view1.header.boxtype().type_field(), FourCC::from(*b"moov"));
        assert_eq!(view1.payload.len(), 8);

        let view2 = boxes[2].as_ref().unwrap();
        assert_eq!(view2.header.boxtype().type_field(), FourCC::from(*b"free"));
        assert_eq!(view2.payload.len(), 0);
    }

    #[test]
    fn iterate_with_eof_box() {
        // EOF box (size=0) consumes all remaining data
        #[rustfmt::skip]
        let data = [
            // First box: size=12, type="ftyp"
            0x00, 0x00, 0x00, 0x0C,
            b'f', b't', b'y', b'p',
            0x01, 0x02, 0x03, 0x04,
            // Second box: size=0 (EOF), type="mdat"
            0x00, 0x00, 0x00, 0x00,
            b'm', b'd', b'a', b't',
            0xDE, 0xAD, 0xBE, 0xEF, // remaining data as payload
            0xCA, 0xFE, 0xBA, 0xBE,
        ];

        let iter = BoxIter::new(&data);
        let boxes: Vec<_> = iter.collect();

        assert_eq!(boxes.len(), 2);

        let view0 = boxes[0].as_ref().unwrap();
        assert_eq!(view0.header.boxtype().type_field(), FourCC::from(*b"ftyp"));

        let view1 = boxes[1].as_ref().unwrap();
        assert_eq!(view1.header.boxtype().type_field(), FourCC::from(*b"mdat"));
        assert!(view1.header.boxsize().is_eof());
        assert_eq!(view1.payload.len(), 8);
    }

    #[test]
    fn iterate_with_extended_size() {
        #[rustfmt::skip]
        let mut data = vec![
            // First box: size=12, type="ftyp"
            0x00, 0x00, 0x00, 0x0C,
            b'f', b't', b'y', b'p',
            0x01, 0x02, 0x03, 0x04,
            // Second box: extended size, type="mdat", largesize=24
            0x00, 0x00, 0x00, 0x01, // size: 1 (extended marker)
            b'm', b'd', b'a', b't', // type: mdat
            0x00, 0x00, 0x00, 0x00, // largesize high
            0x00, 0x00, 0x00, 0x18, // largesize low: 24
        ];
        data.extend_from_slice(&[0xAB; 8]); // payload (24 - 16 = 8 bytes)

        let iter = BoxIter::new(&data);
        let boxes: Vec<_> = iter.collect();

        assert_eq!(boxes.len(), 2);

        let view0 = boxes[0].as_ref().unwrap();
        assert_eq!(view0.header.boxtype().type_field(), FourCC::from(*b"ftyp"));
        assert!(!view0.header.boxsize().is_extended());

        let view1 = boxes[1].as_ref().unwrap();
        assert_eq!(view1.header.boxtype().type_field(), FourCC::from(*b"mdat"));
        assert!(view1.header.boxsize().is_extended());
        assert_eq!(view1.payload.len(), 8);
    }

    #[test]
    fn iterate_error_on_truncated_box() {
        #[rustfmt::skip]
        let data = [
            // First box: valid
            0x00, 0x00, 0x00, 0x0C,
            b'f', b't', b'y', b'p',
            0x01, 0x02, 0x03, 0x04,
            // Second box: truncated (says size=20, but only 6 bytes remain)
            0x00, 0x00, 0x00, 0x14,
            b'm', b'o',
        ];

        let mut iter = BoxIter::new(&data);

        // First box succeeds
        let view = iter.next().unwrap().unwrap();
        assert_eq!(view.header.boxtype().type_field(), FourCC::from(*b"ftyp"));

        // Second box fails - not enough data for header
        let result = iter.next().unwrap();
        assert!(result.is_err());
    }

    #[test]
    fn iterate_error_on_insufficient_payload() {
        #[rustfmt::skip]
        let data = [
            // Box with size=20, but only 10 bytes total
            0x00, 0x00, 0x00, 0x14, // size: 20
            b'f', b't', b'y', b'p', // type: ftyp
            0x01, 0x02, // only 2 bytes of payload (need 12)
        ];

        let mut iter = BoxIter::new(&data);

        let result = iter.next().unwrap();
        assert!(result.is_err());
    }

    #[test]
    fn iterate_collect_types() {
        #[rustfmt::skip]
        let data = [
            0x00, 0x00, 0x00, 0x08,
            b'f', b't', b'y', b'p',
            0x00, 0x00, 0x00, 0x08,
            b'm', b'o', b'o', b'v',
            0x00, 0x00, 0x00, 0x08,
            b't', b'r', b'a', b'k',
            0x00, 0x00, 0x00, 0x08,
            b'm', b'd', b'a', b't',
        ];

        let types: Vec<_> = BoxIter::new(&data)
            .filter_map(|r| r.ok())
            .map(|v| v.header.boxtype().type_field())
            .collect();

        assert_eq!(
            types,
            vec![
                FourCC::from(*b"ftyp"),
                FourCC::from(*b"moov"),
                FourCC::from(*b"trak"),
                FourCC::from(*b"mdat"),
            ]
        );
    }
}
