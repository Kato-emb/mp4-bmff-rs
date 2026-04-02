//! MP4 muxer implementation.
//!
//! This module provides [`Muxer`] for non-fragmented MP4 and [`FragmentedMuxer`]
//! for fragmented MP4 (fMP4) output. Both are constructed through a shared
//! [`Builder`] that configures movie-level timescale and per-track settings.
//!
//! The muxers accept [`Sample`](super::Sample) values and produce the
//! corresponding box structures (`MoovBox`, `MoofBox`) ready for encoding.

use core::num::NonZeroU32;
use core::time::Duration;

use alloc::vec::Vec;

use mp4_bmff::boxes::bmff::*;
use mp4_bmff::types::*;

use super::Sample;
use super::TrackId;

use super::Result;
use super::error::*;

use super::AudioSampleDescription;
use super::EditSegment;
use super::MediaDefinition;
use super::VisualSampleDescription;

use assemble::{build_moof, build_moov, build_mvex};
use types::{Movie, Track};

mod assemble;
mod types;

/// A builder for configuring and constructing a [`Muxer`] or [`FragmentedMuxer`].
#[derive(Debug)]
#[must_use = "Builder is a builder for configuring the Muxer; call build() to finalize the Muxer"]
pub struct Builder {
    movie: Movie,
}

impl Builder {
    /// Creates a new `Builder` with the specified timescale for the movie.
    pub fn new(timescale: u32) -> Result<Self> {
        let timescale = NonZeroU32::new(timescale).ok_or(
            Error::new(ErrorKind::InvalidInput).with_message("Timescale must be non-zero"),
        )?;

        Ok(Self {
            movie: Movie::new(timescale),
        })
    }

    /// Adds a new track to the movie with the given timescale and media definition, returning the assigned track ID.
    pub fn add_track(
        &mut self,
        timescale: u32,
        media: MediaDefinition,
    ) -> Result<TrackBuilder<'_>> {
        let track_id = self.movie.next_track_id();
        let timescale = NonZeroU32::new(timescale).ok_or(
            Error::new(ErrorKind::InvalidInput).with_message("Timescale must be non-zero"),
        )?;
        let track = Track::new(track_id, timescale, media);

        Ok(TrackBuilder {
            builder: self,
            track,
        })
    }

    /// Builds the `Muxer` from the current state of the `Builder`.
    pub fn build(self) -> Result<Muxer> {
        Ok(Muxer { movie: self.movie })
    }

    /// Builds a `FragmentedMuxer` along with the initial `MoovBox` for fragmented MP4 output.
    pub fn build_fragmented(self) -> Result<(MoovBox, FragmentedMuxer)> {
        let mut moov = build_moov(&self.movie)?;
        moov.mvex = Some(build_mvex(&self.movie)?);

        Ok((
            moov,
            FragmentedMuxer {
                movie: self.movie,
                sequence_number: 0,
            },
        ))
    }
}

/// A builder for configuring and adding a track to the movie before finalizing the `Muxer`.
#[derive(Debug)]
#[must_use = "TrackBuilder is a builder for configuring a track; call build() to finalize the track and add it to the movie"]
pub struct TrackBuilder<'a> {
    builder: &'a mut Builder,
    track: Track,
}

impl TrackBuilder<'_> {
    /// Sets the language code for the track and returns the updated `TrackBuilder`.
    pub fn language(mut self, language: LanguageCode) -> Self {
        self.track.language = language;
        self
    }

    /// Sets the matrix for the track and returns the updated `TrackBuilder`.
    pub fn matrix(mut self, matrix: Matrix) -> Self {
        self.track.matrix = matrix;
        self
    }

    /// Sets the alternate group for the track and returns the updated `TrackBuilder`.
    pub fn alternate_group(mut self, group: i16) -> Self {
        self.track.alternate_group = group;
        self
    }

    /// Sets the edit list for the track and returns the updated `TrackBuilder`.
    pub fn edit_list(mut self, segments: Vec<EditSegment>) -> Self {
        self.track.edit_list = Some(segments);
        self
    }

    /// Sets the default sample duration for the track and returns the updated `TrackBuilder`.
    pub fn sample_default_duration(mut self, duration: Duration) -> Self {
        self.track.defaults.sample_duration = Some(duration);
        self
    }

    /// Sets the default sample size for the track and returns the updated `TrackBuilder`.
    pub fn sample_default_size(mut self, size: u32) -> Self {
        self.track.defaults.sample_size = Some(size);
        self
    }

    /// Sets the default sample flags for the track and returns the updated `TrackBuilder`.
    pub fn sample_default_flags(mut self, flags: SampleFlags) -> Self {
        self.track.defaults.sample_flags = Some(flags);
        self
    }

    /// Finalizes the track and adds it to the builder, returning the assigned track ID.
    pub fn build(self) -> TrackId {
        let track_id = self.track.id;
        self.builder.movie.tracks.push(self.track);
        track_id
    }
}

/// A muxer for producing non-fragmented MP4 files.
#[derive(Debug)]
pub struct Muxer {
    movie: Movie,
}

impl Muxer {
    /// Creates a new [`Builder`] with the specified movie timescale.
    pub fn builder(timescale: u32) -> Result<Builder> {
        Builder::new(timescale)
    }

    /// Adds a sample to the specified track in the movie, associating it with the given data offset in the `mdat` box.
    pub fn add_sample(&mut self, track_id: TrackId, sample: Sample) -> Result<()> {
        self.movie.add_sample(track_id, sample)
    }

    /// Finalizes the muxer and produces the `moov` box.
    pub fn finalize(self) -> Result<MoovBox> {
        if self
            .movie
            .tracks
            .iter()
            .any(|t| t.sample_table.chunks.is_empty())
        {
            return Err(Error::new(ErrorKind::InvalidInput).with_message(
                "Non-fragmented muxer cannot have tracks with no samples; all tracks must have at least one sample",
            ));
        }

        build_moov(&self.movie)
    }
}

/// A muxer for producing fragmented MP4 (fMP4) files.
#[derive(Debug)]
pub struct FragmentedMuxer {
    movie: Movie,
    sequence_number: u32,
}

impl FragmentedMuxer {
    /// Creates a new [`Builder`] with the specified movie timescale.
    pub fn builder(timescale: u32) -> Result<Builder> {
        Builder::new(timescale)
    }

    /// Adds a sample to the specified track in the movie, associating it with the given data offset in the `mdat` box.
    pub fn add_sample(&mut self, track_id: TrackId, sample: Sample) -> Result<()> {
        self.movie.add_sample(track_id, sample)
    }

    /// Flushes the accumulated samples into a `moof` box and resets the fragment state.
    pub fn flush_fragment(&mut self) -> Result<MoofBox> {
        self.sequence_number = self
            .sequence_number
            .checked_add(1)
            .ok_or(ErrorKind::Overflow)?;

        let moof = build_moof(&self.movie, self.sequence_number)?;

        self.clear();
        Ok(moof)
    }

    fn clear(&mut self) {
        for track in self.movie.tracks.iter_mut() {
            track.sample_table.chunks.clear();
        }
    }
}
