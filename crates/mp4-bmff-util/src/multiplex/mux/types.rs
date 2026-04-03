use core::num::NonZeroU32;
use core::time::Duration;

use mp4_bmff::boxes::bmff::SampleFlags;
use mp4_bmff::types::*;

use super::Error;
use super::ErrorKind;
use super::Result;

use super::EditSegment;
use super::MediaDefinition;
use super::Sample;
use super::TrackId;

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

    pub(super) fn next_track_id(&self) -> Result<TrackId> {
        let next_id = match self.tracks.iter().map(|t| t.id).max() {
            Some(max_id) => max_id.get().checked_add(1),
            None => Some(1),
        };

        next_id.and_then(TrackId::new).ok_or(
            Error::new(ErrorKind::Overflow)
                .with_message("Exceeded maximum track ID limit of 2^32 - 1"),
        )
    }

    pub(super) fn begin_chunk(&mut self, track_id: TrackId, data_offset: u64) -> Result<()> {
        let track = self
            .get_track_mut(track_id)
            .ok_or(ErrorKind::TrackNotFound(track_id))?;

        track.sample_table.begin_chunk(data_offset)?;
        Ok(())
    }

    pub(super) fn push_sample(&mut self, track_id: TrackId, sample: Sample) -> Result<()> {
        let track = self
            .get_track_mut(track_id)
            .ok_or(ErrorKind::TrackNotFound(track_id))?;

        track.sample_table.push_sample(sample)?;
        Ok(())
    }

    fn get_track_mut(&mut self, track_id: TrackId) -> Option<&mut Track> {
        self.tracks.iter_mut().find(|t| t.id == track_id)
    }
}

#[derive(Debug)]
pub(super) struct Track {
    pub id: TrackId,
    pub timescale: NonZeroU32,
    pub language: LanguageCode,
    pub matrix: Matrix,
    pub alternate_group: i16,
    pub edit_list: Option<Vec<EditSegment>>,
    pub media: MediaDefinition,
    pub defaults: TrackDefaults,
    pub sample_table: SampleTable,
}

impl Track {
    pub(super) fn new(id: TrackId, timescale: NonZeroU32, media: MediaDefinition) -> Self {
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

#[derive(Debug, Default)]
pub(super) struct TrackDefaults {
    pub sample_duration: Option<Duration>,
    pub sample_size: Option<u32>,
    pub sample_flags: Option<SampleFlags>,
}

#[derive(Debug, Default)]
pub(super) struct SampleTable {
    pub chunks: Vec<Chunk>,
}

impl SampleTable {
    pub(super) fn first_sample_dts(&self) -> Option<u64> {
        self.chunks
            .first()
            .and_then(|chunk| chunk.samples.first().map(|s| s.dts_ns))
    }

    fn begin_chunk(&mut self, data_offset: u64) -> Result<()> {
        if self.chunks.last().is_some_and(|c| c.samples.is_empty()) {
            return Err(Error::new(ErrorKind::InvalidInput).with_message(
                "Cannot begin a new chunk before adding samples to the previous chunk",
            ));
        }

        self.chunks.push(Chunk {
            data_offset,
            samples: Vec::new(),
        });
        Ok(())
    }

    fn push_sample(&mut self, sample: Sample) -> Result<()> {
        let Some(last_chunk) = self.chunks.last_mut() else {
            return Err(Error::new(ErrorKind::InvalidInput)
                .with_message("Before adding sample, must add chunks to the sample table"));
        };
        // Verify DTS is monotonically non-decreasing.
        if last_chunk
            .samples
            .last()
            .map_or(false, |last_sample| sample.dts_ns < last_sample.dts_ns)
        {
            return Err(Error::new(ErrorKind::InvalidInput)
                .with_message("Samples must be added in non-decreasing DTS order"));
        }

        last_chunk.samples.push(sample);
        Ok(())
    }
}

#[derive(Debug)]
pub(super) struct Chunk {
    pub data_offset: u64,
    pub samples: Vec<Sample>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(dts_ns: u64, size: u32) -> Sample {
        Sample::new(dts_ns, None, Duration::from_millis(33), true, size)
    }

    fn dummy_media() -> MediaDefinition {
        MediaDefinition::Other(FourCC::new(*b"test"))
    }

    fn nz(v: u32) -> NonZeroU32 {
        NonZeroU32::new(v).unwrap()
    }

    // --- SampleTable ---

    #[test]
    fn sample_table_push_creates_chunks() {
        let mut table = SampleTable::default();
        // Contiguous samples → same chunk
        table.push_sample(sample(0, 100)).unwrap();
        table.push_sample(sample(1, 100)).unwrap();
        assert_eq!(table.chunks.len(), 1);

        // Non-contiguous → new chunk
        table.push_sample(sample(2, 100)).unwrap();
        assert_eq!(table.chunks.len(), 2);
    }

    #[test]
    fn sample_table_push_rejects_decreasing_dts() {
        let mut table = SampleTable::default();
        table.push_sample(sample(100, 100)).unwrap();
        assert!(table.push_sample(sample(50, 100)).is_err());
    }

    #[test]
    fn sample_table_first_sample_dts() {
        let mut table = SampleTable::default();
        assert!(table.first_sample_dts().is_none());

        table.push_sample(sample(42_000, 100)).unwrap();
        assert_eq!(table.first_sample_dts(), Some(42_000));
    }

    // --- Track ---

    #[test]
    fn track_edit_duration() {
        let mut track = Track::new(TrackId::new(1).unwrap(), nz(1000), dummy_media());
        assert!(track.edit_duration().is_none());

        track.edit_list = Some(vec![
            EditSegment::Empty {
                duration: Duration::from_secs(1),
            },
            EditSegment::Media {
                duration: Duration::from_secs(3),
                media_start: Duration::ZERO,
                media_rate: 1.0,
            },
        ]);
        assert_eq!(track.edit_duration(), Some(Duration::from_secs(4)));
    }

    // --- Movie ---

    #[test]
    fn movie_next_track_id_starts_at_one() {
        let movie = Movie::new(nz(1000));
        assert_eq!(movie.next_track_id().unwrap().get(), 1);
    }

    #[test]
    fn movie_next_track_id_increments() {
        let mut movie = Movie::new(nz(1000));
        movie.tracks.push(Track::new(
            TrackId::new(1).unwrap(),
            nz(1000),
            dummy_media(),
        ));
        assert_eq!(movie.next_track_id().unwrap().get(), 2);

        movie.tracks.push(Track::new(
            TrackId::new(2).unwrap(),
            nz(1000),
            dummy_media(),
        ));
        assert_eq!(movie.next_track_id().unwrap().get(), 3);
    }

    #[test]
    fn movie_add_sample_to_existing_track() {
        let mut movie = Movie::new(nz(1000));
        let id = TrackId::new(1).unwrap();
        movie.tracks.push(Track::new(id, nz(1000), dummy_media()));

        let result = movie.push_sample(id, sample(0, 100));
        assert!(result.is_ok());
        assert_eq!(movie.tracks[0].sample_table.chunks.len(), 1);
    }

    #[test]
    fn movie_add_sample_to_nonexistent_track() {
        let mut movie = Movie::new(nz(1000));
        movie.tracks.push(Track::new(
            TrackId::new(1).unwrap(),
            nz(1000),
            dummy_media(),
        ));

        let result = movie.push_sample(TrackId::new(99).unwrap(), sample(0, 100));
        assert!(result.is_err());
    }
}
