use alloc::vec::Vec;

use mp4_bmff::boxes::bmff::ElstEntry;
use mp4_bmff::types::LanguageCode;
use mp4_bmff::types::Matrix;
use mp4_bmff::types::QuickTimeDateTime;

use crate::multiplex::Result;
use crate::multiplex::error::Error;
use crate::multiplex::error::ErrorKind;

use super::Timescale;
use super::TrackId;

use super::ChunkLayout;
use super::SampleDescription;
use super::SampleSpec;

/// A struct representing a track in a media file, containing information about the track's properties, sample descriptions, chunk layout, and edit list.
#[derive(Debug, Clone)]
pub struct Track {
    pub(crate) track_id: TrackId,
    pub(crate) timescale: Timescale,
    pub(crate) creation_time: QuickTimeDateTime,
    pub(crate) modification_time: QuickTimeDateTime,
    pub(crate) language: LanguageCode,
    pub(crate) matrix: Matrix,
    pub(crate) alternate_group: i16,
    pub(crate) descriptions: Vec<SampleDescription>,
    pub(crate) sample_spec: SampleSpec,
    pub(crate) chunk_layout: ChunkLayout,
    pub(crate) edit_list: Option<Vec<ElstEntry>>,
}

impl Track {
    /// Creates a new track with the given ID, timescale and sample descriptions. Returns an error if `track_id` or `timescale` is zero. Optional fields default to BMFF's standard defaults and can be set via the `with_*` builder methods.
    pub fn new(
        track_id: u32,
        timescale: u32,
        descriptions: Vec<SampleDescription>,
    ) -> Result<Self> {
        let track_id = TrackId::new(track_id).ok_or_else(|| {
            Error::new(ErrorKind::InvalidInput).with_message("Track ID must be non-zero")
        })?;
        let timescale = Timescale::new(timescale).ok_or_else(|| {
            Error::new(ErrorKind::InvalidInput).with_message("Track timescale must be non-zero")
        })?;
        Ok(Self {
            track_id,
            timescale,
            creation_time: QuickTimeDateTime::default(),
            modification_time: QuickTimeDateTime::default(),
            language: LanguageCode::default(),
            matrix: Matrix::default(),
            alternate_group: 0,
            descriptions,
            sample_spec: SampleSpec::default(),
            chunk_layout: ChunkLayout::default(),
            edit_list: None,
        })
    }

    /// Returns this track's identifier.
    pub fn track_id(&self) -> TrackId {
        self.track_id
    }

    /// Returns this track's timescale (units per second).
    pub fn timescale(&self) -> u32 {
        self.timescale.as_u32()
    }

    /// Returns the track creation time as recorded in `tkhd` / `mdhd`.
    pub fn creation_time(&self) -> QuickTimeDateTime {
        self.creation_time
    }

    /// Returns the track modification time as recorded in `tkhd` / `mdhd`.
    pub fn modification_time(&self) -> QuickTimeDateTime {
        self.modification_time
    }

    /// Sets the track creation time written into the resulting `tkhd` / `mdhd`.
    pub fn set_creation_time(&mut self, t: QuickTimeDateTime) {
        self.creation_time = t;
    }

    /// Sets the track modification time written into the resulting `tkhd` / `mdhd`.
    pub fn set_modification_time(&mut self, t: QuickTimeDateTime) {
        self.modification_time = t;
    }

    /// Sets the language code (ISO-639-2/T) of this track.
    pub fn with_language(mut self, language: LanguageCode) -> Self {
        self.language = language;
        self
    }

    /// Sets the display matrix for this track.
    pub fn with_matrix(mut self, matrix: Matrix) -> Self {
        self.matrix = matrix;
        self
    }

    /// Sets the alternate group for this track. Tracks with the same non-zero `alternate_group` value form a group from which only one should be played at a time.
    pub fn with_alternate_group(mut self, group: i16) -> Self {
        self.alternate_group = group;
        self
    }

    /// Sets the sample specification (timing, sizes, sync samples, etc.) for this track.
    pub fn with_sample_spec(mut self, spec: SampleSpec) -> Self {
        self.sample_spec = spec;
        self
    }

    /// Replaces the sample specification of this track.
    pub fn set_sample_spec(&mut self, spec: SampleSpec) {
        self.sample_spec = spec;
    }

    /// Sets the edit list for this track.
    pub fn with_edit_list(mut self, edits: Vec<ElstEntry>) -> Self {
        self.edit_list = Some(edits);
        self
    }

    /// Returns the sample descriptions for this track.
    pub fn descriptions(&self) -> &[SampleDescription] {
        &self.descriptions
    }

    /// Returns the media specification for this track.
    pub fn sample_spec(&self) -> &SampleSpec {
        &self.sample_spec
    }

    /// Returns a mutable reference to the sample specification for this track.
    pub fn sample_spec_mut(&mut self) -> &mut SampleSpec {
        &mut self.sample_spec
    }

    /// Returns the chunk layout for this track.
    pub fn chunk_layout(&self) -> &ChunkLayout {
        &self.chunk_layout
    }

    /// Returns the sample offset (file position and size) for the given sample index, if it exists.
    pub fn sample_offset(&self, sample_index: u32) -> Option<(u64, u32)> {
        let stsz_entry = *self.sample_spec.stsz_entries.get(sample_index as usize)?;
        let (chunk_0, sample_in_chunk) = self.chunk_layout.resolve(sample_index)?;
        let chunk_base = *self.chunk_layout.chunk_offsets.get(chunk_0 as usize)?;

        let first_sample_in_chunk = sample_index - sample_in_chunk;
        let intra_offset: u64 = self.sample_spec.stsz_entries
            [first_sample_in_chunk as usize..sample_index as usize]
            .iter()
            .map(|&s| s.entry_size as u64)
            .sum();

        Some((chunk_base + intra_offset, stsz_entry.entry_size))
    }

