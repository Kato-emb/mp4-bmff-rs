//! MP4 muxer implementation.
//!
//! This module provides [`Muxer`] for non-fragmented MP4 and [`FragmentedMuxer`]
//! for fragmented MP4 (fMP4) output. Both are constructed through a shared
//! [`Builder`] that configures movie-level timescale and per-track settings.
//!
//! The muxers accept [`Sample`](super::Sample) values and produce the
//! corresponding box structures (`MoovBox`, `MoofBox`) ready for encoding.

use core::num::NonZeroU32;

use alloc::vec::Vec;

use mp4_bmff::types::*;

use super::Result;
use super::error::*;

use super::EditSegment;
use super::Sample;
use super::SampleDescription;
use super::Track;
use super::TrackId;

mod assemble;

/// A builder for configuring and constructing a [`Muxer`] or [`FragmentedMuxer`].
#[derive(Debug)]
#[must_use = "Builder is a builder for configuring the Muxer; call build() to finalize the Muxer"]
pub struct Builder {
    timescale: NonZeroU32,
    tracks: Vec<Track>,
}

impl Builder {
    /// Creates a new `Builder` with the specified timescale for the movie.
    pub fn new(timescale: u32) -> Result<Self> {
        let timescale = NonZeroU32::new(timescale).ok_or(
            Error::new(ErrorKind::InvalidInput).with_message("Timescale must be non-zero"),
        )?;

        Ok(Self {
            timescale,
            tracks: Vec::new(),
        })
    }

    /// Adds a new track to the movie with the given timescale and media definition, returning the assigned track ID.
    pub fn add_track(
        &mut self,
        timescale: u32,
        media: SampleDescription,
    ) -> Result<TrackBuilder<'_>> {
        let track_id = self.next_track_id().ok_or(
            Error::new(ErrorKind::Overflow)
                .with_message("Exceeded maximum track ID limit of 2^32 - 1"),
        )?;
        let timescale = NonZeroU32::new(timescale).ok_or(
            Error::new(ErrorKind::InvalidInput).with_message("Timescale must be non-zero"),
        )?;
        let track = Track::new(track_id, timescale, media);

        Ok(TrackBuilder {
            builder: self,
            track,
        })
    }

    fn next_track_id(&self) -> Option<TrackId> {
        let next_id = match self.tracks.iter().map(|s| s.id).max() {
            Some(max_id) => max_id.get().checked_add(1),
            None => Some(1),
        };

        next_id.and_then(TrackId::new)
    }

    /// Builds the `Muxer` from the current state of the `Builder`.
    pub fn build(self) -> Result<Muxer> {
        let next_track_id = self.next_track_id().ok_or(
            Error::new(ErrorKind::Overflow)
                .with_message("Exceeded maximum track ID limit of 2^32 - 1"),
        )?;

        let tracks = self
            .tracks
            .into_iter()
            .map(|t| MuxerTrack {
                track: t,
                pending_chunks: Vec::new(),
            })
            .collect();

        Ok(Muxer {
            timescale: self.timescale,
            next_track_id,
            tracks,
        })
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

    /// Finalizes the track and adds it to the builder, returning the assigned track ID.
    pub fn build(self) -> TrackId {
        let track_id = self.track.id;
        self.builder.tracks.push(self.track);
        track_id
    }
}

/// A muxer for producing non-fragmented MP4 files.
#[derive(Debug)]
pub struct Muxer {
    timescale: NonZeroU32,
    next_track_id: TrackId,
    tracks: Vec<MuxerTrack>,
}

impl Muxer {
    /// Creates a new [`Builder`] with the specified movie timescale.
    pub fn builder(timescale: u32) -> Result<Builder> {
        Builder::new(timescale)
    }

    /// Adds a chunk of samples to the specified track in the current fragment.
    pub fn add_chunk(&mut self, track_id: TrackId, samples: Vec<Sample>) -> Result<()> {
        if samples.is_empty() {
            return Err(Error::new(ErrorKind::InvalidInput).with_message(
                "Chunk must contain at least one sample; empty chunks are not allowed",
            ));
        }

        let Some(track) = self.get_track_mut(track_id) else {
            return Err(Error::new(ErrorKind::TrackNotFound(track_id)));
        };

        if let Some(last_chunk) = track.pending_chunks.last() {
            if samples.first().unwrap().dts_ns() < last_chunk.last().unwrap().dts_ns() {
                return Err(Error::new(ErrorKind::InvalidInput).with_message(
                "Chunk's first sample DTS must not precede the previous chunk's last sample DTS",
            ));
            }
        }

        track.pending_chunks.push(samples);
        Ok(())
    }

    fn get_track_mut(&mut self, track_id: TrackId) -> Option<&mut MuxerTrack> {
        self.tracks.iter_mut().find(|t| t.track.id == track_id)
    }
}

#[derive(Debug)]
struct MuxerTrack {
    track: Track,
    pending_chunks: Vec<Vec<Sample>>,
}
