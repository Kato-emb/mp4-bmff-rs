//! Init segment processing: ftyp pass-through, moov decompose, free
//! placeholder reservation.

use mp4_bmff::boxes::bmff::{MoovBoxView, TrexBox};
use mp4_bmff::{BoxDecode, BoxType, iter_boxes};

use mp4_bmff_util::multiplex::container::Movie;
use mp4_bmff_util::multiplex::decompose_moov;

use crate::platform::error::WasmError;

use super::state::Assembler;
use super::write_largesize_placeholder;

pub(super) fn run(state: &mut Assembler, data: &[u8]) -> Result<(), WasmError> {
    let (movie, trexs, ftyp_bytes) = parse(data)?;

    // ftyp is copied byte-for-byte to preserve compatibility brand list.
    state.handle.write(ftyp_bytes)?;

    // Reserve a 16-byte largesize placeholder; finalize will rewrite this
    // slot as an `mdat` largesize header that swallows every fragment byte.
    state.free_pos = state.handle.position();
    write_largesize_placeholder(&mut state.handle, BoxType::FREE)?;

    state.movie = movie;
    state.trexs = trexs;
    state.accum.clear();
    Ok(())
}

/// Decodes the init segment into `(Movie, trexs, ftyp_bytes)` without
/// allocating beyond the necessary `Vec<TrexBox>` and the `Movie` itself.
fn parse(bytes: &[u8]) -> Result<(Movie, Vec<TrexBox>, &[u8]), WasmError> {
    let mut ftyp_slice: Option<&[u8]> = None;
    let mut moov_payload: Option<&[u8]> = None;

    let mut cursor = 0usize;
    for raw in iter_boxes(bytes) {
        let raw = raw?;
        let len = raw.len();
        let total = &bytes[cursor..cursor + len];
        match raw.boxtype() {
            BoxType::FTYP => ftyp_slice = Some(total),
            BoxType::MOOV => moov_payload = Some(raw.into_payload()),
            _ => {}
        }
        cursor += len;
    }

    let ftyp = ftyp_slice.ok_or_else(|| WasmError::msg("init: ftyp missing"))?;
    let moov_payload = moov_payload.ok_or_else(|| WasmError::msg("init: moov missing"))?;

    let moov_view = MoovBoxView::decode(moov_payload)?;

    let mut trexs = Vec::new();
    if let Some(mvex) = moov_view.mvex()? {
        for trex in mvex.trexs() {
            trexs.push(trex?);
        }
    }

    let movie = decompose_moov(&moov_view)?;
    Ok((movie, trexs, ftyp))
}
