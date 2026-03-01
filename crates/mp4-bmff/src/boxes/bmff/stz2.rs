//! Compact Sample Size Box (`stz2`).
//!
//! The `stz2` box is an alternative to the `stsz` box for storing sample sizes in a more compact format when all sample sizes are small.
//! It uses a field size of 4, 8, or 16 bits per sample size, allowing for more efficient storage of small samples.
//! The `field_size` in the box header indicates how many bits are used for each sample size, and the actual sizes are stored in a packed format in the entries table.

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;

use crate::cursor::ReadCursor;

define_box_flags!(
    /// Flags for the Compact Sample Size Box (`stz2`).
    ///
    /// Reserved (should be 0).
    Stz2Flags {}
);

/// An entry in the Compact Sample Size Box (`stz2`).
///
/// Represents the size of a single sample in bytes.
/// The field size (4, 8, or 16 bits) is determined by the `field_size` field in the box header,
/// and the actual size is stored in the entries table as either 4-bit nibbles, 8-bit bytes, or 16-bit words.
#[derive(Debug, Clone, Copy)]
pub struct Stz2Entry {
    /// The size of the sample in bytes.
    pub entry_size: u16,
}

/// An iterator over entries in the Compact Sample Size Box (`stz2`).
#[derive(Debug)]
pub struct Stz2EntryIter<'a> {
    entries: &'a [u8],
    field_size: u8,
    remaining: usize,
    low_nibble: Option<u8>,
}

impl Iterator for Stz2EntryIter<'_> {
    type Item = Stz2Entry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;

        let entry_size = match self.field_size {
            4 => {
                if let Some(lo) = self.low_nibble.take() {
                    u16::from(lo)
                } else {
                    let (entry_byte, rest) = self.entries.split_first()?;
                    self.entries = rest;
                    self.low_nibble = Some(entry_byte & 0x0F);
                    u16::from(entry_byte >> 4)
                }
            }
            8 => {
                let (entry_byte, rest) = self.entries.split_first()?;
                self.entries = rest;
                u16::from(*entry_byte)
            }
            16 => {
                let (entry_bytes, rest) = self.entries.split_at(2);
                self.entries = rest;
                u16::from_be_bytes([entry_bytes[0], entry_bytes[1]])
            }
            s => unreachable!("Invalid field size in stz2 box: {s}"),
        };

        Some(Stz2Entry { entry_size })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for Stz2EntryIter<'_> {}

/// A reference to a Compact Sample Size Box (`stz2`).
///
/// Provides the size of each sample in the track using a compact format. The `field_size` indicates how many bits are used for each sample size (4, 8, or 16).
///
/// # Structure
/// - `version`: Box version (should be 0).
/// - `flags`: Reserved (should be 0).
/// - `field_size`: Number of bits used for each sample size (4, 8, or 16).
/// - `sample_count`: Total number of samples in the track.
/// - `entries`: Packed sample sizes (4-bit nibbles, 8-bit bytes, or 16-bit words depending on `field_size`).
#[derive(Debug)]
pub struct Stz2BoxView<'a> {
    /// Box version (should be 0).
    pub version: u8,
    /// Reserved flags (should be 0).
    pub flags: Stz2Flags,
    /// Field size in bits (4, 8, or 16).
    pub field_size: u8,
    /// Total number of samples in the track.
    pub sample_count: u32,
    entries: &'a [u8],
}

impl<'a> Stz2BoxView<'a> {
    /// Returns an iterator over the entries in the Compact Sample Size Box.
    pub fn entries(&self) -> Stz2EntryIter<'a> {
        Stz2EntryIter {
            entries: self.entries,
            field_size: self.field_size,
            remaining: self.sample_count as usize,
            low_nibble: None,
        }
    }
}

impl BoxCodec for Stz2BoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::STZ2
    }
}

