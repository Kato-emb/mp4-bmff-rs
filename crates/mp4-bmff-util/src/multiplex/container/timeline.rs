use alloc::vec::Vec;

use core::ops::Range;

use mp4_bmff::boxes::bmff::{
    CttsEntry, //
    SdtpEntry,
    StssEntry,
    SttsEntry,
};

#[derive(Debug, Clone)]
pub struct Timeline {
    pub(crate) stts_entries: Vec<SttsEntry>,
    pub(crate) ctts_entries: Option<Vec<CttsEntry>>,
    pub(crate) stss_entries: Option<Vec<StssEntry>>,
    pub(crate) sdtp_entries: Option<Vec<SdtpEntry>>,
}

impl Timeline {
    /// Retrieves a slice of the timeline entries based on the provided index range.
    pub fn get(&self, index: Range<usize>) -> Option<Timeline> {
        let stts_entries = self.stts_entries.get(index.clone())?.to_vec();
        let ctts_entries = self
            .ctts_entries
            .as_ref()
            .and_then(|ctts| ctts.get(index.clone()).map(|e| e.to_vec()));
        let stss_entries = self
            .stss_entries
            .as_ref()
            .and_then(|stss| stss.get(index.clone()).map(|e| e.to_vec()));
        let sdtp_entries = self
            .sdtp_entries
            .as_ref()
            .and_then(|sdtp| sdtp.get(index.clone()).map(|e| e.to_vec()));

        Some(Timeline {
            stts_entries,
            ctts_entries,
            stss_entries,
            sdtp_entries,
        })
    }

    pub fn append(&mut self, other: Timeline) {}
}
