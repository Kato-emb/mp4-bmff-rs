//! Common types used by multiple BMFF boxes.
//!

use core::fmt;

/// is_leading (2 bits)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum IsLeading {
    #[default]
    Unknown = 0,
    HasDependencyBefore = 1,
    NotLeading = 2,
    NoDependencyBefore = 3,
}

/// sample_depends_on (2 bits)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum SampleDependsOn {
    #[default]
    Unknown = 0,
    Others = 1,
    NotOthers = 2,
    Reserved = 3,
}

/// sample_is_depended_on (2 bits)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum SampleIsDependedOn {
    #[default]
    Unknown = 0,
    Yes = 1,
    No = 2,
    Reserved = 3,
}

/// sample_has_redundancy (2 bits)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum SampleHasRedundancy {
    #[default]
    Unknown = 0,
    Redundant = 1,
    NotRedundant = 2,
    Reserved = 3,
}

/// Sample flags as defined in ISO 14496-12 §8.8.3.
///
/// Used by `trex`, `tfhd`, and `trun` boxes.
#[must_use]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SampleFlags(u32);

const fn extract_2bit(raw: u32, shift: u32) -> u8 {
    ((raw >> shift) & 0x03) as u8
}

const fn embed_2bit(raw: u32, shift: u32, value: u8) -> u32 {
    (raw & !(0x03 << shift)) | ((value as u32 & 0x03) << shift)
}

impl SampleFlags {
    /// Creates a new `SampleFlags` instance from raw flags, masking to 24 bits.
    #[inline]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the raw flags value as a `u32`.
    #[inline]
    pub const fn to_raw(&self) -> u32 {
        self.0
    }

    /// Creates a `SampleFlags` instance representing a sync sample (keyframe).
    pub const fn sync() -> Self {
        Self(0)
            .with_sample_depends_on(SampleDependsOn::NotOthers)
            .with_sample_is_non_sync(false)
    }

    /// Creates a `SampleFlags` instance representing a non-sync sample (not a keyframe).
    pub const fn non_sync() -> Self {
        Self(0)
            .with_sample_depends_on(SampleDependsOn::Others)
            .with_sample_is_non_sync(true)
    }

    /// Returns [IsLeading] value extracted from the flags.
    #[inline]
    pub const fn is_leading(&self) -> IsLeading {
        match extract_2bit(self.0, 26) {
            1 => IsLeading::HasDependencyBefore,
            2 => IsLeading::NotLeading,
            3 => IsLeading::NoDependencyBefore,
            _ => IsLeading::Unknown,
        }
    }

    /// Sets the [IsLeading] value in the flags.
    #[inline]
    pub const fn with_is_leading(self, v: IsLeading) -> Self {
        Self(embed_2bit(self.0, 26, v as u8))
    }

    /// Returns [SampleDependsOn] value extracted from the flags.
    #[inline]
    pub const fn sample_depends_on(&self) -> SampleDependsOn {
        match extract_2bit(self.0, 24) {
            1 => SampleDependsOn::Others,
            2 => SampleDependsOn::NotOthers,
            3 => SampleDependsOn::Reserved,
            _ => SampleDependsOn::Unknown,
        }
    }

    /// Sets the [SampleDependsOn] value in the flags.
    #[inline]
    pub const fn with_sample_depends_on(self, v: SampleDependsOn) -> Self {
        Self(embed_2bit(self.0, 24, v as u8))
    }

    /// Returns [SampleIsDependedOn] value extracted from the flags.
    #[inline]
    pub const fn sample_is_depended_on(&self) -> SampleIsDependedOn {
        match extract_2bit(self.0, 22) {
            1 => SampleIsDependedOn::Yes,
            2 => SampleIsDependedOn::No,
            3 => SampleIsDependedOn::Reserved,
            _ => SampleIsDependedOn::Unknown,
        }
    }

