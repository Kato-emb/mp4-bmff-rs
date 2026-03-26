use core::error;
use core::fmt;

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
