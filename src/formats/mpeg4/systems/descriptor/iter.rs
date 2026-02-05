//! Descriptor iterator for MPEG-4 Systems descriptors.

use crate::error::*;

use crate::cursor::ReadCursor;

use super::RawDescriptorRef;

/// An iterator over MPEG-4 Systems descriptors in a byte slice.
pub struct DescriptorIter<'a> {
    cur: ReadCursor<'a>,
}

impl<'a> DescriptorIter<'a> {
    pub(crate) fn new(content: &'a [u8]) -> Self {
        Self {
            cur: ReadCursor::new(content),
        }
    }
}

impl<'a> Iterator for DescriptorIter<'a> {
    type Item = Result<RawDescriptorRef<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur.is_empty() {
            return None;
        }

        match RawDescriptorRef::parse(self.cur.remaining_slice()) {
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

/// Creates an iterator over MPEG-4 Systems descriptors in the given byte slice.
pub fn iter_descriptors(data: &[u8]) -> DescriptorIter<'_> {
    DescriptorIter::new(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::mpeg4::systems::descriptor::Tag;

    #[test]
    fn test_iter_empty() {
        let data: [u8; 0] = [];
        let mut iter = iter_descriptors(&data);

        assert!(iter.next().is_none());
    }

    #[test]
    fn test_iter_single_descriptor() {
        // tag(0x05) + size(0x04) + instance(4 bytes)
        let data = [0x05, 0x04, 0x01, 0x02, 0x03, 0x04];
        let mut iter = iter_descriptors(&data);

        let first = iter.next();
        assert!(first.is_some());
        let descr = first.unwrap().unwrap();
        assert_eq!(descr.tag(), Tag::DECODER_SPECIFIC_INFO_TAG);
        assert_eq!(descr.instance(), &[0x01, 0x02, 0x03, 0x04]);

        assert!(iter.next().is_none());
    }

    #[test]
    fn test_iter_multiple_descriptors() {
        // descriptor 1: tag(0x05) + size(0x02) + instance(2 bytes)
        // descriptor 2: tag(0x06) + size(0x01) + instance(1 byte)
        // descriptor 3: tag(0x03) + size(0x03) + instance(3 bytes)
        let data = [
            0x05, 0x02, 0xAA, 0xBB, // descriptor 1
            0x06, 0x01, 0xCC, // descriptor 2
            0x03, 0x03, 0xDD, 0xEE, 0xFF, // descriptor 3
        ];
        let iter = iter_descriptors(&data);

        let descriptors: Vec<_> = iter.collect();
        assert_eq!(descriptors.len(), 3);

        let d1 = descriptors[0].as_ref().unwrap();
        assert_eq!(d1.tag(), Tag::DECODER_SPECIFIC_INFO_TAG);
        assert_eq!(d1.instance(), &[0xAA, 0xBB]);

        let d2 = descriptors[1].as_ref().unwrap();
        assert_eq!(d2.tag(), Tag::SL_CONFIG_DESCR_TAG);
        assert_eq!(d2.instance(), &[0xCC]);

        let d3 = descriptors[2].as_ref().unwrap();
        assert_eq!(d3.tag(), Tag::ES_DESCR_TAG);
        assert_eq!(d3.instance(), &[0xDD, 0xEE, 0xFF]);
    }

    #[test]
    fn test_iter_with_two_byte_size() {
        // tag(0x05) + size(0x81 0x00 = 128) + instance(128 bytes)
        let mut data = vec![0x05, 0x81, 0x00];
        data.extend_from_slice(&[0xAB; 128]);

        let mut iter = iter_descriptors(&data);

        let descr = iter.next().unwrap().unwrap();
        assert_eq!(descr.tag(), Tag::DECODER_SPECIFIC_INFO_TAG);
        assert_eq!(descr.instance().len(), 128);

        assert!(iter.next().is_none());
    }

    #[test]
    fn test_iter_truncated_descriptor() {
        // tag(0x05) + size(0x10 = 16) but only 4 bytes of instance
        let data = [0x05, 0x10, 0x01, 0x02, 0x03, 0x04];
        let mut iter = iter_descriptors(&data);

        let result = iter.next();
        assert!(result.is_some());
        assert!(result.unwrap().is_err());
    }

    #[test]
    fn test_iter_count() {
        let data = [
            0x05, 0x01, 0xAA, // descriptor 1
            0x06, 0x01, 0xBB, // descriptor 2
            0x03, 0x01, 0xCC, // descriptor 3
        ];
        let iter = iter_descriptors(&data);

        assert_eq!(iter.count(), 3);
    }
}
