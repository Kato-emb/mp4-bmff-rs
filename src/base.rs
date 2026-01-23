//! ISOBMFF base structures:
//! - `header`: Box headers (size and type).
//! - `rawbox`: Raw box representation (header + payload).

pub mod header;
pub mod rawbox;
