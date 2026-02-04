//! ISO/IEC 14496-15 Carriage of NAL unit structured video

#[cfg(feature = "avc")]
pub mod avc;

/// An iterator over NAL units in a byte slice
pub struct NalUnitIter<'a> {
    data: &'a [u8],
    remaining: usize,
}

impl<'a> Iterator for NalUnitIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 || self.data.len() < 2 {
            return None;
        }

        let len = u16::from_be_bytes([self.data[0], self.data[1]]) as usize;
        if self.data.len() < 2 + len {
            return None;
        }

        // Extract NAL unit
        let nalu = &self.data[2..2 + len];

        // Advance cursor
        self.data = &self.data[2 + len..];
        self.remaining -= 1;

        Some(nalu)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for NalUnitIter<'_> {
    fn len(&self) -> usize {
        self.remaining
    }
}

define_box_types!(
    /// AVC Configuration Box
    AVCC = b"avcC",
    /// AVC Sample Entry Box
    AVC1 = b"avc1",
    /// AVC3 Sample Entry Box
    AVC3 = b"avc3",
    /// AVC2 Sample Entry Box
    AVC2 = b"avc2",
    /// AVC4 Sample Entry Box
    AVC4 = b"avc4",
);
