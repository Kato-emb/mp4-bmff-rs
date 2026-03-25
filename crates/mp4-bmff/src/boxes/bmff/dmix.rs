//! Down Mix Instructions Box (`dmix`) implementation.
//!
//! The Down Mix Instructions Box provides instructions for down-mixing
//! audio channels from a source layout to a target layout, as defined
//! in ISO/IEC 14496-12 Section 12.2.
//!
//! # Types
//!
//! - [`DmixBoxView`]: Zero-copy reference to a down mix instructions box.
//! - [`DmixBox`]: Owned down mix instructions box (requires `alloc`).

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;

use crate::cursor::ReadCursor;

/// A 4-bit encoded downmix coefficient (`bs_downmix_coefficient`).
///
/// Wraps a raw 4-bit value (0–15) and provides methods to decode it
/// to a dB gain value. The decoding table differs for LFE and non-LFE
/// channels (Tables 5 and 6 in ISO/IEC 14496-12).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DownmixCoefficient(u8);

impl DownmixCoefficient {
    /// Silence value (−∞ dB).
    pub const SILENCE: Self = Self(0x0F);

    /// Downmix coefficient encoding for non-LFE channels (Table 5).
    #[rustfmt::skip]
    const TABLE_NON_LFE: [f64; 16] = [
        0.0, -0.5, -1.0, -1.5, -2.0, -2.5, -3.0, -3.5,
        -4.0, -4.5, -5.0, -5.5, -6.0, -7.5, -9.0, f64::NEG_INFINITY,
    ];

    /// Downmix coefficient encoding for LFE channel (Table 6).
    #[rustfmt::skip]
    const TABLE_LFE: [f64; 16] = [
        10.0, 6.0, 4.5, 3.0, 1.5, 0.0, -1.5, -3.0,
        -4.5, -6.0, -10.0, -15.0, -20.0, -30.0, -40.0, f64::NEG_INFINITY,
    ];

    /// Creates a new coefficient from a raw 4-bit value.
    ///
    /// The value is masked to 4 bits.
    pub const fn new(value: u8) -> Self {
        Self(value & 0x0F)
    }

    /// Returns the raw 4-bit encoded value.
    pub const fn raw(self) -> u8 {
        self.0
    }

    /// Returns `true` if this coefficient represents silence (−∞ dB).
    pub const fn is_silence(self) -> bool {
        self.0 == 0x0F
    }

    /// Decodes to a dB value for a non-LFE channel (Table 5).
    pub fn to_db(self) -> f64 {
        Self::TABLE_NON_LFE[self.0 as usize]
    }

    /// Decodes to a dB value for an LFE channel (Table 6).
    pub fn to_db_lfe(self) -> f64 {
        Self::TABLE_LFE[self.0 as usize]
    }
}

impl core::fmt::Debug for DownmixCoefficient {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "DownmixCoefficient(0x{:X})", self.0)
    }
}

impl core::fmt::Display for DownmixCoefficient {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.is_silence() {
            write!(f, "-inf dB")
        } else {
            write!(f, "{:.1} dB", self.to_db())
        }
    }
}

/// Packed 4-bit downmix coefficient matrix.
///
/// Contains the raw byte data of a `target_channel_count × baseChannelCount`
/// matrix of [`DownmixCoefficient`] values, packed as 4-bit nibbles.
/// The data is not directly accessible; use [`coefficients`](Self::coefficients)
/// to iterate over the decoded values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownmixMatrix<T> {
    target_channel_count: u8,
    data: T,
}

impl<T: AsRef<[u8]>> DownmixMatrix<T> {
    /// Creates a new `DownmixMatrix` from raw packed coefficient data.
    pub fn new(target_channel_count: u8, data: T) -> Self {
        Self {
            target_channel_count,
            data,
        }
    }

    /// Returns the target channel count for this matrix.
    pub fn target_channel_count(&self) -> u8 {
        self.target_channel_count
    }

