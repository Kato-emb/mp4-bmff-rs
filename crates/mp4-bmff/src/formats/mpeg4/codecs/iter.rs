//! Iterators for codec parameter sets (SPS, PPS, etc.).
//!
//! This module provides iterators for parsing NAL unit parameter sets
//! stored in codec configuration records. Parameter sets use a
//! length-prefixed format where each entry has a 16-bit big-endian
//! length followed by the NAL unit data.

/// Iterator over length-prefixed parameter sets in a byte slice.
///
/// This iterator parses parameter sets (SPS, PPS, etc.) from codec
/// configuration records where each set is prefixed with a 16-bit
/// big-endian length.
///
/// # Format
///
/// ```text
/// +--------+--------+------------------+
/// | Length (16-bit) | NAL Unit Data    |
/// | big-endian      | (Length bytes)   |
/// +--------+--------+------------------+
/// ```
///
/// # Example
///
/// ```
/// use mp4_bmff::formats::mpeg4::codecs::avc::AVCDecoderConfigurationRecordView;
///
/// // Parse an AVC configuration and iterate over SPS
/// # let avc_config_data = [
/// #     0x01, 0x42, 0xC0, 0x1E, 0xFF, 0xE1,
/// #     0x00, 0x04, 0x67, 0x42, 0xC0, 0x1E,
/// #     0x01, 0x00, 0x04, 0x68, 0xCE, 0x3C, 0x80,
/// # ];
/// let config = AVCDecoderConfigurationRecordView::parse(&avc_config_data).unwrap();
/// for sps in config.sps() {
///     println!("SPS NAL unit: {} bytes", sps.len());
/// }
/// ```
#[cfg(any(feature = "avc", feature = "hevc"))]
pub struct ParameterSetsIter<'a> {
    pub(super) data: &'a [u8],
    pub(super) remaining: usize,
}

#[cfg(any(feature = "avc", feature = "hevc"))]
impl<'a> Iterator for ParameterSetsIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 || self.data.len() < 2 {
            return None;
        }

        let len = u16::from_be_bytes([self.data[0], self.data[1]]) as usize;
        if self.data.len() < 2 + len {
            return None;
        }

        // Extract parameter set
        let ps = &self.data[2..2 + len];

        // Advance cursor
        self.data = &self.data[2 + len..];
        self.remaining -= 1;

        Some(ps)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

#[cfg(any(feature = "avc", feature = "hevc"))]
impl ExactSizeIterator for ParameterSetsIter<'_> {
    fn len(&self) -> usize {
        self.remaining
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_iter() {
        let data: [u8; 0] = [];
        let mut iter = ParameterSetsIter {
            data: &data,
            remaining: 0,
        };

        assert_eq!(iter.len(), 0);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_single_parameter_set() {
        // length(2) = 4, data = [0x01, 0x02, 0x03, 0x04]
        let data = [0x00, 0x04, 0x01, 0x02, 0x03, 0x04];
        let mut iter = ParameterSetsIter {
            data: &data,
            remaining: 1,
        };

        assert_eq!(iter.len(), 1);
        let ps = iter.next().unwrap();
        assert_eq!(ps, &[0x01, 0x02, 0x03, 0x04]);
        assert_eq!(iter.len(), 0);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_multiple_parameter_sets() {
        // PS1: length=3, data=[0xAA, 0xBB, 0xCC]
        // PS2: length=2, data=[0xDD, 0xEE]
        // PS3: length=4, data=[0x11, 0x22, 0x33, 0x44]
        let data = [
            0x00, 0x03, 0xAA, 0xBB, 0xCC, // PS1
            0x00, 0x02, 0xDD, 0xEE, // PS2
            0x00, 0x04, 0x11, 0x22, 0x33, 0x44, // PS3
        ];
        let mut iter = ParameterSetsIter {
            data: &data,
            remaining: 3,
        };

        assert_eq!(iter.len(), 3);

        let ps1 = iter.next().unwrap();
        assert_eq!(ps1, &[0xAA, 0xBB, 0xCC]);
        assert_eq!(iter.len(), 2);

        let ps2 = iter.next().unwrap();
        assert_eq!(ps2, &[0xDD, 0xEE]);
        assert_eq!(iter.len(), 1);

        let ps3 = iter.next().unwrap();
        assert_eq!(ps3, &[0x11, 0x22, 0x33, 0x44]);
        assert_eq!(iter.len(), 0);

        assert!(iter.next().is_none());
    }

    #[test]
    fn test_size_hint() {
        let data = [0x00, 0x02, 0xAA, 0xBB, 0x00, 0x01, 0xCC];
        let iter = ParameterSetsIter {
            data: &data,
            remaining: 2,
        };

        assert_eq!(iter.size_hint(), (2, Some(2)));
    }

    #[test]
    fn test_truncated_length() {
        // Only 1 byte, need 2 for length
        let data = [0x00];
        let mut iter = ParameterSetsIter {
            data: &data,
            remaining: 1,
        };

        assert!(iter.next().is_none());
    }

    #[test]
    fn test_truncated_data() {
        // length=10 but only 4 bytes of data
        let data = [0x00, 0x0A, 0x01, 0x02, 0x03, 0x04];
        let mut iter = ParameterSetsIter {
            data: &data,
            remaining: 1,
        };

        assert!(iter.next().is_none());
    }

    #[test]
    fn test_zero_remaining_with_data() {
        // Data exists but remaining is 0
        let data = [0x00, 0x02, 0xAA, 0xBB];
        let mut iter = ParameterSetsIter {
            data: &data,
            remaining: 0,
        };

        assert!(iter.next().is_none());
    }

    #[test]
    fn test_collect() {
        let data = [
            0x00, 0x02, 0xAA, 0xBB, // PS1
            0x00, 0x03, 0xCC, 0xDD, 0xEE, // PS2
        ];
        let iter = ParameterSetsIter {
            data: &data,
            remaining: 2,
        };

        let collected: Vec<&[u8]> = iter.collect();
        assert_eq!(collected.len(), 2);
        assert_eq!(collected[0], &[0xAA, 0xBB]);
        assert_eq!(collected[1], &[0xCC, 0xDD, 0xEE]);
    }
}
