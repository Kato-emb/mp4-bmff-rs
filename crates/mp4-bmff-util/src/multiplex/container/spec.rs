use mp4_bmff::boxes::bmff::{CttsEntry, SdtpEntry, StssEntry, StszEntry, SttsEntry};

#[derive(Debug, Clone, Default)]
pub struct MediaSpec {
    pub(crate) stts_entries: Vec<SttsEntry>,
    pub(crate) stsz_entries: Vec<StszEntry>,
    pub(crate) ctts_entries: Option<Vec<CttsEntry>>,
    pub(crate) stss_entries: Option<Vec<StssEntry>>,
    pub(crate) sdtp_entries: Option<Vec<SdtpEntry>>,
}
