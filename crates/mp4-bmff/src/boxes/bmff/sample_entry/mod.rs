//! Sample Entry base types and common fields.
//!
//! This module provides base structures for sample entries that appear in
//! the Sample Description Box (`stsd`). Sample entries describe the format
//! of media samples and reference data sources.

use crate::error::*;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

pub(crate) mod audio;
pub(crate) mod visual;

/// Fields common to all Sample Entry boxes.
///
/// This structure contains the base fields present in every sample entry
/// regardless of media type (audio, video, etc.).
///
/// # Structure
///
/// - `data_reference_index`: 1-based index into the Data Reference Box (`dref`)
///   identifying the data source for samples using this entry.
#[derive(Debug, Clone, Copy)]
pub struct SampleEntry {
    /// Index (1-based) into the Data Reference Box (`dref`) in the same track.
    /// Identifies the data source containing the media samples.
    pub data_reference_index: u16,
}

impl SampleEntry {
    const RESERVED: usize = 6;

    /// Returns the size of the `SampleEntry` data.
    pub const fn size() -> usize {
        Self::RESERVED + 2 // reserved + data_reference_index
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'_>) -> Result<Self> {
        // Skip reserved bytes
        cur.advance(Self::RESERVED)?;
        let data_reference_index = cur.read_u16_be()?;

        Ok(SampleEntry {
            data_reference_index,
        })
    }

    pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
        // Write reserved bytes (6 bytes of zeros)
        cur.reserve_zeros(Self::RESERVED)?;
        cur.write_u16_be(self.data_reference_index)?;
        Ok(())
    }
}
