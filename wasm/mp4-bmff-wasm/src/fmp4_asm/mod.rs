//! fMP4 → non-fragmented MP4 assembler bindings (OPFS-backed).
//!
//! The JavaScript caller drives the assembler through four `#[wasm_bindgen]`
//! functions exposed at the crate root:
//!
//! 1. [`initialize`] — bind a `FileSystemSyncAccessHandle` (the OPFS sink).
//! 2. [`process_init_segment_from_u8_array`] — feed the initialization
//!    segment bytes once.
//! 3. [`process_media_segment_from_u8_array`] — feed each media fragment in
//!    order; called repeatedly. Each call holds at most one fragment in WASM
//!    memory.
//! 4. [`finalize`] — emit `moov`, rewrite the placeholder mdat header, and
//!    flush. After this the OPFS file is a complete non-fragmented MP4.
//!
//! Module layout:
//!
//! - [`state`] — per-thread `Assembler` struct + `thread_local!` cell.
//! - [`init`]  — init-segment parsing (`parse_init`).
//! - [`fragment`] — per-fragment streaming + table accumulation.
//! - [`finalize_impl`] — moov composition, mdat header rewrite, timestamps.
//!
//! Memory footprint is bounded by the largest single fragment passed from JS
//! plus the per-track accumulation tables; OPFS is written through
//! [`crate::platform::opfs::WasmFileSystemSyncAccessHandle`], which buffers
//! small writes and bypasses the buffer for large ones.

use wasm_bindgen::prelude::*;

use crate::platform::error::WasmError;
use crate::platform::opfs::WasmFileSystemSyncAccessHandle;

mod finalize_impl;
mod fragment;
mod init;
mod state;

use state::{Assembler, STATE};

/// Initializes the assembler with a `FileSystemSyncAccessHandle` obtained on
/// the JS side. Any prior state is replaced.
#[wasm_bindgen]
pub fn initialize(handle: web_sys::FileSystemSyncAccessHandle) -> Result<(), JsValue> {
    let handle = WasmFileSystemSyncAccessHandle::new(handle);
    STATE.with(|s| {
        *s.borrow_mut() = Some(Assembler::new(handle));
    });
    Ok(())
}

/// Parses the init segment, copies its `ftyp` to the OPFS file, decomposes
/// `moov` into the in-memory `Movie` template, and reserves a 16-byte
/// largesize `free` placeholder that will later be rewritten as the mdat
/// header.
#[wasm_bindgen]
pub fn process_init_segment_from_u8_array(data: &[u8]) -> Result<(), JsValue> {
    with_state(|state| init::run(state, data))
}

/// Streams a single media fragment to OPFS and accumulates per-track tables.
#[wasm_bindgen]
pub fn process_media_segment_from_u8_array(data: &[u8]) -> Result<(), JsValue> {
    with_state(|state| fragment::run(state, data))
}

/// Composes the final non-fragmented `moov`, writes it to OPFS, rewrites the
/// reserved placeholder as a `mdat` largesize header that swallows every
/// fragment byte, and syncs the underlying file. After this call the OPFS
/// file is a complete MP4 and the assembler state is dropped.
///
/// `date` is optional. When supplied it is used as the modification time of
/// the resulting movie and tracks; the original creation time decoded from
/// the init segment is preserved. When `date` is `None` and no creation time
/// was decoded, both timestamps fall back to the QuickTime epoch.
#[wasm_bindgen]
pub fn finalize(date: Option<js_sys::Date>) -> Result<(), JsValue> {
    let state = STATE.with(|s| s.borrow_mut().take());
    let state = state.ok_or_else(|| WasmError::msg("assembler not initialized"))?;
    finalize_impl::run(state, date).map_err(JsValue::from)
}

// =============================================================================
// Internal: state borrow helper
// =============================================================================

fn with_state<F>(f: F) -> Result<(), JsValue>
where
    F: FnOnce(&mut Assembler) -> Result<(), WasmError>,
{
    STATE.with(|s| {
        let mut borrow = s.borrow_mut();
        let state = borrow
            .as_mut()
            .ok_or_else(|| WasmError::msg("assembler not initialized"))?;
        f(state)
    })?;
    Ok(())
}

// =============================================================================
// Internal: shared helpers used by multiple sub-modules
// =============================================================================

use mp4_bmff::{BoxHeader, BoxType};

/// Size of an extended (`largesize`) BMFF box header (4-byte size marker +
/// 4-byte type + 8-byte 64-bit size field).
pub(crate) const LARGESIZE_HEADER_LEN: usize = 16;

/// Writes a fixed-size 16-byte largesize box header (no payload).
pub(crate) fn write_largesize_placeholder(
    handle: &mut WasmFileSystemSyncAccessHandle,
    box_type: BoxType,
) -> Result<(), WasmError> {
    let header = BoxHeader::new_extended(box_type, 0);
    debug_assert_eq!(header.header_len(), LARGESIZE_HEADER_LEN);
    let mut buf = [0u8; LARGESIZE_HEADER_LEN];
    header.write(&mut buf)?;
    handle.write(&buf)?;
    Ok(())
}
