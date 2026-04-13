use super::Timescale;
use super::Track;

#[derive(Debug, Clone)]
pub struct Movie {
    pub(crate) timescale: Timescale,
    pub(crate) tracks: Vec<Track>,
}

impl Movie {
    /// Returns the tracks contained in this movie.
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }
}
