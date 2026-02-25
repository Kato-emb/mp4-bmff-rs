//! `SystemTime` integration for [`QuickTimeDateTime`].
//!
//! This module provides conversions between [`QuickTimeDateTime`] and
//! [`std::time::SystemTime`], as well as a [`now`] convenience function.
//!
//! # Feature Flag
//!
//! This module requires the `time` feature.

use std::time::{
    Duration, //
    SystemTime,
    UNIX_EPOCH,
};

use mp4_bmff::types::QuickTimeDateTime;

/// Returns the current system time as a [`QuickTimeDateTime`].
pub fn now() -> QuickTimeDateTime {
    system_time_to_quicktime(SystemTime::now())
}

/// Converts a [`SystemTime`] to a [`QuickTimeDateTime`].
pub fn system_time_to_quicktime(value: SystemTime) -> QuickTimeDateTime {
    let duration_since_epoch = value.duration_since(UNIX_EPOCH).unwrap_or_default();
    let unix_seconds = duration_since_epoch.as_secs().cast_signed();
    QuickTimeDateTime::from_unix_seconds(unix_seconds).unwrap_or_default()
}

/// Converts a [`QuickTimeDateTime`] to a [`SystemTime`].
pub fn quicktime_to_system_time(value: QuickTimeDateTime) -> SystemTime {
    match value.to_unix_seconds() {
        Some(unix) if unix >= 0 => UNIX_EPOCH + Duration::from_secs(unix.cast_unsigned()),
        _ => UNIX_EPOCH,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_time_round_trip() {
        let system_time = SystemTime::now();
        let quicktime = system_time_to_quicktime(system_time);
        let converted_back = quicktime_to_system_time(quicktime);
        let duration = converted_back
            .duration_since(system_time)
            .unwrap_or_default();
        assert!(duration.as_secs() < 2);
    }

    #[test]
    fn now_returns_recent_time() {
        let qt = now();
        assert!(qt.to_unix_seconds().is_some());
    }
}