impl<'de> BoxDecode<'de> for Stz2BoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = Stz2Flags::from_be_bytes(cur.read_array::<3>()?);

        let field_size = cur.read_u32_be()? as u8;
        if !matches!(field_size, 4 | 8 | 16) {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxField {
                    field: "field_size",
                    reason: "must be 4, 8, or 16",
                },
                BoxType::STZ2,
            ));
        }

        let sample_count = cur.read_u32_be()?;

        // Calculate expected size of entries based on field size and sample count
        let bits_per_sample = field_size as usize;
        let total_bits = bits_per_sample * sample_count as usize;
        let expected_size = (total_bits + 7) / 8; // Round up to nearest byte

        if cur.remaining() != expected_size {
            return Err(Error::in_box(
                ErrorKind::InvalidBoxField {
                    field: "entries",
                    reason: "size does not match field_size and sample_count",
                },
                BoxType::STZ2,
            ));
        }

        let entries = cur.take(expected_size)?;

        Ok(Self {
            version,
            flags,
            field_size,
            sample_count,
            entries,
        })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use alloc::vec::Vec;

    use super::*;
    use crate::BoxEncode;

    use crate::cursor::WriteCursor;

    /// An owned Compact Sample Size Box (`stz2`).
    ///
    /// Contains the size of each sample in the track using a compact format. The `field_size` indicates how many bits are used for each sample size (4, 8, or 16).
    ///
    /// # Structure
    /// - `version`: Box version (should be 0).
    /// - `flags`: Reserved (should be 0).
    /// - `entries`: Vector of sample size entries. The field size is determined by the maximum sample size in the entries.
    #[derive(Debug, Clone)]
    pub struct Stz2Box {
        /// Box version (should be 0).
        pub version: u8,
        /// Reserved flags (should be 0).
        pub flags: Stz2Flags,
        /// Array of sample size entries.
        pub entries: Vec<Stz2Entry>,
    }

    impl Default for Stz2Box {
        fn default() -> Self {
            Stz2Box {
                version: 0,
                flags: Stz2Flags::empty(),
                entries: Vec::new(),
            }
        }
    }

    impl Stz2Box {
        /// Determines the field size (4, 8, or 16 bits) needed to store the sample sizes in this box.
        pub fn field_size(&self) -> Option<u8> {
            self.entries
                .iter()
                .map(|e| e.entry_size)
                .max()
                .map(|max_size| {
                    if max_size <= 0x0F {
                        4
                    } else if max_size <= 0xFF {
                        8
                    } else {
                        16
                    }
                })
        }
    }

    impl From<&Stz2BoxView<'_>> for Stz2Box {
        fn from(view: &Stz2BoxView<'_>) -> Self {
            let entries = view.entries().collect();

            Stz2Box {
                version: view.version,
                flags: view.flags,
                entries,
            }
        }
    }

    impl Stz2BoxView<'_> {
        /// Converts this view into an owned `Stz2Box`.
        pub fn to_owned(&self) -> Stz2Box {
            Stz2Box::from(self)
        }
    }

    impl BoxCodec for Stz2Box {
        fn boxtype(&self) -> BoxType {
            BoxType::STZ2
        }
    }

    impl BoxDecode<'_> for Stz2Box {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = Stz2BoxView::decode(bytes)?;
            Ok(Self::from(&view))
        }
    }

    impl BoxEncode for Stz2Box {
        fn encoded_len(&self) -> usize {
            let mut size = 4 // version + flags
            + 4 // field_size
            + 4; // sample_count

            if let Some(field_size) = self.field_size() {
                let bits_per_sample = field_size as usize;
                let total_bits = bits_per_sample * self.entries.len();
                size += (total_bits + 7) / 8; // Round up to nearest byte
            }

            size
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?; // version
            cur.write_array(&self.flags.to_be_bytes())?; // flags

            cur.reserve_zeros(3)?; // Reserved
            let field_size = self.field_size().unwrap_or(8); // Default to 8 if entries are empty
            cur.write_u8(field_size)?; // field_size
            cur.write_u32_be(u32::try_from(self.entries.len())?)?; // sample_count

            let mut high_nibble: Option<u8> = None;
            for entry in &self.entries {
                match field_size {
                    4 => {
                        if let Some(hi) = high_nibble.take() {
                            cur.write_u8(hi << 4 | (entry.entry_size as u8 & 0x0F))?;
                        } else {
                            high_nibble = Some((entry.entry_size as u8) & 0x0F);
                        }
                    }
                    8 => {
                        cur.write_u8(entry.entry_size as u8)?;
                    }
                    16 => {
                        cur.write_u16_be(entry.entry_size)?;
                    }
                    s => unreachable!("Invalid field size in stz2 box: {s}"),
                }
            }

            // Flush remaining high nibble for odd-count 4-bit entries
            if let Some(hi) = high_nibble {
                cur.write_u8(hi << 4)?;
            }

            Ok(cur.position())
        }
    }
}

#[cfg(feature = "alloc")]
pub use owned::*;

#[cfg(test)]
mod tests {
    use super::*;

