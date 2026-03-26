//! Audio Sample Entry base types.
//!
//! This module provides the base structure for audio sample entries
//! as defined in ISO/IEC 14496-12 Section 12.2. All audio codec sample entries
//! (e.g. `mp4a`) extend this base with codec-specific boxes.

use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;
use crate::types::U16F16;

use crate::cursor::ReadCursor;

use super::SampleEntry;
use crate::boxes::bmff::ChnlBoxView;
use crate::boxes::bmff::DmixBoxView;

/// A reference to an Audio Sample Entry.
///
/// Contains common fields for all audio sample entries as defined in
/// ISO/IEC 14496-12. Codec-specific sample entries extend this base
/// with additional configuration boxes (e.g. `esds` for AAC).
///
/// # Structure
///
/// - `base`: Common sample entry fields (data reference index).
/// - `channelcount`: Number of audio channels (1=mono, 2=stereo, etc.).
/// - `samplesize`: Bits per sample (typically 16).
/// - `samplerate`: Audio sample rate in Hz (stored as 16.16 fixed-point).
#[derive(Debug)]
pub struct AudioSampleEntryView<'a> {
    base: SampleEntry,
    /// Number of audio channels (1=mono, 2=stereo, 6=5.1, etc.).
    pub channelcount: u16,
    /// Bits per sample (typically 16 for PCM-like formats).
    pub samplesize: u16,
    /// Audio sample rate in Hz, stored as 16.16 fixed-point.
    pub samplerate: U16F16,
    content: &'a [u8],
}

mod layout {
    use super::SampleEntry;

    pub(super) const RESERVED_0: usize = 8;
    pub(super) const PRE_DEFINED: usize = 2;
    pub(super) const RESERVED_1: usize = 2;

    pub(super) const FIXED_FIELDS_SIZE: usize = SampleEntry::size()
        + RESERVED_0
        + 2 // channelcount
        + 2 // samplesize
        + PRE_DEFINED
        + RESERVED_1
        + 4; // samplerate
}

impl<'a> AudioSampleEntryView<'a> {
    /// Returns a reference to the base `SampleEntry` fields.
    pub fn sample_entry(&self) -> &SampleEntry {
        &self.base
    }

