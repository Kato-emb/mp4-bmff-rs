//! MP4/BMFF file inspection utilities.
//!
//! This crate provides analysis tools built on top of [`mp4_bmff`]:
//!
//! - [`tree`]: Box structure tree — parses a byte slice into a hierarchical
//!   [`BoxNode`](tree::BoxNode) tree and formats it for display.
//! - [`stats`]: Sample table statistics — extracts per-track metrics
//!   (sample count, sizes, timing, sync samples) from a `moov` box.
//!
//! # Feature Flags
//!
//! | Feature | Default | Description |
//! |---------|---------|-------------|
//! | `std` | Yes | Enables standard library support (implies `alloc`) |
//! | `alloc` | | Enables heap allocation (`no_std` compatible) |
//!
//! # Example
//!
//! ```no_run
//! use mp4_bmff::{BoxDecode, BoxType, iter_boxes};
//! use mp4_bmff::boxes::bmff::MoovBoxView;
//! use mp4_inspect::tree;
//! use mp4_inspect::stats;
//!
//! let data = std::fs::read("video.mp4").unwrap();
//!
//! // Print box tree
//! let nodes = tree::build_tree(&data).unwrap();
//! print!("{}", tree::format_tree(&nodes));
//!
//! // Find moov via typed API and print sample stats
//! for result in iter_boxes(&data) {
//!     let raw = result.unwrap();
//!     if raw.boxtype() == BoxType::MOOV {
//!         let moov = MoovBoxView::decode(raw.payload()).unwrap();
//!         let tracks = stats::analyze(&moov).unwrap();
//!         for track in &tracks {
//!             print!("{}", track);
//!         }
//!     }
//! }
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod stats;
pub mod tree;
