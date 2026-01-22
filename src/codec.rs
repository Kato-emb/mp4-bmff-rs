//! BMFF box codec traits and functions.
//!
//! This module defines traits for encoding and decoding BMFF boxes,
//! as well as functions for reading and writing boxes to and from byte slices.
//! It leverages the `BoxCodec`, `BoxDecode`, and `BoxEncode` traits
//! to provide a flexible and extensible framework for handling BMFF boxes.

use crate::base::header::BoxHeader;
use crate::base::header::BoxType;
use crate::base::rawbox::RawBox;

use crate::error::Result;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

/// A trait for BMFF box codecs.
pub trait BoxCodec {
    /// Returns the box type.
    fn boxtype(&self) -> BoxType;
}

/// A trait for decoding BMFF boxes from byte slices.
pub trait BoxDecode<'de>: Sized {
    /// Decodes the box from the given byte slice.
    fn decode(bytes: &'de [u8]) -> Result<Self>;
}

/// A trait for encoding BMFF boxes into byte slices.
pub trait BoxEncode {
    /// Encodes the box into the given byte slice.
    fn encode(&self, bytes: &mut [u8]) -> Result<usize>;
}

/// Reads and decodes a BMFF box from the given byte slice.
pub fn read_box<'de, B>(bytes: &'de [u8]) -> Result<B>
where
    B: BoxCodec + BoxDecode<'de>,
{
    let mut cur = ReadCursor::new(bytes);
    let b = read_box_in(&mut cur)?;
    Ok(b)
}

/// Encodes and writes a BMFF box into the given byte slice.
pub fn write_box<B>(bytes: &mut [u8], boxed: &B) -> Result<usize>
where
    B: BoxCodec + BoxEncode,
{
    let mut cur = WriteCursor::new(bytes);
    write_box_in(&mut cur, boxed)?;
    Ok(cur.position())
}

pub(crate) fn read_box_in<'de, B>(cur: &mut ReadCursor<'de>) -> Result<B>
where
    B: BoxCodec + BoxDecode<'de>,
{
    let raw = RawBox::parse(cur.inner())?;
    debug_assert!(raw.len() == raw.header().total_size() as usize);
    cur.advance(raw.len())?;

    let b = B::decode(raw.into_payload())?;
    Ok(b)
}

pub(crate) fn write_box_in<B>(cur: &mut WriteCursor<'_>, boxed: &B) -> Result<()>
where
    B: BoxCodec + BoxEncode,
{
    let start_pos = cur.position();
    let header = BoxHeader::new(boxed.boxtype(), 0);
    let header_len = header.header_len();
    header.write(&mut cur.take_mut(header_len)?)?;

    let buf = cur.take_mut(cur.remaining())?;
    let payload_size = boxed.encode(buf)?;

    let total_size = header.header_len() + payload_size;

    // patch the size
    // TODO. handle extended size if needed
    cur.set_position(start_pos);
    cur.write_u32_be(total_size as u32)?;

    // advance to the end of the box
    cur.set_position(start_pos + total_size);

    Ok(())
}