    /// Returns an iterator over the child boxes contained within this audio sample entry.
    pub(crate) fn boxes_in(&self) -> BoxIter<'a> {
        BoxIter::new(self.content)
    }

    /// Returns the Channel Layout Box (`chnl`) if it exists within this audio sample entry, or `None` if it does not.
    ///
    /// # Errors
    ///
    /// Returns an error if there is a problem decoding the `chnl` box, such as invalid box structure or data.
    pub fn chnl(&self) -> Result<Option<ChnlBoxView<'a>>> {
        for b in self.boxes_in() {
            let b = b?;
            if b.boxtype() == BoxType::CHNL {
                let chnl = ChnlBoxView::decode(b.into_payload())?;
                return Ok(Some(chnl));
            }
        }

        Ok(None)
    }

    /// Returns an iterator over the Down Mix Instructions Boxes (`dmix`)
    /// within this audio sample entry.
    ///
    /// The spec allows multiple `dmix` boxes, each with a unique `downmix_id`.
    pub fn dmixes(&self) -> impl Iterator<Item = Result<DmixBoxView<'a>>> + 'a {
        self.boxes_in().filter_map(|result| match result {
            Ok(rawbox) if rawbox.boxtype() == BoxType::DMIX => {
                Some(DmixBoxView::decode(rawbox.into_payload()))
            }
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<Self> {
        let base = SampleEntry::parse_in(cur)?;

        cur.advance(layout::RESERVED_0)?;
        let channelcount = cur.read_u16_be()?;
        let samplesize = cur.read_u16_be()?;
        cur.advance(layout::PRE_DEFINED + layout::RESERVED_1)?;
        let samplerate = U16F16::from_raw(cur.read_u32_be()?);

        let content = cur.take(cur.remaining())?;

        Ok(AudioSampleEntryView {
            base,
            channelcount,
            samplesize,
            samplerate,
            content,
        })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use alloc::vec::Vec;

    use super::*;

    use crate::RawBoxRef;
    use crate::codec::boxed_len;
    use crate::codec::write_box_in;
    use crate::cursor::WriteCursor;

    use crate::boxes::bmff::ChnlBox;
    use crate::boxes::bmff::DmixBox;

    /// Owned version of [`AudioSampleEntryView`], with owned data and child boxes.
    #[derive(Debug, Clone)]
    pub struct AudioSampleEntry {
        /// Base sample entry fields (data reference index).
        pub base: SampleEntry,
        /// Number of audio channels (1=mono, 2=stereo, 6=5.1, etc.).
        pub channelcount: u16,
        /// Bits per sample (typically 16 for PCM-like formats).
        pub samplesize: u16,
        /// Audio sample rate in Hz, stored as 16.16 fixed-point.
        pub samplerate: U16F16,
        /// Optional Channel Layout Box (`chnl`) if present.
        pub chnl: Option<ChnlBox>,
        /// Down Mix Instructions Boxes (`dmix`). May contain zero or more entries.
        pub dmix: Vec<DmixBox>,
    }

    impl Default for AudioSampleEntry {
        fn default() -> Self {
            AudioSampleEntry {
                base: SampleEntry {
                    data_reference_index: 1,
                },
                channelcount: 2,
                samplesize: 16,
                samplerate: U16F16::from_raw(44100 << 16), // {default samplerate of media} << 16
                chnl: None,
                dmix: Vec::new(),
            }
        }
    }

    impl AudioSampleEntry {
        pub(crate) fn try_parse_view<'a>(
            view: &AudioSampleEntryView<'a>,
        ) -> Result<(Self, Vec<RawBoxRef<'a>>)> {
            let mut chnl = None;
            let mut dmix = Vec::new();
            let mut rest = Vec::new();

            for b in view.boxes_in() {
                let b = b?;
                match b.boxtype() {
                    BoxType::CHNL => {
                        if chnl.is_some() {
                            return Err(Error::new(ErrorKind::BoxDuplicate {
                                duplicate: BoxType::CHNL,
                            }));
                        }
                        chnl = Some(ChnlBox::decode(b.into_payload())?);
                    }
                    BoxType::DMIX => {
                        dmix.push(DmixBox::decode(b.into_payload())?);
                    }
                    _ => rest.push(b),
                }
            }

            Ok((
                AudioSampleEntry {
                    base: view.base,
                    channelcount: view.channelcount,
                    samplesize: view.samplesize,
                    samplerate: view.samplerate,
                    chnl,
                    dmix,
                },
                rest,
            ))
        }

        pub(crate) fn encode_len(&self) -> usize {
            let mut len = layout::FIXED_FIELDS_SIZE;

            if let Some(chnl) = &self.chnl {
                len += boxed_len(chnl);
            }

            for dmix in &self.dmix {
                len += boxed_len(dmix);
            }

            len
        }

        pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            self.base.write_in(cur)?;

            cur.reserve_zeros(layout::RESERVED_0)?;
            cur.write_u16_be(self.channelcount)?;
            cur.write_u16_be(self.samplesize)?;
            cur.reserve_zeros(layout::PRE_DEFINED + layout::RESERVED_1)?;
            cur.write_u32_be(self.samplerate.to_raw())?;

            if let Some(chnl) = &self.chnl {
                write_box_in(cur, chnl)?;
            }

            for dmix in &self.dmix {
                write_box_in(cur, dmix)?;
            }

            Ok(())
        }
    }
}

#[cfg(feature = "alloc")]
pub use owned::*;

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a raw AudioSampleEntry payload (28 bytes).
    fn raw_data() -> [u8; 28] {
        [
            // SampleEntry base (8 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x01, // data_reference_index = 1
            // AudioSampleEntry specific (20 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x02, // channelcount = 2
            0x00, 0x10, // samplesize = 16
            0x00, 0x00, // pre_defined
            0x00, 0x00, // reserved
            0xAC, 0x44, 0x00, 0x00, // samplerate = 44100 << 16
        ]
    }

    /// Creates a chnl box payload with predefined layout.
    fn chnl_box_bytes() -> Vec<u8> {
        let payload: [u8; 14] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x01, // stream_structure = CHANNEL_STRUCTURED
            0x02, // defined_layout = 2 (predefined)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, // omitted_channels_map = 3
        ];
        let size = (8 + payload.len()) as u32;
        let mut data = Vec::with_capacity(size as usize);
        data.extend_from_slice(&size.to_be_bytes());
        data.extend_from_slice(b"chnl");
        data.extend_from_slice(&payload);
        data
    }

    /// Creates a dmix box payload with in_stream coefficients.
    fn dmix_box_bytes() -> Vec<u8> {
        let payload: [u8; 7] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x01, // target_layout = 1
            0x02, // target_channel_count = 2
            0x85, // in_stream(1) + downmix_id = 5
        ];
        let size = (8 + payload.len()) as u32;
        let mut data = Vec::with_capacity(size as usize);
        data.extend_from_slice(&size.to_be_bytes());
        data.extend_from_slice(b"dmix");
        data.extend_from_slice(&payload);
        data
    }

    #[test]
    fn test_audio_sample_entry_view_decode() {
        let data = raw_data();
        let mut cur = ReadCursor::new(&data);
        let view = AudioSampleEntryView::parse_in(&mut cur).unwrap();

        assert_eq!(view.sample_entry().data_reference_index, 1);
        assert_eq!(view.channelcount, 2);
        assert_eq!(view.samplesize, 16);
        assert_eq!(view.samplerate, U16F16::from_raw(0xAC440000));
    }

    #[test]
    fn test_audio_sample_entry_view_truncated() {
        let data: [u8; 10] = [0x00; 10];
        let mut cur = ReadCursor::new(&data);
        assert!(AudioSampleEntryView::parse_in(&mut cur).is_err());
    }

    #[test]
    fn test_audio_sample_entry_view_no_child_boxes() {
        let data = raw_data();
        let mut cur = ReadCursor::new(&data);
        let view = AudioSampleEntryView::parse_in(&mut cur).unwrap();

        assert!(view.chnl().unwrap().is_none());
        assert_eq!(view.dmixes().count(), 0);
    }

    #[test]
    fn test_audio_sample_entry_view_chnl() {
        let mut data = Vec::new();
        data.extend_from_slice(&raw_data());
        data.extend_from_slice(&chnl_box_bytes());

        let mut cur = ReadCursor::new(&data);
        let view = AudioSampleEntryView::parse_in(&mut cur).unwrap();
        let chnl = view.chnl().unwrap().unwrap();

        assert_eq!(chnl.version, 0);
        assert_eq!(chnl.stream_structure, 0x01);
    }

    #[test]
    fn test_audio_sample_entry_view_dmix() {
        let mut data = Vec::new();
        data.extend_from_slice(&raw_data());
        data.extend_from_slice(&dmix_box_bytes());

        let mut cur = ReadCursor::new(&data);
        let view = AudioSampleEntryView::parse_in(&mut cur).unwrap();

        let dmix_boxes: Vec<_> = view.dmixes().collect();
        assert_eq!(dmix_boxes.len(), 1);
        let dmix = dmix_boxes[0].as_ref().unwrap();
        assert_eq!(dmix.target_layout, 1);
        assert!(dmix.in_stream);
    }

    #[test]
    fn test_audio_sample_entry_view_multiple_dmix() {
        let mut data = Vec::new();
        data.extend_from_slice(&raw_data());
        data.extend_from_slice(&dmix_box_bytes());
        data.extend_from_slice(&dmix_box_bytes());

        let mut cur = ReadCursor::new(&data);
        let view = AudioSampleEntryView::parse_in(&mut cur).unwrap();

        let dmix_boxes: Vec<_> = view.dmixes().collect();
        assert_eq!(dmix_boxes.len(), 2);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_audio_sample_entry_try_from() {
        let data = raw_data();
        let mut cur = ReadCursor::new(&data);
        let view = AudioSampleEntryView::parse_in(&mut cur).unwrap();
        let (owned, _) = AudioSampleEntry::try_parse_view(&view).unwrap();

        assert_eq!(owned.base.data_reference_index, 1);
        assert_eq!(owned.channelcount, 2);
        assert_eq!(owned.samplesize, 16);
        assert_eq!(owned.samplerate, U16F16::from_raw(0xAC440000));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_audio_sample_entry_try_from_with_chnl() {
        let mut data = Vec::new();
        data.extend_from_slice(&raw_data());
        data.extend_from_slice(&chnl_box_bytes());

        let mut cur = ReadCursor::new(&data);
        let view = AudioSampleEntryView::parse_in(&mut cur).unwrap();
        let (owned, _) = AudioSampleEntry::try_parse_view(&view).unwrap();

        assert!(owned.chnl.is_some());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_audio_sample_entry_round_trip() {
        let mut original = Vec::new();
        original.extend_from_slice(&raw_data());
        original.extend_from_slice(&chnl_box_bytes());

        let mut cur = ReadCursor::new(&original);
        let view = AudioSampleEntryView::parse_in(&mut cur).unwrap();
        let (owned, _) = AudioSampleEntry::try_parse_view(&view).unwrap();

        let mut encoded = vec![0u8; owned.encode_len()];
        let mut write_cur = crate::cursor::WriteCursor::new(&mut encoded);
        owned.write_in(&mut write_cur).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_audio_sample_entry_try_from_multiple_dmix() {
        let mut data = Vec::new();
        data.extend_from_slice(&raw_data());
        data.extend_from_slice(&dmix_box_bytes());
        data.extend_from_slice(&dmix_box_bytes());

        let mut cur = ReadCursor::new(&data);
        let view = AudioSampleEntryView::parse_in(&mut cur).unwrap();
        let (owned, _) = AudioSampleEntry::try_parse_view(&view).unwrap();

        assert_eq!(owned.dmix.len(), 2);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_audio_sample_entry_round_trip_with_dmix() {
        let mut original = Vec::new();
        original.extend_from_slice(&raw_data());
        original.extend_from_slice(&dmix_box_bytes());
        original.extend_from_slice(&dmix_box_bytes());

        let mut cur = ReadCursor::new(&original);
        let view = AudioSampleEntryView::parse_in(&mut cur).unwrap();
        let (owned, _) = AudioSampleEntry::try_parse_view(&view).unwrap();

        let mut encoded = vec![0u8; owned.encode_len()];
        let mut write_cur = crate::cursor::WriteCursor::new(&mut encoded);
        owned.write_in(&mut write_cur).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_audio_sample_entry_duplicate_chnl_error() {
        let mut data = Vec::new();
        data.extend_from_slice(&raw_data());
        data.extend_from_slice(&chnl_box_bytes());
        data.extend_from_slice(&chnl_box_bytes()); // duplicate

        let mut cur = ReadCursor::new(&data);
        let view = AudioSampleEntryView::parse_in(&mut cur).unwrap();
        let err = AudioSampleEntry::try_parse_view(&view).unwrap_err();

        match err.kind() {
            ErrorKind::BoxDuplicate { duplicate } => {
                assert_eq!(duplicate, BoxType::CHNL);
            }
            _ => panic!("Expected BoxDuplicate error"),
        }
    }
}
