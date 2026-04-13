//! Platform-layer shared infrastructure.
//!
//! Modules here implement the JS / WASM platform glue (error conversions,
//! OPFS sink) that every use-case binding builds on top of. Use-case modules
//! (`fmp4_asm`, future `demuxer`, …) only depend on this layer and on the
//! pure-Rust `mp4-bmff*` crates — never on each other.

pub mod error;

#[cfg(feature = "opfs")]
pub mod opfs;