    /// Returns an iterator over the decoded coefficients.
    ///
    /// The coefficients are yielded in row-major order:
    /// `target_channel_count × base_channel_count`.
    /// `base_channel_count` comes from the parent sample entry.
    pub fn coefficients(
        &self,
        base_channel_count: u16,
    ) -> impl Iterator<Item = DownmixCoefficient> + ExactSizeIterator + '_ {
        DownmixCoefficientIter::new(
            self.data.as_ref(),
            self.target_channel_count as usize,
            base_channel_count as usize,
        )
    }

    /// Returns the raw packed byte data.
    pub fn as_bytes(&self) -> &[u8] {
        self.data.as_ref()
    }
}

/// Iterator over 4-bit down-mix coefficients.
///
/// Yields [`DownmixCoefficient`] values representing individual
/// `bs_downmix_coefficient` entries packed 2-per-byte in the coefficient data.
/// The total number of coefficients is `target_channel_count × base_channel_count`.
pub(crate) struct DownmixCoefficientIter<'a> {
    data: &'a [u8],
    pos: usize,
    remaining: usize,
    high_nibble: bool,
}

impl<'a> DownmixCoefficientIter<'a> {
    fn new(data: &'a [u8], target_channel_count: usize, base_channel_count: usize) -> Self {
        Self {
            data,
            pos: 0,
            remaining: target_channel_count * base_channel_count,
            high_nibble: true,
        }
    }
}

impl Iterator for DownmixCoefficientIter<'_> {
    type Item = DownmixCoefficient;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;

        let byte = *self.data.get(self.pos)?;
        let nibble = if self.high_nibble {
            self.high_nibble = false;
            (byte >> 4) & 0x0F
        } else {
            self.high_nibble = true;
            self.pos += 1;
            byte & 0x0F
        };
        Some(DownmixCoefficient::new(nibble))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for DownmixCoefficientIter<'_> {}

define_box_flags!(
    /// Flags for the Down Mix Instructions Box (`dmix`).
    ///
    /// Reserved (should be 0).
    DmixFlags {}
);

/// A reference to a Down Mix Instructions Box (`dmix`).
///
/// Contains instructions for down-mixing audio channels from a source
/// channel layout to a target layout with fewer channels.
///
/// # Structure
///
/// - `version`: Box version (should be 0).
/// - `flags`: Reserved (should be 0).
/// - `target_layout`: Target channel layout identifier.
/// - `target_channel_count`: Number of channels in the target layout.
/// - `in_stream`: If true, coefficients are carried in-stream; if false,
///   they are supplied in this box.
/// - `downmix_id`: Identifier for this down-mix instruction set.
/// - `coefficients`: Raw down-mix coefficient data (4 bits per coefficient),
///   present only when `in_stream` is false. Interpretation requires
///   `baseChannelCount` from the parent sample entry.
#[derive(Debug, Clone)]
pub struct DmixBoxView<'a> {
    /// Box version (should be 0).
    pub version: u8,
    /// Reserved (should be 0).
    pub flags: DmixFlags,
    /// Target channel layout identifier.
    pub target_layout: u8,
    /// Number of channels in the target layout (7 bits).
    pub target_channel_count: u8,
    /// If true, down-mix coefficients are carried in the audio stream.
    /// If false, coefficients are supplied in this box.
    pub in_stream: bool,
    /// Identifier for this down-mix instruction set (7 bits).
    pub downmix_id: u8,
    /// Packed down-mix coefficient matrix.
    /// Present only when `in_stream` is false.
    pub coefficients: Option<DownmixMatrix<&'a [u8]>>,
}

impl BoxCodec for DmixBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::DMIX
    }
}

