use core::num::NonZeroU32;
use core::time::Duration;

use mp4_bmff::boxes::bmff::*;
use mp4_bmff::types::*;

use super::EditSegment;
use super::MediaDefinition;
use super::Sample;

use super::Result;
use super::error::*;

#[derive(Debug)]
pub(super) struct Chunk {
    pub data_offset: u64,
    pub samples: Vec<Sample>,
}

impl Chunk {
    pub(super) fn total_size(&self) -> u64 {
        self.samples.iter().map(|e| u64::from(e.size)).sum()
    }
}

#[derive(Debug, Default)]
pub(super) struct SampleTable {
    pub chunks: Vec<Chunk>,
}

impl SampleTable {
    pub(super) fn media_duration(&self) -> Duration {
        self.chunks
            .iter()
            .flat_map(|group| group.samples.iter())
            .fold(Duration::ZERO, |acc, entry| acc + entry.duration)
    }

    pub(super) fn first_decode_time_ns(&self) -> Option<u64> {
        self.chunks
            .first()
            .and_then(|chunk| chunk.samples.first())
            .map(|s| s.dts_ns)
    }
}

#[derive(Debug, Default)]
pub(super) struct TrackDefaults {
    pub sample_duration: Option<u32>,
    pub sample_size: Option<u32>,
    pub sample_flags: Option<SampleFlags>,
}

/// Represents a track in an MP4 file, containing metadata about the track.
#[derive(Debug)]
pub(super) struct Track {
    pub id: NonZeroU32,
    pub timescale: NonZeroU32,
    pub language: LanguageCode,
    pub matrix: Matrix,
    pub alternate_group: i16,
    pub edit_list: Option<Vec<EditSegment>>,
    pub media: MediaDefinition,
    pub sample_table: SampleTable,
    pub defaults: TrackDefaults,
}

impl Track {
    pub(super) fn new(id: NonZeroU32, timescale: NonZeroU32, media: MediaDefinition) -> Self {
        Self {
            id,
            timescale,
            language: LanguageCode::UNDETERMINED,
            matrix: Matrix::identity(),
            alternate_group: 0,
            edit_list: None,
            media,
            sample_table: SampleTable::default(),
            defaults: TrackDefaults::default(),
        }
    }

    pub(super) fn edit_duration(&self) -> Option<Duration> {
        self.edit_list
            .as_ref()
            .map(|segments| segments.iter().map(|s| s.duration()).sum())
    }
}

#[derive(Debug)]
pub(super) struct Movie {
    pub timescale: NonZeroU32,
    pub tracks: Vec<Track>,
}

impl Movie {
    pub(super) fn new(timescale: NonZeroU32) -> Self {
        Self {
            timescale,
            tracks: Vec::new(),
        }
    }

    pub(super) fn push_chunk(&mut self, track_id: u32, chunk: Chunk) -> Result<()> {
        let track = self.get_track_mut(track_id).ok_or_else(|| {
            Error::new(ErrorKind::InvalidInput)
                .with_message(format!("Track ID {} not found", track_id))
        })?;

        track.sample_table.chunks.push(chunk);
        Ok(())
    }

    pub(super) fn next_track_id(&self) -> Option<u32> {
        match self.tracks.iter().map(|t| t.id.get()).max() {
            Some(max_id) => max_id.checked_add(1),
            None => Some(1),
        }
    }

    pub(super) fn movie_duration(&self) -> Duration {
        self.tracks
            .iter()
            .map(|t| t.edit_duration().unwrap_or(t.sample_table.media_duration()))
            .max()
            .unwrap_or(Duration::ZERO)
    }

    fn get_track_mut(&mut self, track_id: u32) -> Option<&mut Track> {
        self.tracks.iter_mut().find(|t| t.id.get() == track_id)
    }
}

pub(super) fn nanos_to_ticks(nanos: u64, timescale: u32) -> Option<u64> {
    let timescale = u64::from(timescale);
    let secs = nanos / 1_000_000_000;
    let sub_nanos = nanos % 1_000_000_000;

    // nanos * ts は最大 ≈ 4.3 × 10¹⁸ で、u64::MAX より小さいためオーバーフローしない。
    secs.checked_mul(timescale)?
        .checked_add(sub_nanos * timescale / 1_000_000_000)
}

pub(super) fn duration_to_ticks(duration: Duration, timescale: u32) -> Option<u64> {
    let secs = duration.as_secs();
    let sub_nanos = u64::from(duration.subsec_nanos());

    let nanos = secs.checked_mul(1_000_000_000)?.checked_add(sub_nanos)?;
    nanos_to_ticks(nanos, timescale)
}
