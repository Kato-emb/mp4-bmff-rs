//! Unified error type used by the WASM bindings.
//!
//! Direct `impl From<X> for JsValue` is blocked by the orphan rule for foreign
//! types (`mp4_bmff::Error`, `mp4_bmff_util::multiplex::MuxError`, …), so this
//! module defines a single local [`WasmError`] enum that wraps every error
//! kind we care about. `From` is implemented in both directions:
//!
//! - `From<E> for WasmError` for each library/source error → enables `?` in
//!   binding code that returns `Result<T, WasmError>`.
//! - `From<WasmError> for JsValue` → enables `?` again at the outermost layer
//!   that returns `Result<T, JsValue>` to JavaScript.

use std::fmt;

use wasm_bindgen::prelude::*;

#[cfg(feature = "opfs")]
use super::opfs::WasmFileSystemSyncAccessHandleError;

/// All error kinds surfaced by the WASM bindings.
#[derive(Debug)]
pub enum WasmError {
    /// A parsing / encoding error from `mp4-bmff`.
    Bmff(mp4_bmff::Error),
    /// A multiplex error from `mp4-bmff-util`.
    Multiplex(mp4_bmff_util::multiplex::MuxError),
    /// An OPFS / `FileSystemSyncAccessHandle` I/O error.
    #[cfg(feature = "opfs")]
    Opfs(WasmFileSystemSyncAccessHandleError),
    /// An ad-hoc message (used for invariants the type system can't enforce).
    Message(String),
}

impl WasmError {
    /// Builds a `WasmError::Message` from any `Display` value.
    pub fn msg(m: impl fmt::Display) -> Self {
        Self::Message(m.to_string())
    }
}

impl fmt::Display for WasmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bmff(e) => write!(f, "mp4-bmff: {e}"),
            Self::Multiplex(e) => write!(f, "multiplex: {e}"),
            #[cfg(feature = "opfs")]
            Self::Opfs(e) => write!(f, "opfs: {e}"),
            Self::Message(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for WasmError {}

impl From<mp4_bmff::Error> for WasmError {
    fn from(e: mp4_bmff::Error) -> Self {
        Self::Bmff(e)
    }
}

impl From<mp4_bmff_util::multiplex::MuxError> for WasmError {
    fn from(e: mp4_bmff_util::multiplex::MuxError) -> Self {
        Self::Multiplex(e)
    }
}

#[cfg(feature = "opfs")]
impl From<WasmFileSystemSyncAccessHandleError> for WasmError {
    fn from(e: WasmFileSystemSyncAccessHandleError) -> Self {
        Self::Opfs(e)
    }
}

impl From<&str> for WasmError {
    fn from(s: &str) -> Self {
        Self::Message(s.to_string())
    }
}

impl From<String> for WasmError {
    fn from(s: String) -> Self {
        Self::Message(s)
    }
}

impl From<WasmError> for JsValue {
    fn from(e: WasmError) -> Self {
        // Preserve the underlying DomException when present so JS-side
        // try/catch can still inspect `.name`, `.message`, etc.
        #[cfg(feature = "opfs")]
        if let WasmError::Opfs(opfs) = e {
            return opfs.into_js_value();
        }
        JsValue::from_str(&e.to_string())
    }
}
