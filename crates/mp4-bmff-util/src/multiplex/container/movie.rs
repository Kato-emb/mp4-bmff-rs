use crate::multiplex::Result;
use crate::multiplex::error::{Error, ErrorKind};

use super::Timescale;
use super::Track;
use super::TrackId;

/// A struct representing a movie in a media file, containing information about the movie's timescale and tracks.
#[derive(Debug, Clone)]
pub struct Movie {
    pub(crate) timescale: Timescale,
    pub(crate) tracks: Vec<Track>,
}

impl Movie {
    /// Creates a new empty movie with the given timescale. Returns an error if `timescale` is zero. Tracks can be appended via [`Movie::add_track`].
    pub fn new(timescale: u32) -> Result<Self> {
        let timescale = Timescale::new(timescale).ok_or_else(|| {
            Error::new(ErrorKind::InvalidInput).with_message("Movie timescale must be non-zero")
        })?;
        Ok(Self {
            timescale,
            tracks: Vec::new(),
        })
    }

    /// Returns the movie timescale (units per second) used by `mvhd`.
    pub fn timescale(&self) -> u32 {
        self.timescale.as_u32()
    }

    /// Appends a track to this movie, returning `&mut self` for chaining.
    pub fn add_track(&mut self, track: Track) -> &mut Self {
        self.tracks.push(track);
        self
    }

    /// Returns the next available track ID for this movie.
    pub fn next_track_id(&self) -> Option<TrackId> {
        let next_id = match self.tracks.iter().map(|t| t.track_id).max() {
            Some(max_id) => max_id.as_u32().checked_add(1),
            None => Some(1),
        };

        next_id.and_then(TrackId::new)
    }

    /// Returns the tracks contained in this movie.
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    /// Returns the tracks contained in this movie as a mutable slice.
    pub fn tracks_mut(&mut self) -> &mut [Track] {
        &mut self.tracks
    }

    /// Returns the duration of the movie in the movie's timescale, if it can be determined. The duration is calculated as the maximum media duration of all tracks, rescaled to the movie's timescale.
    pub(crate) fn movie_duration(&self) -> Option<u64> {
        self.tracks.iter().try_fold(None::<u64>, |acc, track| {
            let media_duration = track
                .timescale
                .rescale_ticks(track.sample_spec.media_duration(), self.timescale)?;
            Some(Some(acc.map_or(media_duration, |a| a.max(media_duration))))
        })?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multiplex::container::SampleDescription;
    use mp4_bmff::types::FourCC;

    fn dummy_track(track_id: u32, timescale: u32) -> Track {
        Track::new(
            track_id,
            timescale,
            vec![SampleDescription::Other(FourCC::new(*b"test"))],
        )
        .unwrap()
    }

    #[test]
    fn new_rejects_zero_timescale() {
        assert_eq!(
            Movie::new(0).unwrap_err().kind(),
            ErrorKind::InvalidInput
        );
    }

    #[test]
    fn new_creates_empty_movie() {
        let movie = Movie::new(1000).unwrap();
        assert_eq!(movie.timescale(), 1000);
        assert!(movie.tracks().is_empty());
    }

    #[test]
    fn next_track_id_starts_at_one_for_empty_movie() {
        let movie = Movie::new(1000).unwrap();
        assert_eq!(movie.next_track_id().unwrap().as_u32(), 1);
    }

    #[test]
    fn next_track_id_returns_max_plus_one() {
        let mut movie = Movie::new(1000).unwrap();
        movie.add_track(dummy_track(1, 48_000));
        movie.add_track(dummy_track(5, 48_000));
        movie.add_track(dummy_track(3, 48_000));
        assert_eq!(movie.next_track_id().unwrap().as_u32(), 6);
    }

    #[test]
    fn next_track_id_overflow_returns_none() {
        let mut movie = Movie::new(1000).unwrap();
        movie.add_track(dummy_track(u32::MAX, 48_000));
        assert!(movie.next_track_id().is_none());
    }

    #[test]
    fn add_track_supports_chaining() {
        let mut movie = Movie::new(1000).unwrap();
        movie
            .add_track(dummy_track(1, 48_000))
            .add_track(dummy_track(2, 48_000));
        assert_eq!(movie.tracks().len(), 2);
    }

    #[test]
    fn movie_duration_takes_max_across_tracks() {
        let mut movie = Movie::new(1000).unwrap();

        let mut t1 = dummy_track(1, 1000);
        t1.set_sample_spec(super::super::SampleSpec::from_samples(
            &[100; 5],
            &[1; 5],
            None,
            None,
            None,
        ));
        let mut t2 = dummy_track(2, 1000);
        t2.set_sample_spec(super::super::SampleSpec::from_samples(
            &[100; 3],
            &[1; 3],
            None,
            None,
            None,
        ));
        movie.add_track(t1);
        movie.add_track(t2);

        // t1 = 500 ticks, t2 = 300 ticks → max = 500
        assert_eq!(movie.movie_duration(), Some(500));
    }

    #[test]
    fn movie_duration_empty_returns_none() {
        let movie = Movie::new(1000).unwrap();
        assert_eq!(movie.movie_duration(), None);
    }
}
