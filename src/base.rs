//! ISOBMFF base structures:
//! - `header`: Box headers (size and type).
//! - `frame`: Box frames (header + payload).
//! - `iter`: Box iterator over a byte slice.
//! - `codec`: Traits for encoding/decoding boxes.

pub mod codec;
pub mod frame;
pub mod header;
pub mod iter;
