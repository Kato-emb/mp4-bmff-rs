use mp4_bmff::boxes::bmff::SampleFlags;

/// Metadata for a single sample
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sample {
    /// Byte size of the sample.
    pub size: u64,
    /// Duration in media timescale units.
    pub duration: u32,
    /// Composition time offset (CTS - DTS).
    pub composition_time_offset: i32,
    /// Whether this sample is a sync sample (random access point).
    pub is_sync: bool,
    /// Per-sample dependency flags from `sdtp`. `None` if `sdtp` was absent.
    pub dependency: Option<SampleFlags>,
}

/// A sample with all metadata resolved, including the description index and file offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResolvedSample {
    /// Sample metadata from the sample table.
    pub sample: Sample,
    /// Index into `stsd` (1-based).
    pub description_index: u32,
    /// Byte offset of the sample in the file.
    pub offset: u64,
    /// Decode time (DTS) in media timescale units.
    /// TODO: デコードタイミングは、サンプルが時間的にどの位置にあるかを表現するため必要。
    pub decode_time: u64,
}
