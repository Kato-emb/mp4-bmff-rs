use core::error;
use core::fmt;

use core::time::Duration;

#[derive(Debug)]
pub enum MuxError {
    Overflow,
    Bmff(mp4_bmff::Error),
}

impl fmt::Display for MuxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MuxError::Overflow => write!(f, "Overflow error"),
            MuxError::Bmff(e) => write!(f, "ISOBMFF error: {}", e),
        }
    }
}

impl error::Error for MuxError {}

impl From<mp4_bmff::Error> for MuxError {
    fn from(e: mp4_bmff::Error) -> Self {
        MuxError::Bmff(e)
    }
}

pub(super) fn duration_to_ticks(duration: Duration, timescale: u32) -> Option<u64> {
    let timescale = u64::from(timescale);
    let secs = duration.as_secs();
    let nanos = u64::from(duration.subsec_nanos());

    // nanos * ts は最大 ≈ 4.3 × 10¹⁸ で、u64::MAX より小さいためオーバーフローしない。
    secs.checked_mul(timescale)?
        .checked_add(nanos * timescale / 1_000_000_000)
}
