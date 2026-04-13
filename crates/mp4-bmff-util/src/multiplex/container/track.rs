use alloc::vec::Vec;

use mp4_bmff::boxes::bmff::ElstEntry;
use mp4_bmff::types::LanguageCode;
use mp4_bmff::types::Matrix;

use super::Timescale;
use super::TrackId;

use super::DataLayout;
use super::SampleDescription;
use super::Timeline;

#[derive(Debug, Clone)]
pub struct Track {
    pub(crate) track_id: TrackId,
    pub(crate) timescale: Timescale,
    pub(crate) language: LanguageCode,
    pub(crate) matrix: Matrix,
    pub(crate) alternate_group: i16,
    pub(crate) descriptions: Vec<SampleDescription>,
    pub(crate) timeline: Timeline,
    pub(crate) data_layout: DataLayout,
    pub(crate) edit_list: Option<Vec<ElstEntry>>,
}

impl Track {
    /// Returns the sample descriptions for this track.
    pub fn descriptions(&self) -> &[SampleDescription] {
        &self.descriptions
    }

    /// Returns the timeline for this track.
    pub fn timeline(&self) -> &Timeline {
        &self.timeline
    }

    /// Returns the data layout for this track.
    pub fn data_layout(&self) -> &DataLayout {
        &self.data_layout
    }

    pub(crate) fn edit_duration(&self) -> Option<u64> {
        self.edit_list.as_ref().and_then(|edits| {
            edits
                .iter()
                .find(|edit| edit.media_time != -1)
                .map(|edit| edit.segment_duration)
        })
    }
}
