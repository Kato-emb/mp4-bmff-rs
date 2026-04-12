use mp4_bmff::boxes::bmff::MoovBoxView;

use super::Timescale;
use super::Track;

use crate::multiplex::Result;
use crate::multiplex::decompose;

#[derive(Debug, Clone)]
pub struct Movie {
    pub(crate) timescale: Timescale,
    pub(crate) tracks: Vec<Track>,
}

impl Movie {
    pub fn from_moov(moov: &MoovBoxView<'_>) -> Result<Self> {
        let movie = decompose::parse_movie(moov)?;
        Ok(movie)
    }
}
