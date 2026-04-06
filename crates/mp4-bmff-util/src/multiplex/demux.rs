//! MP4 demuxer implementation.
//!
//! This module provides [`Demuxer`] for non-fragmented MP4 and [`FragmentedDemuxer`]
//! for fragmented MP4 (fMP4) input. Both accept box structures (`MoovBox`, `MoofBox`)
//! and produce [`Track`], [`Chunk`], and [`Sample`](super::Sample) values.

use core::num::NonZeroU32;

use alloc::vec::Vec;

use mp4_bmff::boxes::bmff::*;

use super::error::*;
use super::{Chunk, Result, Track, TrackId};

use disassemble::{disassemble_traf, disassemble_trak};
use types::{DemuxedTrack, TrackContext, TrackDefaults};

mod disassemble;
mod types;

/// A demuxer for reading non-fragmented MP4 files.
#[derive(Debug)]
pub struct Demuxer {
    tracks: Vec<DemuxedTrack>,
}

impl Demuxer {
    /// Creates a new `Demuxer` from a `MoovBox`, extracting track metadata and sample tables.
    pub fn new(moov: &MoovBox) -> Result<Self> {
        let movie_timescale = NonZeroU32::new(moov.mvhd.timescale).ok_or(
            Error::new(ErrorKind::InvalidInput)
                .with_message("Movie timescale must be non-zero"),
        )?;

        let tracks = moov
            .traks
            .iter()
            .map(|trak| {
                let (info, chunks) = disassemble_trak(trak, movie_timescale)?;
                Ok(DemuxedTrack { info, chunks })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self { tracks })
    }

    /// Returns the list of tracks in the movie.
    pub fn tracks(&self) -> impl Iterator<Item = &Track> {
        self.tracks.iter().map(|t| &t.info)
    }

    /// Returns the chunks for the specified track.
    pub fn chunks(&self, track_id: TrackId) -> Result<&[Chunk]> {
        self.tracks
            .iter()
            .find(|t| t.info.id == track_id)
            .map(|t| t.chunks.as_slice())
            .ok_or(ErrorKind::TrackNotFound(track_id).into())
    }
}

/// A demuxer for reading fragmented MP4 (fMP4) files.
#[derive(Debug)]
pub struct FragmentedDemuxer {
    tracks: Vec<TrackContext>,
}

impl FragmentedDemuxer {
    /// Creates a new `FragmentedDemuxer` from a `MoovBox` containing `mvex`.
    pub fn new(moov: &MoovBox) -> Result<Self> {
        let movie_timescale = NonZeroU32::new(moov.mvhd.timescale).ok_or(
            Error::new(ErrorKind::InvalidInput)
                .with_message("Movie timescale must be non-zero"),
        )?;

        let mvex = moov.mvex.as_ref().ok_or(
            Error::new(ErrorKind::InvalidInput)
                .with_message("Fragmented MP4 moov must contain mvex"),
        )?;

        let tracks = moov
            .traks
            .iter()
            .map(|trak| {
                let (info, _chunks) = disassemble_trak(trak, movie_timescale)?;

                // Find matching trex for this track
                let trex = mvex
                    .trexs
                    .iter()
                    .find(|t| t.track_id == info.id.get())
                    .ok_or_else(|| {
                        Error::new(ErrorKind::InvalidInput).with_message(format!(
                            "No trex box found for track {}",
                            info.id.get()
                        ))
                    })?;

                Ok(TrackContext {
                    info,
                    trex_defaults: TrackDefaults {
                        default_sample_duration: trex.default_sample_duration,
                        default_sample_size: trex.default_sample_size,
                        default_sample_flags: trex.default_sample_flags,
                    },
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self { tracks })
    }

    /// Returns the list of tracks in the movie.
    pub fn tracks(&self) -> impl Iterator<Item = &Track> {
        self.tracks.iter().map(|t| &t.info)
    }

    /// Reads a fragment (`MoofBox`) and returns the demuxed samples grouped by track.
    pub fn read_fragment(&self, moof: &MoofBox) -> Result<Fragment> {
        let mut track_chunks = Vec::with_capacity(moof.trafs.len());

        for traf in &moof.trafs {
            let track_id = traf.tfhd.track_id;
            let ctx = self.tracks.iter().find(|t| t.info.id.get() == track_id).ok_or(
                Error::new(ErrorKind::InvalidInput).with_message(format!(
                    "Fragment references unknown track {}",
                    track_id
                )),
            )?;

            let timescale = NonZeroU32::new(ctx.info.timescale()).ok_or(
                Error::new(ErrorKind::InvalidInput)
                    .with_message("Track timescale must be non-zero"),
            )?;

            let chunks = disassemble_traf(traf, timescale, &ctx.trex_defaults)?;
            let tid = TrackId::new(track_id).ok_or(
                Error::new(ErrorKind::InvalidInput)
                    .with_message("Track ID must be non-zero"),
            )?;
            track_chunks.push((tid, chunks));
        }

        Ok(Fragment { track_chunks })
    }
}

/// A demuxed fragment containing chunks grouped by track.
#[derive(Debug)]
pub struct Fragment {
    track_chunks: Vec<(TrackId, Vec<Chunk>)>,
}

impl Fragment {
    /// Returns the chunks for the specified track in this fragment.
    pub fn chunks(&self, track_id: TrackId) -> Result<&[Chunk]> {
        self.track_chunks
            .iter()
            .find(|(id, _)| *id == track_id)
            .map(|(_, chunks)| chunks.as_slice())
            .ok_or(ErrorKind::TrackNotFound(track_id).into())
    }
}
