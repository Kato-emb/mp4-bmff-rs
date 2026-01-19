use crate::cursor::ReadCursor;

use super::view::DescriptorView;
use crate::error::*;

/// Descriptor Iterator over a byte slice
pub struct DescrptorIter<'a> {
    cur: ReadCursor<'a>,
}

impl<'a> DescrptorIter<'a> {
    /// Creates a new Descriptor Iterator from the given byte slice
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            cur: ReadCursor::new(data),
        }
    }
}

impl<'a> Iterator for DescrptorIter<'a> {
    type Item = Result<DescriptorView<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur.is_empty() {
            return None;
        }

        Some(DescriptorView::parse_in(&mut self.cur))
    }
}
