//! QuickTime epoch timestamp type.
//!
//! BMFF stores creation and modification timestamps as seconds since
//! 1904-01-01T00:00:00Z, known as the "QuickTime" or "Mac" epoch. This
//! predates the Unix epoch (1970-01-01) by 66 years.
//!
//! # Epoch Comparison
//!
//! | Epoch | Date | Seconds Offset |
//! |-------|------|----------------|
//! | QuickTime | 1904-01-01 00:00:00 UTC | 0 |
//! | Unix | 1970-01-01 00:00:00 UTC | +2,082,844,800 |
//!
//! # Usage
//!
//! ```
//! use mp4_bmff::types::QuickTimeDateTime;
//!
//! // Create from QuickTime seconds
//! let timestamp = QuickTimeDateTime::from_quicktime_seconds(3_600_000_000);
//!
//! // Convert to Unix seconds
//! if let Some(unix) = timestamp.to_unix_seconds() {
//!     println!("Unix timestamp: {}", unix);
//! }
//! ```

use core::fmt;

/// Seconds offset between the QuickTime (1904) and Unix (1970) epochs.
const QUICKTIME_UNIX_OFFSET: i64 = 2_082_844_800;

/// Timestamp as seconds since 1904-01-01 UTC (QuickTime epoch).
///
/// This type represents timestamps as stored in BMFF `mvhd`, `tkhd`, and
/// `mdhd` boxes for creation and modification times.
///
/// # Conversions
///
/// - `from_quicktime_seconds()` / `to_quicktime_seconds()`: Raw epoch value.
/// - `from_unix_seconds()` / `to_unix_seconds()`: Unix epoch conversion.
/// - `now()`: Current time (requires `std` feature).
/// - `From<SystemTime>` / `Into<SystemTime>`: System time conversion (requires `std`).
///
/// # Example
///
/// ```
/// use mp4_bmff::types::QuickTimeDateTime;
///
/// // Unix timestamp 1600000000 (2020-09-13)
/// let qt = QuickTimeDateTime::from_unix_seconds(1_600_000_000).unwrap();
/// assert_eq!(qt.to_unix_seconds(), Some(1_600_000_000));
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct QuickTimeDateTime(u64);

impl QuickTimeDateTime {
    /// Construct from raw seconds since the QuickTime epoch.
    #[inline]
    pub const fn from_quicktime_seconds(seconds: u64) -> Self {
        Self(seconds)
    }

    /// Raw seconds since the QuickTime epoch.
    #[inline]
    pub const fn to_quicktime_seconds(self) -> u64 {
        self.0
    }

    /// Convert from Unix seconds if the value fits the QuickTime epoch range.
    #[inline]
    pub fn from_unix_seconds(unix: i64) -> Option<Self> {
        unix.checked_add(QUICKTIME_UNIX_OFFSET)
            .and_then(|v| u64::try_from(v).ok())
            .map(Self)
    }

    /// Convert back to Unix seconds, returning `None` if outside the `i64` range
    /// or predating the Unix epoch.
    #[inline]
    pub fn to_unix_seconds(self) -> Option<i64> {
        let offset = QUICKTIME_UNIX_OFFSET.cast_unsigned();
        if self.0 < offset {
            return None;
        }
        let unix = self.0 - offset;
        if unix > i64::MAX.cast_unsigned() {
            None
        } else {
            Some(unix.cast_signed())
        }
    }

    /// Get the current system time as a `QuickTimeDateTime`.
    #[cfg(feature = "std")]
    pub fn now() -> Self {
        std::time::SystemTime::now().into()
    }
}

impl fmt::Debug for QuickTimeDateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("QuickTimeDateTime").field(&self.0).finish()
    }
}

impl fmt::Display for QuickTimeDateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.to_unix_seconds() {
            Some(unix) => write!(f, "{} s since Unix epoch (UTC)", unix),
            None => write!(f, "<before Unix epoch>"),
        }
    }
}

impl From<QuickTimeDateTime> for u64 {
    fn from(value: QuickTimeDateTime) -> Self {
        value.to_quicktime_seconds()
    }
}

impl From<u64> for QuickTimeDateTime {
    fn from(seconds: u64) -> Self {
        Self::from_quicktime_seconds(seconds)
    }
}

#[cfg(feature = "std")]
impl From<std::time::SystemTime> for QuickTimeDateTime {
    fn from(value: std::time::SystemTime) -> Self {
        let duration_since_epoch = value
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let unix_seconds = duration_since_epoch.as_secs().cast_signed();
        QuickTimeDateTime::from_unix_seconds(unix_seconds).unwrap_or_default()
    }
}

#[cfg(feature = "std")]
impl From<QuickTimeDateTime> for std::time::SystemTime {
    fn from(value: QuickTimeDateTime) -> Self {
        match value.to_unix_seconds() {
            Some(unix) if unix >= 0 => {
                std::time::UNIX_EPOCH + core::time::Duration::from_secs(unix.cast_unsigned())
            }
            _ => std::time::UNIX_EPOCH,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_conversion_round_trip() {
        let unix = 1_600_000_000i64;
        let quicktime = QuickTimeDateTime::from_unix_seconds(unix).expect("convert");
        assert_eq!(quicktime.to_unix_seconds(), Some(unix));
    }

    #[test]
    fn unix_underflow_is_none() {
        let quicktime =
            QuickTimeDateTime::from_quicktime_seconds((QUICKTIME_UNIX_OFFSET - 1).cast_unsigned());
        assert_eq!(quicktime.to_unix_seconds(), None);
    }

    #[cfg(feature = "std")]
    #[test]
    fn system_time_conversion() {
        let system_time = std::time::SystemTime::now();
        let quicktime: QuickTimeDateTime = system_time.into();
        let converted_back: std::time::SystemTime = quicktime.into();
        let duration = converted_back
            .duration_since(system_time)
            .unwrap_or_default();
        assert!(duration.as_secs() < 2); // Allow up to 2 seconds difference
    }
}
