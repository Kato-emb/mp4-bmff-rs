use alloc::vec::Vec;

use super::Result;
use super::TrackId;
use super::error::*;

/// Physical data layout of an MP4 file, describing how sample data is
/// organized into chunks across tracks.
#[derive(Debug, Clone, Default)]
pub struct DataLayout {
    chunks: Vec<Chunk>,
}

impl DataLayout {
    /// Creates a new empty layout.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a chunk to the layout.
    ///
    /// A chunk is a contiguous region of sample data for a single track.
    /// `base_offset` is the byte offset of the first sample in the chunk,
    /// and `sizes` contains the byte size of each sample in the chunk.
    ///
    /// # Errors
    ///
    /// Returns an error if `sizes` is empty.
    pub fn add_chunk(
        &mut self,
        track_id: TrackId,
        base_offset: u64,
        sizes: Vec<u32>,
    ) -> Result<()> {
        if sizes.is_empty() {
            return Err(Error::new(ErrorKind::InvalidInput)
                .with_message("Chunk must contain at least one sample"));
        }

        self.chunks.push(Chunk {
            track_id,
            base_offset,
            sizes,
        });

        Ok(())
    }

    /// Returns an iterator over chunks belonging to the given track.
    pub fn chunks_for_track(&self, track_id: TrackId) -> impl Iterator<Item = &Chunk> {
        self.chunks
            .iter()
            .filter(move |chunk| chunk.track_id == track_id)
    }
}

/// A contiguous group of samples for a single track, stored at a known
/// byte offset in the file.
#[derive(Debug, Clone)]
pub struct Chunk {
    track_id: TrackId,
    base_offset: u64,
    sizes: Vec<u32>,
}

impl Chunk {
    /// Returns the track that this chunk belongs to.
    pub fn track_id(&self) -> TrackId {
        self.track_id
    }

    /// Returns the byte offset of the first sample in this chunk.
    pub fn base_offset(&self) -> u64 {
        self.base_offset
    }

    /// Returns the byte sizes of each sample in this chunk.
    pub fn sizes(&self) -> &[u32] {
        &self.sizes
    }

    /// Returns the number of samples in this chunk.
    pub fn num_samples(&self) -> usize {
        self.sizes.len()
    }

    /// Returns an iterator over the byte offset of each sample in this chunk,
    /// computed by accumulating sizes from `base_offset`.
    pub fn offsets(&self) -> impl Iterator<Item = u64> + '_ {
        let mut offset = self.base_offset;
        self.sizes.iter().map(move |&s| {
            let o = offset;
            offset += u64::from(s);
            o
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(id: u32) -> TrackId {
        TrackId::new(id).unwrap()
    }

    #[test]
    fn new_layout_is_empty() {
        let layout = DataLayout::new();
        assert!(layout.chunks_for_track(track(1)).next().is_none());
    }

    #[test]
    fn add_chunk_single() {
        let mut layout = DataLayout::new();
        layout.add_chunk(track(1), 0, vec![100, 200, 300]).unwrap();

        let chunks: Vec<_> = layout.chunks_for_track(track(1)).collect();
        assert_eq!(chunks.len(), 1);
        let chunk = chunks[0];
        assert_eq!(chunk.track_id(), track(1));
        assert_eq!(chunk.base_offset(), 0);
        assert_eq!(chunk.sizes(), &[100, 200, 300]);
    }

    #[test]
    fn add_chunk_empty_sizes_returns_error() {
        let mut layout = DataLayout::new();
        assert!(layout.add_chunk(track(1), 0, vec![]).is_err());
        assert!(layout.chunks_for_track(track(1)).next().is_none());
    }

    #[test]
    fn offsets_computes_cumulative_offset() {
        let mut layout = DataLayout::new();
        layout.add_chunk(track(1), 1000, vec![50, 60, 70]).unwrap();
        let chunk = layout.chunks_for_track(track(1)).next().unwrap();

        let offsets: Vec<_> = chunk.offsets().collect();
        assert_eq!(offsets, vec![1000, 1050, 1110]);
    }

    #[test]
    fn interleaved_tracks() {
        let mut layout = DataLayout::new();
        layout.add_chunk(track(1), 0, vec![100]).unwrap();
        layout.add_chunk(track(2), 100, vec![200]).unwrap();
        layout.add_chunk(track(1), 300, vec![150]).unwrap();
        layout.add_chunk(track(2), 450, vec![250]).unwrap();

        assert_eq!(
            layout.chunks_for_track(track(1)).count() + layout.chunks_for_track(track(2)).count(),
            4
        );

        let t1: Vec<_> = layout.chunks_for_track(track(1)).collect();
        assert_eq!(t1.len(), 2);
        assert_eq!(t1[0].base_offset(), 0);
        assert_eq!(t1[1].base_offset(), 300);

        let t2: Vec<_> = layout.chunks_for_track(track(2)).collect();
        assert_eq!(t2.len(), 2);
        assert_eq!(t2[0].base_offset(), 100);
        assert_eq!(t2[1].base_offset(), 450);
    }

    #[test]
    fn chunks_for_nonexistent_track() {
        let mut layout = DataLayout::new();
        layout.add_chunk(track(1), 0, vec![100]).unwrap();

        let chunks: Vec<_> = layout.chunks_for_track(track(99)).collect();
        assert!(chunks.is_empty());
    }

    #[test]
    fn single_sample_chunk() {
        let mut layout = DataLayout::new();
        layout.add_chunk(track(1), 500, vec![42]).unwrap();
        let chunk = layout.chunks_for_track(track(1)).next().unwrap();

        let offsets: Vec<_> = chunk.offsets().collect();
        assert_eq!(offsets, vec![500]);
    }
}
