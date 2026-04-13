//! finalize step: apply accumulated tables, set timestamps, compose moov,
//! rewrite mdat header, sync.

use std::collections::BTreeMap;

use mp4_bmff::codec::write_box;
use mp4_bmff::types::QuickTimeDateTime;
use mp4_bmff::{BoxCodec, BoxEncode, BoxHeader, BoxType};

use mp4_bmff_util::multiplex::compose_moov;
use mp4_bmff_util::multiplex::container::{ChunkLayout, Movie, SampleSpec, TrackId};

use crate::platform::error::WasmError;
use crate::platform::opfs::WasmFileSystemSyncAccessHandle;

use super::LARGESIZE_HEADER_LEN;
use super::state::Assembler;

pub(super) fn run(mut state: Assembler, date: Option<js_sys::Date>) -> Result<(), WasmError> {
    apply_accumulated_tables(&mut state.movie, std::mem::take(&mut state.accum))?;
    apply_timestamps(&mut state.movie, date);

    let moov = compose_moov(&state.movie)?;
    let moov_pos = state.handle.position();
    write_complete_box(&mut state.handle, &moov)?;

    rewrite_mdat_header(&mut state.handle, state.free_pos, moov_pos)?;

    state.handle.sync()?;
    Ok(())
}

fn apply_accumulated_tables(
    movie: &mut Movie,
    mut accum: BTreeMap<TrackId, (SampleSpec, ChunkLayout)>,
) -> Result<(), WasmError> {
    for track in movie.tracks_mut() {
        let id = track.track_id();
        let (spec, layout) = accum
            .remove(&id)
            .ok_or_else(|| WasmError::msg(format!("no fragment data for track {}", id.as_u32())))?;
        track.set_sample_spec(spec);
        track.set_chunk_layout(layout)?;
    }
    if !accum.is_empty() {
        let mut extras = String::new();
        for id in accum.keys() {
            if !extras.is_empty() {
                extras.push_str(", ");
            }
            use core::fmt::Write;
            let _ = write!(extras, "{}", id.as_u32());
        }
        return Err(WasmError::msg(format!(
            "fragments referenced unknown track IDs: [{extras}]"
        )));
    }
    Ok(())
}

/// Sets `modification_time` on the movie and every track to `date` (when
/// supplied) and back-fills `creation_time` when the init segment did not
/// carry one. The original creation time decoded from the init segment is
/// preserved when present.
fn apply_timestamps(movie: &mut Movie, date: Option<js_sys::Date>) {
    let Some(date) = date else { return };

    // js_sys::Date::get_time() returns ms since 1970-01-01 UTC. NaN or
    // negative values can't be expressed as a QuickTime timestamp; leave the
    // existing values untouched in that case.
    let millis = date.get_time();
    if !millis.is_finite() {
        return;
    }
    let unix_secs = (millis / 1000.0) as i64;
    let Some(now) = QuickTimeDateTime::from_unix_seconds(unix_secs) else {
        return;
    };

    let zero = QuickTimeDateTime::default();

    if movie.creation_time() == zero {
        movie.set_creation_time(now);
    }
    movie.set_modification_time(now);

    for track in movie.tracks_mut() {
        if track.creation_time() == zero {
            track.set_creation_time(now);
        }
        track.set_modification_time(now);
    }
}

fn rewrite_mdat_header(
    handle: &mut WasmFileSystemSyncAccessHandle,
    free_pos: u64,
    moov_pos: u64,
) -> Result<(), WasmError> {
    let span = moov_pos
        .checked_sub(free_pos)
        .ok_or_else(|| WasmError::msg("free_pos > moov_pos"))?;
    let payload_len = span
        .checked_sub(LARGESIZE_HEADER_LEN as u64)
        .ok_or_else(|| {
            WasmError::msg(format!(
                "mdat span {span} < {LARGESIZE_HEADER_LEN} byte largesize header"
            ))
        })?;

    let header = BoxHeader::new_extended(BoxType::MDAT, payload_len);
    debug_assert_eq!(header.header_len(), LARGESIZE_HEADER_LEN);
    let mut buf = [0u8; LARGESIZE_HEADER_LEN];
    header.write(&mut buf)?;
    handle.write_at(free_pos, &buf)?;
    Ok(())
}

/// Allocates a single buffer sized for the complete encoded box (header +
/// payload), serializes the box, then forwards it to OPFS in one call.
/// Single allocation per `moov` write — for typical movies this is a few
/// hundred KB and well below the OPFS large-write bypass threshold.
fn write_complete_box<B>(
    handle: &mut WasmFileSystemSyncAccessHandle,
    boxed: &B,
) -> Result<(), WasmError>
where
    B: BoxCodec + BoxEncode,
{
    let len = boxed.boxed_len();
    let mut buf = vec![0u8; len];
    write_box(&mut buf, boxed)?;
    handle.write(&buf)?;
    Ok(())
}