    /// Sets the [SampleIsDependedOn] value in the flags.
    #[inline]
    pub const fn with_sample_is_depended_on(self, v: SampleIsDependedOn) -> Self {
        Self(embed_2bit(self.0, 22, v as u8))
    }

    /// Returns [SampleHasRedundancy] value extracted from the flags.
    #[inline]
    pub const fn sample_has_redundancy(&self) -> SampleHasRedundancy {
        match extract_2bit(self.0, 20) {
            1 => SampleHasRedundancy::Redundant,
            2 => SampleHasRedundancy::NotRedundant,
            3 => SampleHasRedundancy::Reserved,
            _ => SampleHasRedundancy::Unknown,
        }
    }

    /// Sets the [SampleHasRedundancy] value in the flags.
    #[inline]
    pub const fn with_sample_has_redundancy(self, v: SampleHasRedundancy) -> Self {
        Self(embed_2bit(self.0, 20, v as u8))
    }

    /// Returns the sample padding value (0-7) extracted from the flags.
    #[inline]
    pub const fn sample_padding_value(&self) -> u8 {
        ((self.0 >> 17) & 0x07) as u8
    }

    /// Sets the sample padding value (0-7) in the flags.
    #[inline]
    pub const fn with_sample_padding_value(self, v: u8) -> Self {
        Self((self.0 & !(0x07 << 17)) | (((v as u32) & 0x07) << 17))
    }

    /// Returns `true` if the sample is a non-sync sample (not a keyframe).
    #[inline]
    pub const fn sample_is_non_sync(&self) -> bool {
        (self.0 >> 16) & 0x01 == 1
    }

    /// Sets the non-sync sample (not a keyframe) flag.
    #[inline]
    pub const fn with_sample_is_non_sync(self, v: bool) -> Self {
        Self((self.0 & !(0x01 << 16)) | ((v as u32) << 16))
    }

    /// Returns `true` if the sample is a sync sample (keyframe).
    #[inline]
    pub const fn is_sync(self) -> bool {
        !self.sample_is_non_sync()
    }

    /// Returns the sample degradation priority value (0-65535) extracted from the flags.
    #[inline]
    pub const fn sample_degradation_priority(self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }

    /// Sets the sample degradation priority value (0-65535) in the flags.
    #[inline]
    pub const fn with_sample_degradation_priority(self, v: u16) -> Self {
        Self((self.0 & !0xFFFF) | v as u32)
    }
}

impl fmt::Debug for SampleFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SampleFlags")
            .field("raw", &format_args!("{:#010x}", self.0))
            .field("is_sync", &self.is_sync())
            .field("depends_on", &self.sample_depends_on())
            .field("is_depended_on", &self.sample_is_depended_on())
            .field("is_leading", &self.is_leading())
            .field("has_redundancy", &self.sample_has_redundancy())
            .field("degradation_priority", &self.sample_degradation_priority())
            .finish()
    }
}

