//! WASM bindings for `mp4-bmff` use-cases.
//!
//! This is the umbrella WASM crate. Each use-case is a feature-gated module
//! that re-exports its `#[wasm_bindgen]` API at the crate root. JavaScript
//! sees a single flat namespace per built artifact:
//!
//! ```js
//! import {
//!   // fmp4-asm namespace
//!   initialize,
//!   process_init_segment_from_u8_array,
//!   process_media_segment_from_u8_array,
//!   finalize,
//! } from "@kato-emb/mp4-bmff-wasm";
//! ```
//!
//! Adding a new use-case is a matter of adding a `[features]` flag in
//! `Cargo.toml`, a `#[cfg(feature = "...")] pub mod ...;` here, and a
//! corresponding `pub use` line below.
//!
//! The crate intentionally pulls in `std` (despite the rest of the workspace
//! being `no_std + alloc`): `wasm-bindgen` and `std::thread_local!` are
//! convenient here, and the `wasm32-unknown-unknown` target ships `std`
//! regardless. The reusable parsing / muxing logic itself lives in
//! `mp4-bmff-util`, which remains `no_std + alloc`.

// Platform layer: shared infra (error wrappers, OPFS sink). Use-case modules
// only depend on this layer plus the pure-Rust `mp4-bmff*` crates.
pub mod platform;

// Use-case modules. Each is feature-gated and re-exports its
// `#[wasm_bindgen]` API at the crate root for a flat JS namespace.
#[cfg(feature = "fmp4-asm")]
pub mod fmp4_asm;

#[cfg(feature = "fmp4-asm")]
pub use fmp4_asm::{
    finalize, initialize, process_init_segment_from_u8_array, process_media_segment_from_u8_array,
};
