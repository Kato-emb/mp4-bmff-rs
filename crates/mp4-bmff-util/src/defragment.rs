//! Utilities for defragmenting media data in MP4 files.

use alloc::vec::Vec;

use mp4_bmff::boxes::bmff::{MoofBox, MoovBox};

use crate::sample_table::{SampleTableExt, StblBuilder};

#[derive(Debug)]
struct TrackState {
    track_id: u32,
    stbl: StblBuilder,
}

/// A defragmenter for MP4 files, which can be used to rearrange media data into a more contiguous layout.
#[derive(Debug)]
pub struct Defragmenter {
    moov: MoovBox,
    tracks: Vec<TrackState>,
}

impl Defragmenter {
    /// Creates a new `Defragmenter` from the given `MoovBox`.
    pub fn new(moov: MoovBox) -> Self {
        let tracks = moov
            .traks
            .iter()
            .map(|trak| {
                let track_id = trak.tkhd.track_id;
                let stsd = trak.mdia.minf.stbl.stsd.clone();
                let stbl = StblBuilder::new(stsd);
                TrackState { track_id, stbl }
            })
            .collect();

        Self { moov, tracks }
    }

    /// Adds a media segment (represented by a `MoofBox`) to the defragmenter, updating the sample tables accordingly.
    pub fn add_media_segment(
        &mut self,
        moof: MoofBox,
        data_offset: u32,
    ) -> Result<(), mp4_bmff::Error> {
        for traf in moof.trafs {
            let track_id = traf.tfhd.track_id;

            if let Some(track) = self.tracks.iter_mut().find(|t| t.track_id == track_id) {
                let description_index = traf.tfhd.sample_description_index.unwrap_or(1);
                let samples = traf.resolved_samples()?;
                track.stbl.add_chunk(
                    samples.map(|s| s.sample),
                    description_index,
                    data_offset.into(),
                );
            }
        }

        Ok(())
    }

    /// Finalizes the defragmentation process and returns an updated `MoovBox` with the new sample tables.
    pub fn finalize(mut self) -> MoovBox {
        for (trak, track_state) in self.moov.traks.iter_mut().zip(self.tracks) {
            trak.mdia.minf.stbl = track_state.stbl.build();
        }

        self.moov.mvex = None;
        self.moov
    }
}