impl<'de> BoxDecode<'de> for DmixBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = DmixFlags::from_be_bytes(cur.read_array::<3>()?);

        let target_layout = cur.read_u8()?;

        let byte = cur.read_u8()?;
        let target_channel_count = byte & 0x7F; // lower 7 bits

        let byte = cur.read_u8()?;
        let in_stream = (byte >> 7) != 0; // top bit
        let downmix_id = byte & 0x7F; // lower 7 bits

        let coefficients = if !in_stream {
            let data = cur.take(cur.remaining())?;
            Some(DownmixMatrix::new(target_channel_count, data))
        } else {
            None
        };

        Ok(DmixBoxView {
            version,
            flags,
            target_layout,
            target_channel_count,
            in_stream,
            downmix_id,
            coefficients,
        })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use alloc::vec::Vec;

    use super::*;
    use crate::BoxEncode;

    use crate::cursor::WriteCursor;

    /// An owned Down Mix Instructions Box (`dmix`).
    ///
    /// Owned version of [`DmixBoxView`], with heap-allocated coefficient data.
    #[derive(Debug, Clone)]
    pub struct DmixBox {
        /// Box version (should be 0).
        pub version: u8,
        /// Reserved (should be 0).
        pub flags: DmixFlags,
        /// Target channel layout identifier.
        pub target_layout: u8,
        /// Number of channels in the target layout (7 bits).
        pub target_channel_count: u8,
        /// If true, down-mix coefficients are carried in the audio stream.
        pub in_stream: bool,
        /// Identifier for this down-mix instruction set (7 bits).
        pub downmix_id: u8,
        /// Packed down-mix coefficient matrix.
        /// Present only when `in_stream` is false.
        pub coefficients: Option<DownmixMatrix<Vec<u8>>>,
    }

    impl From<&DmixBoxView<'_>> for DmixBox {
        fn from(view: &DmixBoxView<'_>) -> Self {
            DmixBox {
                version: view.version,
                flags: view.flags,
                target_layout: view.target_layout,
                target_channel_count: view.target_channel_count,
                in_stream: view.in_stream,
                downmix_id: view.downmix_id,
                coefficients: view
                    .coefficients
                    .as_ref()
                    .map(|c| DownmixMatrix::new(c.target_channel_count(), c.as_bytes().to_vec())),
            }
        }
    }

    impl DmixBoxView<'_> {
        /// Converts this `DmixBoxView` into an owned `DmixBox`.
        pub fn to_owned(&self) -> DmixBox {
            DmixBox::from(self)
        }
    }

    impl BoxCodec for DmixBox {
        fn boxtype(&self) -> BoxType {
            BoxType::DMIX
        }
    }

    impl BoxDecode<'_> for DmixBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = DmixBoxView::decode(bytes)?;
            Ok(DmixBox::from(&view))
        }
    }

    impl BoxEncode for DmixBox {
        fn encoded_len(&self) -> usize {
            4 // version + flags
            + 1 // target_layout
            + 1 // reserved(1) + target_channel_count(7)
            + 1 // in_stream(1) + downmix_id(7)
            + self.coefficients.as_ref().map_or(0, |c| c.as_bytes().len())
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_be_bytes())?;

            cur.write_u8(self.target_layout)?;
            cur.write_u8(self.target_channel_count & 0x7F)?; // reserved(1) + target_channel_count(7)

            let in_stream_byte = if self.in_stream { 0x80 } else { 0x00 };
            cur.write_u8(in_stream_byte | (self.downmix_id & 0x7F))?;

            if let Some(coefficients) = &self.coefficients {
                cur.write_slice(coefficients.as_bytes())?;
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

    #[test]
    fn decode_in_stream() {
        let data: [u8; 7] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x01, // target_layout = 1
            0x02, // reserved(0) + target_channel_count = 2
            0x85, // in_stream(1) + downmix_id = 5
        ];
        let view = DmixBoxView::decode(&data).unwrap();

        assert_eq!(view.version, 0);
        assert_eq!(view.target_layout, 1);
        assert_eq!(view.target_channel_count, 2);
        assert!(view.in_stream);
        assert_eq!(view.downmix_id, 5);
        assert!(view.coefficients.is_none());
    }

    #[test]
    fn decode_out_of_stream() {
        let data: [u8; 9] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x01, // target_layout = 1
            0x02, // reserved(0) + target_channel_count = 2
            0x05, // in_stream(0) + downmix_id = 5
            0xAB, 0xCD, // coefficient data
        ];
        let view = DmixBoxView::decode(&data).unwrap();

        assert!(!view.in_stream);
        assert_eq!(view.downmix_id, 5);
        assert_eq!(
            view.coefficients.as_ref().map(|c| c.as_bytes()),
            Some(&[0xAB, 0xCD][..])
        );
    }

    #[test]
    fn decode_truncated() {
        let data: [u8; 3] = [0x00; 3];
        assert!(DmixBoxView::decode(&data).is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn round_trip_in_stream() {
        use crate::BoxEncode;

        let original: [u8; 7] = [0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x85];
        let owned = DmixBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; owned.encoded_len()];
        owned.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn round_trip_out_of_stream() {
        use crate::BoxEncode;

        let original: [u8; 9] = [0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x05, 0xAB, 0xCD];
        let owned = DmixBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; owned.encoded_len()];
        owned.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn to_owned_preserves_fields() {
        let data: [u8; 9] = [0x00, 0x00, 0x00, 0x00, 0x03, 0x04, 0x06, 0xFF, 0xEE];
        let view = DmixBoxView::decode(&data).unwrap();
        let owned = view.to_owned();

        assert_eq!(owned.version, view.version);
        assert_eq!(owned.target_layout, view.target_layout);
        assert_eq!(owned.target_channel_count, view.target_channel_count);
        assert_eq!(owned.in_stream, view.in_stream);
        assert_eq!(owned.downmix_id, view.downmix_id);
        assert_eq!(
            owned.coefficients.as_ref().map(|c| c.as_bytes()),
            view.coefficients.as_ref().map(|c| c.as_bytes()),
        );
    }

    #[test]
    fn downmix_matrix_coefficients_iter() {
        // target_channel_count=2, base_channel_count=3 → 6 coefficients
        // Packed as 4-bit nibbles: [0xAB, 0xCD, 0xEF] → [A, B, C, D, E, F]
        let data: [u8; 10] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x01, // target_layout = 1
            0x02, // target_channel_count = 2
            0x05, // in_stream(0) + downmix_id = 5
            0xAB, 0xCD, 0xEF, // 6 coefficients packed as 4-bit nibbles
        ];
        let view = DmixBoxView::decode(&data).unwrap();
        let matrix = view.coefficients.as_ref().unwrap();

        assert_eq!(matrix.target_channel_count(), 2);
        let coeffs: Vec<_> = matrix.coefficients(3).collect();
        assert_eq!(coeffs.len(), 6); // 2 × 3
        let raw: Vec<u8> = coeffs.iter().map(|c| c.raw()).collect();
        assert_eq!(raw, vec![0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F]);
    }

    #[test]
    fn downmix_coefficients_none_for_in_stream() {
        let data: [u8; 7] = [
            0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x85, // in_stream = true
        ];
        let view = DmixBoxView::decode(&data).unwrap();
        assert!(view.coefficients.is_none());
    }

    #[test]
    fn downmix_matrix_exact_size() {
        let data: [u8; 9] = [
            0x00, 0x00, 0x00, 0x00, 0x01, 0x01, // target_channel_count = 1
            0x05, // in_stream(0)
            0xAB, 0xCD,
        ];
        let view = DmixBoxView::decode(&data).unwrap();
        let matrix = view.coefficients.as_ref().unwrap();

        let iter = matrix.coefficients(2);
        assert_eq!(iter.len(), 2); // 1 × 2
    }

    #[test]
    fn downmix_coefficient_to_db() {
        let c = DownmixCoefficient::new(0x0);
        assert_eq!(c.to_db(), 0.0);
        assert_eq!(c.to_db_lfe(), 10.0);

        let c = DownmixCoefficient::new(0x1);
        assert_eq!(c.to_db(), -0.5);
        assert_eq!(c.to_db_lfe(), 6.0);

        let c = DownmixCoefficient::new(0xE);
        assert_eq!(c.to_db(), -9.0);
        assert_eq!(c.to_db_lfe(), -40.0);
    }

    #[test]
    fn downmix_coefficient_silence() {
        let c = DownmixCoefficient::SILENCE;
        assert!(c.is_silence());
        assert!(c.to_db().is_infinite());
        assert!(c.to_db_lfe().is_infinite());
        assert_eq!(format!("{c}"), "-inf dB");
    }

    #[test]
    fn downmix_coefficient_display() {
        let c = DownmixCoefficient::new(0x0);
        assert_eq!(format!("{c}"), "0.0 dB");

        let c = DownmixCoefficient::new(0x6);
        assert_eq!(format!("{c}"), "-3.0 dB");
    }
}
