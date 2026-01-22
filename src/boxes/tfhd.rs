use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;

use super::FullBoxFlags;

/// Track Fragment Header Box (`tfhd`).
///
/// This box contains the track fragment header for a track fragment.
/// Each track fragment has exactly one track fragment header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TfhdBox {
    /// The version of the box (should be 0).
    pub version: u8,
    /// The flags of the box.
    pub flags: TfhdFlags,
    /// The track ID for this fragment.
    pub track_id: u32,
    /// The base offset to use when calculating data offsets.
    /// Present if `base_data_offset_present` flag is set.
    pub base_data_offset: Option<u64>,
    /// The sample description index.
    /// Present if `sample_description_index_present` flag is set.
    pub sample_description_index: Option<u32>,
    /// The default sample duration.
    /// Present if `default_sample_duration_present` flag is set.
    pub default_sample_duration: Option<u32>,
    /// The default sample size.
    /// Present if `default_sample_size_present` flag is set.
    pub default_sample_size: Option<u32>,
    /// The default sample flags.
    /// Present if `default_sample_flags_present` flag is set.
    pub default_sample_flags: Option<u32>,
}

impl TfhdBox {
    /// Computes the flags value based on the optional fields present.
    fn compute_flags(&self) -> TfhdFlags {
        let mut flags = self.flags;

        if self.base_data_offset.is_some() {
            flags |= TfhdFlags::BASE_DATA_OFFSET_PRESENT;
        }
        if self.sample_description_index.is_some() {
            flags |= TfhdFlags::SAMPLE_DESCRIPTION_INDEX_PRESENT;
        }
        if self.default_sample_duration.is_some() {
            flags |= TfhdFlags::DEFAULT_SAMPLE_DURATION_PRESENT;
        }
        if self.default_sample_size.is_some() {
            flags |= TfhdFlags::DEFAULT_SAMPLE_SIZE_PRESENT;
        }
        if self.default_sample_flags.is_some() {
            flags |= TfhdFlags::DEFAULT_SAMPLE_FLAGS_PRESENT;
        }

        flags
    }
}

impl TryFrom<&[u8]> for TfhdBox {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self> {
        TfhdBox::decode(value)
    }
}

impl BoxCodec for TfhdBox {
    fn boxtype(&self) -> BoxType {
        BoxType::TFHD
    }
}

impl BoxDecode<'_> for TfhdBox {
    fn decode(bytes: &[u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = TfhdFlags::from_bytes(cur.read_array()?);

        let track_id = cur.read_u32_be()?;

        let base_data_offset = if flags.base_data_offset_present() {
            Some(cur.read_u64_be()?)
        } else {
            None
        };

        let sample_description_index = if flags.sample_description_index_present() {
            Some(cur.read_u32_be()?)
        } else {
            None
        };

        let default_sample_duration = if flags.default_sample_duration_present() {
            Some(cur.read_u32_be()?)
        } else {
            None
        };

        let default_sample_size = if flags.default_sample_size_present() {
            Some(cur.read_u32_be()?)
        } else {
            None
        };

        let default_sample_flags = if flags.default_sample_flags_present() {
            Some(cur.read_u32_be()?)
        } else {
            None
        };

        Ok(TfhdBox {
            version,
            flags,
            track_id,
            base_data_offset,
            sample_description_index,
            default_sample_duration,
            default_sample_size,
            default_sample_flags,
        })
    }
}

impl BoxEncode for TfhdBox {
    fn encode(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

        let flags = self.compute_flags();

        cur.write_u8(self.version)?;
        cur.write_array(&flags.to_bytes())?;
        cur.write_u32_be(self.track_id)?;

        if let Some(base_data_offset) = self.base_data_offset {
            cur.write_u64_be(base_data_offset)?;
        }
        if let Some(sample_description_index) = self.sample_description_index {
            cur.write_u32_be(sample_description_index)?;
        }
        if let Some(default_sample_duration) = self.default_sample_duration {
            cur.write_u32_be(default_sample_duration)?;
        }
        if let Some(default_sample_size) = self.default_sample_size {
            cur.write_u32_be(default_sample_size)?;
        }
        if let Some(default_sample_flags) = self.default_sample_flags {
            cur.write_u32_be(default_sample_flags)?;
        }

        Ok(cur.position())
    }
}

