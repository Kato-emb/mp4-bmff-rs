//! Per-thread assembler state.

use std::cell::RefCell;
use std::collections::BTreeMap;

use mp4_bmff::boxes::bmff::TrexBox;
use mp4_bmff_util::multiplex::container::{ChunkLayout, Movie, SampleSpec, TrackId};

use crate::platform::opfs::WasmFileSystemSyncAccessHandle;

// WASM is single-threaded, so a `RefCell` is safe.
thread_local! {
    pub(super) static STATE: RefCell<Option<Assembler>> = const { RefCell::new(None) };
}

/// In-progress assembly state: the OPFS sink, init-derived movie template,
/// trex defaults, accumulated per-track tables, and the seek-back position
/// of the placeholder reserved for the final mdat header.
pub(super) struct Assembler {
    pub(super) handle: WasmFileSystemSyncAccessHandle,
    pub(super) movie: Movie,
    pub(super) trexs: Vec<TrexBox>,
    pub(super) accum: BTreeMap<TrackId, (SampleSpec, ChunkLayout)>,
    pub(super) free_pos: u64,
}

impl Assembler {
    pub(super) fn new(handle: WasmFileSystemSyncAccessHandle) -> Self {
        Self {
            handle,
            // Replaced by `init::run`; the timescale value here never reaches
            // the output.
            movie: Movie::new(1).expect("non-zero timescale"),
            trexs: Vec::new(),
            accum: BTreeMap::new(),
            free_pos: 0,
        }
    }
}
