//! Iterators for BMFF structures.
//!
//! This module provides iterators for traversing BMFF data structures
//! in a memory-efficient, zero-copy manner.
//!
//! # Box Iteration
//!
//! The primary iterator is [`BoxIter`], which parses consecutive boxes
//! from a byte slice. Use [`iter_boxes`] for convenient access:
//!
//! ```
//! use mp4_bmff::iter_boxes;
//!
//! let data = [
//!     0x00, 0x00, 0x00, 0x08, b'f', b'r', b'e', b'e', // free box (8 bytes)
//!     0x00, 0x00, 0x00, 0x08, b's', b'k', b'i', b'p', // skip box (8 bytes)
//! ];
//!
//! let boxes: Vec<_> = iter_boxes(&data)
//!     .filter_map(|r| r.ok())
//!     .collect();
//!
//! assert_eq!(boxes.len(), 2);
//! ```
//!
//! # Fixed-Size Entry Iteration
//!
//! For boxes containing arrays of fixed-size entries (like sample tables),
//! [`FixedSizeEntryIter`] provides efficient iteration over entries
//! without individual allocations.

use core::fmt;

use crate::base::rawbox::RawBoxRef;
use crate::cursor::ReadCursor;

use crate::error::*;

/// Zero-copy iterator over consecutive BMFF boxes in a byte slice.
///
/// This iterator parses boxes one at a time from the underlying byte slice,
/// returning each as a [`RawBoxRef`]. It handles all box size formats:
/// - Compact size (32-bit)
/// - Extended size (64-bit, when size field = 1)
/// - EOF marker (size = 0, consumes remaining data)
///
/// # Error Handling
///
/// Parsing errors are returned as `Err` items rather than panicking.
/// After an error, subsequent calls to `next()` may return more errors
/// or `None` depending on the remaining data.
///
/// # Example
///
/// ```
/// use mp4_bmff::iter_boxes;
///
/// let data = [
///     0x00, 0x00, 0x00, 0x0C, // size = 12
///     b'f', b't', b'y', b'p', // type = "ftyp"
///     b'i', b's', b'o', b'm', // payload
/// ];
///
/// for result in iter_boxes(&data) {
///     let raw_box = result.unwrap();
///     println!("Box: {} ({} bytes)", raw_box.boxtype(), raw_box.len());
/// }
/// ```
pub struct BoxIter<'a> {
    cur: ReadCursor<'a>,
}

impl fmt::Debug for BoxIter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.fork()).finish()
    }
}

impl<'a> BoxIter<'a> {
    /// Creates a new `BoxIter` from the given byte slice.
    pub(crate) fn new(content: &'a [u8]) -> Self {
        Self {
            cur: ReadCursor::new(content),
        }
    }

    pub(crate) fn fork(&self) -> Self {
        Self {
            cur: self.cur.fork(),
        }
    }
}

impl<'a> Iterator for BoxIter<'a> {
    type Item = Result<RawBoxRef<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur.is_empty() {
            return None;
        }

        match RawBoxRef::parse(self.cur.remaining_slice()) {
            Ok(r) => {
                let len = r.len();
                if let Err(e) = self.cur.advance(len) {
                    return Some(Err(e.into()));
                }

                Some(Ok(r))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

/// Creates an iterator over BMFF boxes in the given byte slice.
///
/// This is a convenience function that creates a [`BoxIter`] for parsing
/// consecutive boxes from raw bytes. It's commonly used to iterate over
/// top-level boxes in a file or child boxes within a container box.
///
/// # Arguments
///
/// * `data` - Byte slice containing one or more consecutive BMFF boxes
///
/// # Returns
///
/// An iterator yielding `Result<RawBoxRef>` for each box.
///
/// # Example
///
/// ```
/// use mp4_bmff::iter_boxes;
/// use mp4_bmff::BoxType;
///
/// // Two consecutive boxes
/// let data = [
///     // ftyp box (8 bytes, header only)
///     0x00, 0x00, 0x00, 0x08, b'f', b't', b'y', b'p',
///     // free box (8 bytes, header only)
///     0x00, 0x00, 0x00, 0x08, b'f', b'r', b'e', b'e',
/// ];
///
/// let types: Vec<_> = iter_boxes(&data)
///     .filter_map(|r| r.ok())
///     .map(|b| b.boxtype())
///     .collect();
///
/// assert_eq!(types, vec![BoxType::FTYP, BoxType::FREE]);
/// ```
pub fn iter_boxes(data: &[u8]) -> BoxIter<'_> {
    BoxIter::new(data)
}

pub(crate) trait FixedEntry<const N: usize>: Sized + Copy {
    const ENTRY_SIZE: usize = N;

    fn from_bytes(bytes: &[u8; N]) -> Self;
    #[cfg(feature = "alloc")]
    fn to_bytes(&self) -> [u8; N];
}

pub(crate) struct FixedEntryIter<'a, E: FixedEntry<N>, const N: usize> {
    bytes: &'a [u8],
    remaining: usize,
    _marker: core::marker::PhantomData<E>,
}

impl<'a, E: FixedEntry<N>, const N: usize> FixedEntryIter<'a, E, N> {
    pub(crate) fn new(data: &'a [u8]) -> Self {
        Self {
            bytes: data,
            remaining: data.len() / N,
            _marker: core::marker::PhantomData,
        }
    }

    pub(crate) fn fork(&self) -> Self {
        Self {
            bytes: self.bytes,
            remaining: self.remaining,
            _marker: core::marker::PhantomData,
        }
    }
}

impl<E: FixedEntry<N>, const N: usize> Iterator for FixedEntryIter<'_, E, N> {
    type Item = E;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;

        let (entry_bytes, rest) = self.bytes.split_at(N);
        self.bytes = rest;

        let entry_array: &[u8; N] = entry_bytes.try_into().expect("slice length checked above");
        Some(E::from_bytes(entry_array))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<E: FixedEntry<N>, const N: usize> ExactSizeIterator for FixedEntryIter<'_, E, N> {}

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
        assert_eq!(view.boxtype().type_field(), FourCC::from(*b"ftyp"));
        assert_eq!(view.payload(), &[0x01, 0x02, 0x03, 0x04]);

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
        assert_eq!(view0.boxtype().type_field(), FourCC::from(*b"ftyp"));
        assert_eq!(view0.payload().len(), 4);

        let view1 = boxes[1].as_ref().unwrap();
        assert_eq!(view1.boxtype().type_field(), FourCC::from(*b"moov"));
        assert_eq!(view1.payload().len(), 8);

        let view2 = boxes[2].as_ref().unwrap();
        assert_eq!(view2.boxtype().type_field(), FourCC::from(*b"free"));
        assert_eq!(view2.payload().len(), 0);
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
        assert_eq!(view0.boxtype().type_field(), FourCC::from(*b"ftyp"));

        let view1 = boxes[1].as_ref().unwrap();
        assert_eq!(view1.boxtype().type_field(), FourCC::from(*b"mdat"));
        assert!(view1.boxsize().is_eof());
        assert_eq!(view1.payload().len(), 8);
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
        assert_eq!(view0.boxtype().type_field(), FourCC::from(*b"ftyp"));
        assert!(!view0.boxsize().is_extended());

        let view1 = boxes[1].as_ref().unwrap();
        assert_eq!(view1.boxtype().type_field(), FourCC::from(*b"mdat"));
        assert!(view1.boxsize().is_extended());
        assert_eq!(view1.payload().len(), 8);
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
        assert_eq!(view.boxtype().type_field(), FourCC::from(*b"ftyp"));

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
            .map(|v| v.boxtype().type_field())
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