/// Specification for the Track Fragment Header Box (`tfhd`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TfhdSpec;

/// Flags for the Track Fragment Header Box (`tfhd`).
pub type TfhdFlags = FullBoxFlags<TfhdSpec>;

impl TfhdFlags {
    /// If set, base_data_offset is present.
    pub const BASE_DATA_OFFSET_PRESENT: TfhdFlags = TfhdFlags::from_bits_truncate(0x000001);
    /// If set, sample_description_index is present.
    pub const SAMPLE_DESCRIPTION_INDEX_PRESENT: TfhdFlags = TfhdFlags::from_bits_truncate(0x000002);
    /// If set, default_sample_duration is present.
    pub const DEFAULT_SAMPLE_DURATION_PRESENT: TfhdFlags = TfhdFlags::from_bits_truncate(0x000008);
    /// If set, default_sample_size is present.
    pub const DEFAULT_SAMPLE_SIZE_PRESENT: TfhdFlags = TfhdFlags::from_bits_truncate(0x000010);
    /// If set, default_sample_flags is present.
    pub const DEFAULT_SAMPLE_FLAGS_PRESENT: TfhdFlags = TfhdFlags::from_bits_truncate(0x000020);
    /// If set, there are no samples for this track in this fragment.
    pub const DURATION_IS_EMPTY: TfhdFlags = TfhdFlags::from_bits_truncate(0x010000);
    /// If set, the base_data_offset is relative to the first byte of the enclosing moof.
    pub const DEFAULT_BASE_IS_MOOF: TfhdFlags = TfhdFlags::from_bits_truncate(0x020000);

    /// Returns true if base_data_offset is present.
    pub fn base_data_offset_present(&self) -> bool {
        self.contains(Self::BASE_DATA_OFFSET_PRESENT)
    }

    /// Returns true if sample_description_index is present.
    pub fn sample_description_index_present(&self) -> bool {
        self.contains(Self::SAMPLE_DESCRIPTION_INDEX_PRESENT)
    }

    /// Returns true if default_sample_duration is present.
    pub fn default_sample_duration_present(&self) -> bool {
        self.contains(Self::DEFAULT_SAMPLE_DURATION_PRESENT)
    }

    /// Returns true if default_sample_size is present.
    pub fn default_sample_size_present(&self) -> bool {
        self.contains(Self::DEFAULT_SAMPLE_SIZE_PRESENT)
    }

    /// Returns true if default_sample_flags is present.
    pub fn default_sample_flags_present(&self) -> bool {
        self.contains(Self::DEFAULT_SAMPLE_FLAGS_PRESENT)
    }

    /// Returns true if duration_is_empty is set.
    pub fn duration_is_empty(&self) -> bool {
        self.contains(Self::DURATION_IS_EMPTY)
    }

    /// Returns true if default_base_is_moof is set.
    pub fn default_base_is_moof(&self) -> bool {
        self.contains(Self::DEFAULT_BASE_IS_MOOF)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_minimal() {
        let original = TfhdBox {
            version: 0,
            flags: TfhdFlags::DEFAULT_BASE_IS_MOOF,
            track_id: 1,
            base_data_offset: None,
            sample_description_index: None,
            default_sample_duration: None,
            default_sample_size: None,
            default_sample_flags: None,
        };

        let mut buf = vec![0u8; 32];
        original.encode(&mut buf).unwrap();

        let parsed = TfhdBox::decode(&buf).unwrap();
        assert_eq!(parsed.track_id, original.track_id);
        assert!(parsed.flags.default_base_is_moof());
    }

    #[test]
    fn round_trip_all_optional() {
        let mut original = TfhdBox {
            version: 0,
            flags: TfhdFlags::empty(),
            track_id: 3,
            base_data_offset: Some(100),
            sample_description_index: Some(1),
            default_sample_duration: Some(1000),
            default_sample_size: Some(512),
            default_sample_flags: Some(0x02000000),
        };
        original.flags = original.compute_flags();

        let mut buf = vec![0u8; 32];
        original.encode(&mut buf).unwrap();

        let parsed = TfhdBox::decode(&buf).unwrap();
        assert_eq!(parsed, original);
    }
}
