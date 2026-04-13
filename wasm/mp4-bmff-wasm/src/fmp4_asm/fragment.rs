//! Per-fragment streaming: walks one fragment's top-level boxes, decomposes
//! every `moof` into per-track table increments, and forwards the bytes to
//! OPFS in a single large write.

use mp4_bmff::boxes::bmff::MoofBoxView;
use mp4_bmff::{BoxDecode, BoxType, iter_boxes};

use mp4_bmff_util::multiplex::container::{ChunkLayout, SampleSpec};
use mp4_bmff_util::multiplex::decompose_moof;

use crate::platform::error::WasmError;

use super::state::Assembler;

pub(super) fn run(state: &mut Assembler, data: &[u8]) -> Result<(), WasmError> {
    let written_at = state.handle.position();

    // Walk top-level boxes once to find moofs and decompose them. The bytes
    // themselves are forwarded to OPFS in a single call so the OPFS sink's
    // large-write bypass kicks in for the bulky mdat region.
    let mut cursor: u64 = 0;
    for raw in iter_boxes(data) {
        let raw = raw?;
        let len = raw.len() as u64;

        if raw.boxtype() == BoxType::MOOF {
            let moof_pos_in_output = written_at + cursor;
            let moof = MoofBoxView::decode(raw.payload())?;
            let entries = decompose_moof(&moof, moof_pos_in_output, &state.trexs)?;
            for (id, spec, layout) in entries {
                let entry = state
                    .accum
                    .entry(id)
                    .or_insert_with(|| (SampleSpec::default(), ChunkLayout::default()));
                entry.0 = entry.0.concat(&spec);
                entry.1 = entry.1.concat(&layout);
            }
        }

        cursor += len;
    }

    state.handle.write(data)?;
    Ok(())
}
