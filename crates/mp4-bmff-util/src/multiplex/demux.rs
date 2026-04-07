//! MP4 demuxer implementation.
//!
//! This module provides [`Demuxer`] for non-fragmented MP4 and [`FragmentedDemuxer`]
//! for fragmented MP4 (fMP4) input. Both accept box structures (`MoovBox`, `MoofBox`)
//! and produce [`Track`], [`Chunk`], and [`Sample`](super::Sample) values.

mod disassemble;
