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
    pub(crate) data_layouts: Vec<DataLayout>,
    pub(crate) edit_list: Option<Vec<ElstEntry>>,
}
