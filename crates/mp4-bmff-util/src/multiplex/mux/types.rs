use core::num::NonZeroU32;
use core::time::Duration;

use mp4_bmff::boxes::bmff::SampleFlags;

use super::Error;
use super::ErrorKind;
use super::Result;

use super::Chunk;
use super::Track;
use super::TrackId;

#[derive(Debug)]
pub(super) struct Movie {
    pub timescale: NonZeroU32,
    pub track_entries: Vec<TrackEntry>,
}

impl Movie {
    pub(super) fn new(timescale: NonZeroU32) -> Self {
        Self {
            timescale,
            track_entries: Vec::new(),
        }
    }

    pub(super) fn next_track_id(&self) -> Result<TrackId> {
        let next_id = match self.track_entries.iter().map(|s| s.info.id).max() {
            Some(max_id) => max_id.get().checked_add(1),
            None => Some(1),
        };

        next_id.and_then(TrackId::new).ok_or(
            Error::new(ErrorKind::Overflow)
                .with_message("Exceeded maximum track ID limit of 2^32 - 1"),
        )
    }

    pub(super) fn get_track_entry_mut(&mut self, track_id: TrackId) -> Option<&mut TrackEntry> {
        self.track_entries
            .iter_mut()
            .find(|s| s.info.id == track_id)
    }
}

#[derive(Debug)]
pub(super) struct TrackEntry {
    pub info: Track,
    pub defaults: TrackDefaults,
    pub sample_table: SampleTable,
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
            .map(|chunk| chunk.first_sample().dts_ns())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multiplex::MediaDefinition;
    use mp4_bmff::types::FourCC;

    fn dummy_media() -> MediaDefinition {
        MediaDefinition::Other(FourCC::new(*b"test"))
    }

    fn nz(v: u32) -> NonZeroU32 {
        NonZeroU32::new(v).unwrap()
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
        let track = Track::new(TrackId::new(1).unwrap(), nz(1000), dummy_media());
        movie.track_entries.push(TrackEntry {
            info: track,
            defaults: TrackDefaults::default(),
            sample_table: SampleTable::default(),
        });
        assert_eq!(movie.next_track_id().unwrap().get(), 2);

        let track = Track::new(TrackId::new(2).unwrap(), nz(1000), dummy_media());
        movie.track_entries.push(TrackEntry {
            info: track,
            defaults: TrackDefaults::default(),
            sample_table: SampleTable::default(),
        });
        assert_eq!(movie.next_track_id().unwrap().get(), 3);
    }
}