    // field_size=16, 2 entries: 256, 512
    fn raw_data_16bit() -> [u8; 16] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x10, // reserved(24) + field_size(8) = 16
            0x00, 0x00, 0x00, 0x02, // sample_count = 2
            // entries (2 bytes each)
            0x01, 0x00, // entry_size = 256
            0x02, 0x00, // entry_size = 512
        ]
    }

    // field_size=8, 3 entries: 10, 20, 30
    fn raw_data_8bit() -> [u8; 15] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x08, // reserved(24) + field_size(8) = 8
            0x00, 0x00, 0x00, 0x03, // sample_count = 3
            // entries (1 byte each)
            0x0A, // entry_size = 10
            0x14, // entry_size = 20
            0x1E, // entry_size = 30
        ]
    }

    // field_size=4, 4 entries (even): 1, 2, 3, 4
    fn raw_data_4bit_even() -> [u8; 14] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x04, // reserved(24) + field_size(8) = 4
            0x00, 0x00, 0x00, 0x04, // sample_count = 4
            // entries (high nibble first)
            0x12, // entry 1=0x1, entry 2=0x2
            0x34, // entry 3=0x3, entry 4=0x4
        ]
    }

    // field_size=4, 3 entries (odd): 5, 10, 15 — last byte padded
    fn raw_data_4bit_odd() -> [u8; 14] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x04, // reserved(24) + field_size(8) = 4
            0x00, 0x00, 0x00, 0x03, // sample_count = 3
            // entries (high nibble first)
            0x5A, // entry 1=5, entry 2=10
            0xF0, // entry 3=15, pad=0
        ]
    }

    #[test]
    fn test_stz2_box_view_decode_16bit() {
        let data = raw_data_16bit();
        let stz2 = Stz2BoxView::decode(&data).unwrap();

        assert_eq!(stz2.version, 0);
        assert_eq!(stz2.field_size, 16);
        assert_eq!(stz2.sample_count, 2);

        let entries: Vec<_> = stz2.entries().map(|e| e.entry_size).collect();
        assert_eq!(entries, [256, 512]);
    }

    #[test]
    fn test_stz2_box_view_decode_8bit() {
        let data = raw_data_8bit();
        let stz2 = Stz2BoxView::decode(&data).unwrap();

        assert_eq!(stz2.field_size, 8);
        assert_eq!(stz2.sample_count, 3);

        let entries: Vec<_> = stz2.entries().map(|e| e.entry_size).collect();
        assert_eq!(entries, [10, 20, 30]);
    }

    #[test]
    fn test_stz2_box_view_decode_4bit_even() {
        let data = raw_data_4bit_even();
        let stz2 = Stz2BoxView::decode(&data).unwrap();

        assert_eq!(stz2.field_size, 4);
        assert_eq!(stz2.sample_count, 4);

        let entries: Vec<_> = stz2.entries().map(|e| e.entry_size).collect();
        assert_eq!(entries, [1, 2, 3, 4]);
    }

    #[test]
    fn test_stz2_box_view_decode_4bit_odd() {
        let data = raw_data_4bit_odd();
        let stz2 = Stz2BoxView::decode(&data).unwrap();

        assert_eq!(stz2.field_size, 4);
        assert_eq!(stz2.sample_count, 3);

        let entries: Vec<_> = stz2.entries().map(|e| e.entry_size).collect();
        assert_eq!(entries, [5, 10, 15]);
    }

    #[test]
    fn test_stz2_box_view_empty_entries() {
        let data: [u8; 12] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x08, // reserved(24) + field_size(8) = 8
            0x00, 0x00, 0x00, 0x00, // sample_count = 0
        ];

        let stz2 = Stz2BoxView::decode(&data).unwrap();
        assert_eq!(stz2.sample_count, 0);
        assert_eq!(stz2.entries().count(), 0);
    }

    #[test]
    fn test_stz2_box_view_truncated() {
        let data: [u8; 4] = [0x00; 4];
        let result = Stz2BoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_stz2_box_view_invalid_field_size() {
        let data: [u8; 12] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x03, // reserved(24) + field_size(8) = 3 (invalid)
            0x00, 0x00, 0x00, 0x00, // sample_count = 0
        ];

        let result = Stz2BoxView::decode(&data);
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_stz2_box_round_trip_16bit() {
        use crate::BoxEncode;

        let original = raw_data_16bit();
        let stz2 = Stz2Box::decode(&original).unwrap();

        let mut encoded = vec![0u8; stz2.encoded_len()];
        stz2.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_stz2_box_round_trip_8bit() {
        use crate::BoxEncode;

        let original = raw_data_8bit();
        let stz2 = Stz2Box::decode(&original).unwrap();

        let mut encoded = vec![0u8; stz2.encoded_len()];
        stz2.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_stz2_box_round_trip_4bit_even() {
        use crate::BoxEncode;

        let original = raw_data_4bit_even();
        let stz2 = Stz2Box::decode(&original).unwrap();

        let mut encoded = vec![0u8; stz2.encoded_len()];
        stz2.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_stz2_box_round_trip_4bit_odd() {
        use crate::BoxEncode;

        let original = raw_data_4bit_odd();
        let stz2 = Stz2Box::decode(&original).unwrap();

        let mut encoded = vec![0u8; stz2.encoded_len()];
        stz2.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_stz2_box_to_owned() {
        let data = raw_data_8bit();
        let view = Stz2BoxView::decode(&data).unwrap();
        let owned = view.to_owned();

        assert_eq!(owned.version, view.version);
        assert_eq!(owned.flags.bits(), view.flags.bits());
        assert_eq!(owned.entries.len(), view.sample_count as usize);
    }
}
