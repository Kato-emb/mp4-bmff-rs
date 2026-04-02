use core::num::NonZeroU32;
use core::time::Duration;

use mp4_bmff::boxes::bmff::SampleFlags;
use mp4_bmff::types::*;

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

    pub(super) fn next_track_id(&self) -> TrackId {
        let next_id = match self.tracks.iter().map(|t| t.id).max() {
            Some(max_id) => max_id.get().checked_add(1),
            None => Some(1),
        };

        next_id
            .and_then(TrackId::new)
            .expect("Exceeded maximum track ID limit of 2^32 - 1")
    }

    pub(super) fn add_sample(&mut self, track_id: TrackId, sample: Sample) -> Result<()> {
        let track = self
            .get_track_mut(track_id)
            .ok_or(ErrorKind::TrackNotFound(track_id))?;

        track.sample_table.push_sample(sample);
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
        self.chunks.first().map(|chunk| chunk.first().dts_ns)
    }

    fn push_sample(&mut self, sample: Sample) {
        if !self
            .chunks
            .last_mut()
            .is_some_and(|last| last.try_push(sample))
        {
            self.chunks.push(Chunk::new(sample));
        }
    }
}

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

    pub(super) fn try_push(&mut self, sample: Sample) -> bool {
        if self.end_position() == sample.data_offset {
            self.rest.push(sample);
            true
        } else {
            false
        }
    }

    pub(super) const fn first(&self) -> &Sample {
        &self.first
    }

    pub(super) fn last(&self) -> &Sample {
        self.rest.last().unwrap_or(&self.first)
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

    pub(super) fn end_position(&self) -> u64 {
        let last = self.last();
        last.data_offset + u64::from(last.size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(dts_ns: u64, size: u32, data_offset: u64) -> Sample {
        Sample::new(
            dts_ns,
            None,
            Duration::from_millis(33),
            true,
            size,
            data_offset,
        )
    }

    fn dummy_media() -> MediaDefinition {
        MediaDefinition::Other(FourCC::new(*b"test"))
    }

    fn nz(v: u32) -> NonZeroU32 {
        NonZeroU32::new(v).unwrap()
    }

    // --- Chunk ---

    #[test]
    fn chunk_new_has_one_sample() {
        let chunk = Chunk::new(sample(0, 100, 0));
        assert_eq!(chunk.sample_count(), 1);
        assert_eq!(chunk.data_offset(), 0);
    }

    #[test]
    fn chunk_try_push_contiguous() {
        let mut chunk = Chunk::new(sample(0, 100, 0));
        let s2 = sample(33_000_000, 200, 100);
        assert!(chunk.try_push(s2));
        assert_eq!(chunk.sample_count(), 2);
        assert_eq!(chunk.end_position(), 300);
    }

    #[test]
    fn chunk_try_push_non_contiguous() {
        let mut chunk = Chunk::new(sample(0, 100, 0));
        let s2 = sample(33_000_000, 200, 500);
        assert!(!chunk.try_push(s2));
        assert_eq!(chunk.sample_count(), 1);
    }

    #[test]
    fn chunk_first_and_last_single() {
        let s = sample(42, 100, 0);
        let chunk = Chunk::new(s);
        assert_eq!(chunk.first().dts_ns, 42);
        assert_eq!(chunk.last().dts_ns, 42);
    }

    #[test]
    fn chunk_first_and_last_multiple() {
        let mut chunk = Chunk::new(sample(0, 100, 0));
        chunk.try_push(sample(33_000_000, 100, 100));
        assert_eq!(chunk.first().dts_ns, 0);
        assert_eq!(chunk.last().dts_ns, 33_000_000);
    }

    #[test]
    fn chunk_samples_iterator() {
        let mut chunk = Chunk::new(sample(0, 50, 0));
        chunk.try_push(sample(1, 60, 50));
        chunk.try_push(sample(2, 70, 110));
        let sizes: Vec<u32> = chunk.samples().map(|s| s.size).collect();
        assert_eq!(sizes, vec![50, 60, 70]);
    }

    // --- SampleTable ---

    #[test]
    fn sample_table_push_creates_chunks() {
        let mut table = SampleTable::default();
        // Contiguous samples → same chunk
        table.push_sample(sample(0, 100, 0));
        table.push_sample(sample(1, 100, 100));
        assert_eq!(table.chunks.len(), 1);

        // Non-contiguous → new chunk
        table.push_sample(sample(2, 100, 500));
        assert_eq!(table.chunks.len(), 2);
    }

    #[test]
    fn sample_table_first_sample_dts() {
        let mut table = SampleTable::default();
        assert!(table.first_sample_dts().is_none());

        table.push_sample(sample(42_000, 100, 0));
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
        assert_eq!(movie.next_track_id().get(), 1);
    }

    #[test]
    fn movie_next_track_id_increments() {
        let mut movie = Movie::new(nz(1000));
        movie.tracks.push(Track::new(
            TrackId::new(1).unwrap(),
            nz(1000),
            dummy_media(),
        ));
        assert_eq!(movie.next_track_id().get(), 2);

        movie.tracks.push(Track::new(
            TrackId::new(2).unwrap(),
            nz(1000),
            dummy_media(),
        ));
        assert_eq!(movie.next_track_id().get(), 3);
    }

    #[test]
    fn movie_add_sample_to_existing_track() {
        let mut movie = Movie::new(nz(1000));
        let id = TrackId::new(1).unwrap();
        movie.tracks.push(Track::new(id, nz(1000), dummy_media()));

        let result = movie.add_sample(id, sample(0, 100, 0));
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

        let result = movie.add_sample(TrackId::new(99).unwrap(), sample(0, 100, 0));
        assert!(result.is_err());
    }
}
