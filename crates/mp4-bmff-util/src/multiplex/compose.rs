use mp4_bmff::boxes::bmff::*;

use super::Result;

use super::Context;
use super::Layout;

use super::repr::*;

pub fn build_moov(context: &Context, layout: &Layout) -> Result<MoovBox> {
    todo!()
}

fn build_mvhd(movie: &Movie) -> Result<MvhdBox> {
    let movie_timescale = movie.timescale;
    let duration = movie.tracks.iter().filter_map(|tr| {}).max().unwrap_or(0);
}

fn edit_duration(track: &Track) -> Option<u64> {}
