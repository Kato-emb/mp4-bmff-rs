use core::num::NonZeroU32;
use core::time::Duration;

use mp4_bmff::boxes::bmff::*;
use mp4_bmff::types::*;

use super::EditSegment;
use super::MediaDefinition;
use super::Sample;
use super::TrackId;

#[derive(Debug)]
pub(super) struct Chunk {
    first: Sample,
    rest: Vec<Sample>,
}

impl Chunk {
    pub(super) fn new(first: Sample) -> Self {
        Self {
            first,
            rest: Vec::new(),
        }
    }

    pub(super) fn push(&mut self, sample: Sample) {
        self.rest.push(sample);
    }

    pub(super) const fn first(&self) -> &Sample {
        &self.first
    }

    pub(super) fn samples(&self) -> impl Iterator<Item = &Sample> {
        core::iter::once(&self.first).chain(self.rest.iter())
    }

    pub(super) fn sample_count(&self) -> usize {
        1 + self.rest.len()
    }

    pub(super) fn data_offset(&self) -> u64 {
        self.first.data_offset
    }

    pub(super) fn total_size(&self) -> u64 {
        self.samples().map(|s| s.size as u64).sum()
    }

    pub(super) fn end_position(&self) -> u64 {
        self.data_offset() + self.total_size()
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
            .flat_map(|group| group.samples())
            .fold(Duration::ZERO, |acc, entry| acc + entry.duration)
    }

    pub(super) fn first_decode_time_ns(&self) -> Option<u64> {
        self.chunks.first().map(|chunk| chunk.first().dts_ns)
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

    pub(super) fn next_track_id(&self) -> Option<NonZeroU32> {
        match self.tracks.iter().map(|t| t.id.get()).max() {
            Some(max_id) => max_id.checked_add(1).and_then(NonZeroU32::new),
            None => NonZeroU32::new(1),
        }
    }

    pub(super) fn movie_duration(&self) -> Duration {
        self.tracks
            .iter()
            .map(|t| t.edit_duration().unwrap_or(t.sample_table.media_duration()))
            .max()
            .unwrap_or(Duration::ZERO)
    }

    pub(super) fn get_track_mut(&mut self, track_id: TrackId) -> Option<&mut Track> {
        self.tracks
            .iter_mut()
            .find(|t| t.id.get() == track_id.get())
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