    /// Sets the chunk layout for this track. The number of samples in the chunk layout must match the number of samples in the sample specification, otherwise an error is returned.
    pub fn set_chunk_layout(&mut self, layout: ChunkLayout) -> Result<()> {
        if layout.sample_count() != self.sample_spec.sample_count() {
            return Err(Error::new(ErrorKind::SampleCountMismatch {
                expected: self.sample_spec.sample_count(),
                actual: layout.sample_count(),
            }));
        }

        self.chunk_layout = layout;
        Ok(())
    }

    pub(crate) fn edit_duration(&self) -> Option<u64> {
        self.edit_list.as_ref().and_then(|edits| {
            edits
                .iter()
                .find(|edit| edit.media_time != -1)
                .map(|edit| edit.segment_duration)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mp4_bmff::types::FourCC;

    fn dummy_descriptions() -> Vec<SampleDescription> {
        vec![SampleDescription::Other(FourCC::new(*b"test"))]
    }

    fn elst(media_time: i64, segment_duration: u64) -> ElstEntry {
        ElstEntry {
            segment_duration,
            media_time,
            media_rate: mp4_bmff::types::I16F16::from_integer(1),
        }
    }

    #[test]
    fn new_rejects_zero_track_id() {
        let err = Track::new(0, 48_000, dummy_descriptions()).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }

    #[test]
    fn new_rejects_zero_timescale() {
        let err = Track::new(1, 0, dummy_descriptions()).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }

    #[test]
    fn new_initializes_defaults() {
        let track = Track::new(7, 48_000, dummy_descriptions()).unwrap();
        assert_eq!(track.track_id().as_u32(), 7);
        assert_eq!(track.timescale(), 48_000);
        assert_eq!(track.alternate_group, 0);
        assert!(track.edit_list.is_none());
        assert_eq!(track.sample_spec().sample_count(), 0);
    }

    #[test]
    fn builders_chain() {
        let track = Track::new(1, 48_000, dummy_descriptions())
            .unwrap()
            .with_alternate_group(2)
            .with_edit_list(vec![elst(0, 100)])
            .with_sample_spec(SampleSpec::from_samples(&[10; 4], &[1; 4], None, None, None));

        assert_eq!(track.alternate_group, 2);
        assert!(track.edit_list.is_some());
        assert_eq!(track.sample_spec().sample_count(), 4);
    }

    #[test]
    fn set_chunk_layout_rejects_count_mismatch() {
        let mut track = Track::new(1, 48_000, dummy_descriptions())
            .unwrap()
            .with_sample_spec(SampleSpec::from_samples(&[10; 4], &[1; 4], None, None, None));
        // layout that resolves to 6 samples vs spec's 4
        let layout = ChunkLayout::new(&[2, 2, 2], vec![100, 200, 300]).unwrap();
        let err = track.set_chunk_layout(layout).unwrap_err();
        match err.kind() {
            ErrorKind::SampleCountMismatch { expected, actual } => {
                assert_eq!(expected, 4);
                assert_eq!(actual, 6);
            }
            other => panic!("unexpected error kind: {:?}", other),
        }
    }

    #[test]
    fn set_chunk_layout_accepts_matching_count() {
        let mut track = Track::new(1, 48_000, dummy_descriptions())
            .unwrap()
            .with_sample_spec(SampleSpec::from_samples(&[10; 4], &[1; 4], None, None, None));
        let layout = ChunkLayout::new(&[2, 2], vec![100, 200]).unwrap();
        assert!(track.set_chunk_layout(layout).is_ok());
        assert_eq!(track.chunk_layout().chunk_offsets, vec![100, 200]);
    }

    #[test]
    fn sample_offset_returns_position_and_size() {
        // 4 samples of size 10, 2 per chunk at offsets [100, 200]
        let mut track = Track::new(1, 48_000, dummy_descriptions())
            .unwrap()
            .with_sample_spec(SampleSpec::from_samples(&[1; 4], &[10; 4], None, None, None));
        track
            .set_chunk_layout(ChunkLayout::new(&[2, 2], vec![100, 200]).unwrap())
            .unwrap();

        // sample 0: chunk 0 base 100, intra-offset 0, size 10
        assert_eq!(track.sample_offset(0), Some((100, 10)));
        // sample 1: chunk 0, intra-offset 10
        assert_eq!(track.sample_offset(1), Some((110, 10)));
        // sample 2: chunk 1 base 200, intra-offset 0
        assert_eq!(track.sample_offset(2), Some((200, 10)));
        // sample 3
        assert_eq!(track.sample_offset(3), Some((210, 10)));
        // out of bounds
        assert_eq!(track.sample_offset(4), None);
    }

    #[test]
    fn edit_duration_picks_first_non_empty_edit() {
        let track = Track::new(1, 48_000, dummy_descriptions())
            .unwrap()
            .with_edit_list(vec![elst(-1, 50), elst(0, 200), elst(0, 999)]);
        // skips the leading empty edit (media_time == -1), picks segment_duration of next
        assert_eq!(track.edit_duration(), Some(200));
    }

    #[test]
    fn edit_duration_none_when_no_edit_list() {
        let track = Track::new(1, 48_000, dummy_descriptions()).unwrap();
        assert_eq!(track.edit_duration(), None);
    }

    #[test]
    fn edit_duration_none_when_only_empty_edits() {
        let track = Track::new(1, 48_000, dummy_descriptions())
            .unwrap()
            .with_edit_list(vec![elst(-1, 100)]);
        assert_eq!(track.edit_duration(), None);
    }
}