impl fmt::Display for SampleFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_sync() {
            write!(f, "[sync")?;
        } else {
            write!(f, "[non-sync")?;
        }
        match self.sample_depends_on() {
            SampleDependsOn::Unknown => {}
            v => write!(f, " depends={v:?}")?,
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── extract_2bit / embed_2bit helpers ────────────────────────────

    #[test]
    fn extract_2bit_all_positions() {
        // Place 0b11 at shift 0
        assert_eq!(extract_2bit(0x03, 0), 3);
        // Place 0b10 at shift 24
        assert_eq!(extract_2bit(0x02_00_00_00, 24), 2);
        // Ensure surrounding bits are masked out
        assert_eq!(extract_2bit(0xFF_FF_FF_FF, 4), 3);
    }

    #[test]
    fn embed_2bit_round_trip() {
        let raw = 0xAB_CD_EF_00u32;
        for shift in (0..30).step_by(2) {
            for value in 0..=3u8 {
                let updated = embed_2bit(raw, shift, value);
                assert_eq!(extract_2bit(updated, shift), value);
            }
        }
    }

    #[test]
    fn embed_2bit_preserves_other_bits() {
        let raw = 0xFF_FF_FF_FF;
        let updated = embed_2bit(raw, 20, 0);
        // Only bits 20-21 should be cleared
        assert_eq!(updated, 0xFF_CF_FF_FF);
    }

    // ── SampleFlags: from_raw / to_raw ──────────────────────────────

    #[test]
    fn from_raw_to_raw_round_trip() {
        let raw = 0x07_AB_CD_EF;
        let flags = SampleFlags::from_raw(raw);
        assert_eq!(flags.to_raw(), raw);
    }

    #[test]
    fn default_is_zero() {
        let flags = SampleFlags::default();
        assert_eq!(flags.to_raw(), 0);
    }

    // ── SampleFlags: sync / non_sync constructors ───────────────────

    #[test]
    fn sync_flags() {
        let flags = SampleFlags::sync();
        assert!(flags.is_sync());
        assert!(!flags.sample_is_non_sync());
        assert_eq!(flags.sample_depends_on(), SampleDependsOn::NotOthers);
    }

    #[test]
    fn non_sync_flags() {
        let flags = SampleFlags::non_sync();
        assert!(!flags.is_sync());
        assert!(flags.sample_is_non_sync());
        assert_eq!(flags.sample_depends_on(), SampleDependsOn::Others);
    }

    // ── SampleFlags: is_leading round-trip ──────────────────────────

    #[test]
    fn is_leading_round_trip() {
        let variants = [
            IsLeading::Unknown,
            IsLeading::HasDependencyBefore,
            IsLeading::NotLeading,
            IsLeading::NoDependencyBefore,
        ];
        for v in variants {
            let flags = SampleFlags::default().with_is_leading(v);
            assert_eq!(flags.is_leading(), v);
        }
    }

    // ── SampleFlags: sample_depends_on round-trip ───────────────────

    #[test]
    fn sample_depends_on_round_trip() {
        let variants = [
            SampleDependsOn::Unknown,
            SampleDependsOn::Others,
            SampleDependsOn::NotOthers,
            SampleDependsOn::Reserved,
        ];
        for v in variants {
            let flags = SampleFlags::default().with_sample_depends_on(v);
            assert_eq!(flags.sample_depends_on(), v);
        }
    }

    // ── SampleFlags: sample_is_depended_on round-trip ───────────────

    #[test]
    fn sample_is_depended_on_round_trip() {
        let variants = [
            SampleIsDependedOn::Unknown,
            SampleIsDependedOn::Yes,
            SampleIsDependedOn::No,
            SampleIsDependedOn::Reserved,
        ];
        for v in variants {
            let flags = SampleFlags::default().with_sample_is_depended_on(v);
            assert_eq!(flags.sample_is_depended_on(), v);
        }
    }

    // ── SampleFlags: sample_has_redundancy round-trip ────────────────

    #[test]
    fn sample_has_redundancy_round_trip() {
        let variants = [
            SampleHasRedundancy::Unknown,
            SampleHasRedundancy::Redundant,
            SampleHasRedundancy::NotRedundant,
            SampleHasRedundancy::Reserved,
        ];
        for v in variants {
            let flags = SampleFlags::default().with_sample_has_redundancy(v);
            assert_eq!(flags.sample_has_redundancy(), v);
        }
    }

    // ── SampleFlags: sample_padding_value ────────────────────────────

    #[test]
    fn sample_padding_value_round_trip() {
        for v in 0..=7u8 {
            let flags = SampleFlags::default().with_sample_padding_value(v);
            assert_eq!(flags.sample_padding_value(), v);
        }
    }

    #[test]
    fn sample_padding_value_masks_to_3bits() {
        // 0xFF should be masked to 0x07
        let flags = SampleFlags::default().with_sample_padding_value(0xFF);
        assert_eq!(flags.sample_padding_value(), 7);
    }

    // ── SampleFlags: sample_is_non_sync / is_sync ───────────────────

    #[test]
    fn sample_is_non_sync_round_trip() {
        let flags_sync = SampleFlags::default().with_sample_is_non_sync(false);
        assert!(!flags_sync.sample_is_non_sync());
        assert!(flags_sync.is_sync());

        let flags_non_sync = SampleFlags::default().with_sample_is_non_sync(true);
        assert!(flags_non_sync.sample_is_non_sync());
        assert!(!flags_non_sync.is_sync());
    }

    // ── SampleFlags: sample_degradation_priority ────────────────────

    #[test]
    fn sample_degradation_priority_round_trip() {
        for &v in &[0u16, 1, 255, 1000, 65535] {
            let flags = SampleFlags::default().with_sample_degradation_priority(v);
            assert_eq!(flags.sample_degradation_priority(), v);
        }
    }

    // ── Fields do not clobber each other ─────────────────────────────

    #[test]
    fn fields_are_independent() {
        let flags = SampleFlags::default()
            .with_is_leading(IsLeading::NoDependencyBefore)
            .with_sample_depends_on(SampleDependsOn::Others)
            .with_sample_is_depended_on(SampleIsDependedOn::Yes)
            .with_sample_has_redundancy(SampleHasRedundancy::Redundant)
            .with_sample_padding_value(5)
            .with_sample_is_non_sync(true)
            .with_sample_degradation_priority(42);

        assert_eq!(flags.is_leading(), IsLeading::NoDependencyBefore);
        assert_eq!(flags.sample_depends_on(), SampleDependsOn::Others);
        assert_eq!(flags.sample_is_depended_on(), SampleIsDependedOn::Yes);
        assert_eq!(
            flags.sample_has_redundancy(),
            SampleHasRedundancy::Redundant
        );
        assert_eq!(flags.sample_padding_value(), 5);
        assert!(flags.sample_is_non_sync());
        assert_eq!(flags.sample_degradation_priority(), 42);
    }

    #[test]
    fn overwrite_single_field_preserves_others() {
        let original = SampleFlags::default()
            .with_is_leading(IsLeading::NotLeading)
            .with_sample_depends_on(SampleDependsOn::NotOthers)
            .with_sample_degradation_priority(999);

        let modified = original.with_sample_depends_on(SampleDependsOn::Others);

        // Changed field
        assert_eq!(modified.sample_depends_on(), SampleDependsOn::Others);
        // Untouched fields
        assert_eq!(modified.is_leading(), IsLeading::NotLeading);
        assert_eq!(modified.sample_degradation_priority(), 999);
    }

    // ── Known bit pattern ────────────────────────────────────────────

    #[test]
    fn known_bit_pattern() {
        // Build a known pattern by hand:
        //   is_leading           = NoDependencyBefore (3) at bits 26-27
        //   sample_depends_on    = Others (1)             at bits 24-25
        //   sample_is_depended_on= No (2)                 at bits 22-23
        //   sample_has_redundancy= Redundant (1)          at bits 20-21
        //   sample_padding_value = 7                      at bits 17-19
        //   sample_is_non_sync   = true (1)               at bit 16
        //   degradation_priority = 0x00FF                 at bits 0-15
        let raw: u32 =
            (3 << 26) | (1 << 24) | (2 << 22) | (1 << 20) | (7 << 17) | (1 << 16) | 0x00FF;

        let flags = SampleFlags::from_raw(raw);
        assert_eq!(flags.is_leading(), IsLeading::NoDependencyBefore);
        assert_eq!(flags.sample_depends_on(), SampleDependsOn::Others);
        assert_eq!(flags.sample_is_depended_on(), SampleIsDependedOn::No);
        assert_eq!(
            flags.sample_has_redundancy(),
            SampleHasRedundancy::Redundant
        );
        assert_eq!(flags.sample_padding_value(), 7);
        assert!(flags.sample_is_non_sync());
        assert_eq!(flags.sample_degradation_priority(), 0x00FF);
        assert_eq!(flags.to_raw(), raw);
    }
}
