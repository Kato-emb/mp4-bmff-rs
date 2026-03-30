use core::num::NonZeroU32;
use core::time::Duration;

use mp4_bmff::boxes::bmff::*;
use mp4_bmff::types::*;

use super::EditSegment;
use super::MediaDefinition;
use super::Sample;

use super::Result;
use super::error::*;

/// Metadata for a sample, used for building sample tables entries.
#[derive(Debug, Clone, Copy)]
pub(super) struct SampleMetadata {
    pub dts_ns: u64,
    pub pts_ns: Option<i64>,
    pub duration: Duration,
    pub is_sync: bool,
    pub size: u32,
}

impl SampleMetadata {
    pub(super) fn from_sample<T: AsRef<[u8]>>(sample: &Sample<T>) -> Result<Self> {
        let size = u32::try_from(sample.data.as_ref().len())?;

        Ok(SampleMetadata {
            dts_ns: sample.dts_ns,
            pts_ns: sample.pts_ns,
            duration: sample.duration,
            is_sync: sample.is_sync,
            size,
        })
    }

    pub(super) fn sample_delta(&self, timescale: NonZeroU32) -> Result<u32> {
        let Some(ticks) = duration_to_ticks(self.duration, timescale.get()) else {
            return Err(ErrorKind::Overflow.into());
        };

        u32::try_from(ticks).map_err(Into::into)
    }

    pub(super) fn sample_flags(&self) -> SampleFlags {
        if self.is_sync {
            SampleFlags::sync()
        } else {
            SampleFlags::non_sync()
        }
    }

    pub(super) fn composition_time_offset(&self, media_timescale: NonZeroU32) -> Result<i32> {
        let Some(pts) = self.pts_ns else {
            return Ok(0);
        };

        let diff_ns = pts.checked_sub(self.dts_ns as i64);
        let ticks = diff_ns
            .and_then(|d| d.checked_mul(i64::from(media_timescale.get())))
            .and_then(|ticks| ticks.checked_div(1_000_000_000))
            .ok_or(ErrorKind::Overflow)?;

        i32::try_from(ticks).map_err(Into::into)
    }
}

impl<T: AsRef<[u8]>> TryFrom<&Sample<T>> for SampleMetadata {
    type Error = Error;

    fn try_from(sample: &Sample<T>) -> Result<Self> {
        Self::from_sample(sample)
    }
}

#[derive(Debug)]
pub(super) struct SampleChunk {
    pub data_offset: u64,
    pub entries: Vec<SampleMetadata>,
}

impl SampleChunk {
    pub(super) fn total_size(&self) -> u64 {
        self.entries.iter().map(|e| u64::from(e.size)).sum()
    }
}

#[derive(Debug, Default)]
pub(super) struct SampleTable {
    pub chunks: Vec<SampleChunk>,
}

impl SampleTable {
    pub(super) fn media_duration(&self) -> Duration {
        self.chunks
            .iter()
            .flat_map(|group| group.entries.iter())
            .fold(Duration::ZERO, |acc, entry| acc + entry.duration)
    }

    pub(super) fn first_decode_time_ns(&self) -> Option<u64> {
        self.chunks
            .first()
            .and_then(|chunk| chunk.entries.first())
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

    pub(super) fn push_chunk(&mut self, track_id: u32, chunk: SampleChunk) -> Result<()> {
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
