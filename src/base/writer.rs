//!

use crate::BoxCodec;
use crate::BoxEncode;
use crate::BoxHeader;
use crate::error::Result;

use crate::cursor::WriteCursor;

/// A trait for writing boxes into byte slices.
pub trait BoxWrite {
    /// Writes the box into the given byte slice.
    fn write_to(&self, bytes: &mut [u8]) -> Result<usize>;
}

impl<B> BoxWrite for B
where
    B: BoxCodec + BoxEncode,
{
    fn write_to(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);
        write_box_in(&mut cur, self)?;
        Ok(cur.position())
    }
}

pub(crate) fn write_box_in<B>(cur: &mut WriteCursor<'_>, boxed: &B) -> Result<()>
where
    B: BoxCodec + BoxEncode,
{
    let start_pos = cur.position();
    let header = BoxHeader::new(boxed.boxtype(), 0);
    header.write_in(cur)?;

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
