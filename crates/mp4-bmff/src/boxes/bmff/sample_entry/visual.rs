//! Visual Sample Entry base types.
//!
//! This module provides the base structure for visual (video) sample entries
//! as defined in ISO/IEC 14496-12 Section 12.1. All video codec sample entries
//! (e.g. `avc1`, `hvc1`, `mp4v`) extend this base with codec-specific boxes.

use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;
use crate::types::U16F16;

use crate::cursor::ReadCursor;

use super::SampleEntry;
use crate::boxes::bmff::clap::ClapBox;
use crate::boxes::bmff::colr::ColrBoxView;
use crate::boxes::bmff::pasp::PaspBox;

/// Compressor name field for Visual Sample Entries.
///
/// The `compressorname` field is a 32-byte array where the first byte indicates the length of the compressor name string (up to 31 bytes),
/// followed by the UTF-8 encoded compressor name data. This field is used to provide a human-readable description of the compressor used for encoding the video sample,
/// and is typically included in visual sample entries such as `avc1`, `mp4v`, and `hvc1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompressorName([u8; 32]);

impl CompressorName {
    /// Creates a new `CompressorName` from a string slice.
    ///
    /// The input string is truncated to 31 bytes if it exceeds that length, and the first byte of
    /// the `CompressorName` is set to the length of the string (up to 31).
    pub fn new(name: &str) -> Self {
        let mut compressorname = [0u8; 32];
        let bytes = name.as_bytes();
        let len = bytes.len().min(31);
        compressorname[0] = len as u8;
        compressorname[1..1 + len].copy_from_slice(&bytes[..len]);
        CompressorName(compressorname)
    }

    /// Creates a `CompressorName` from a raw 32-byte array, including the length byte and name data.
    pub fn from_raw(raw: [u8; 32]) -> Self {
        CompressorName(raw)
    }

    /// Returns the raw 32-byte array representation of the `CompressorName`, including the length byte and name data.
    pub const fn into_raw(self) -> [u8; 32] {
        self.0
    }

    /// Returns the compressor name as a string slice, if valid UTF-8.
    pub fn as_str(&self) -> Option<&str> {
        let len = (self.0[0] as usize).min(31);
        let name_bytes = &self.0[1..1 + len];
        core::str::from_utf8(name_bytes).ok()
    }
}

/// A reference to a Visual Sample Entry.
///
/// Contains common fields for all video sample entries as defined in
/// ISO/IEC 14496-12. Codec-specific sample entries extend this base
/// with additional configuration boxes (e.g. `avcC` for AVC).
///
/// # Structure
///
/// - `base`: Common sample entry fields (data reference index).
/// - `width`: Video width in pixels.
/// - `height`: Video height in pixels.
/// - `horizresolution`: Horizontal resolution in pixels per inch (72 dpi typical).
/// - `vertresolution`: Vertical resolution in pixels per inch (72 dpi typical).
/// - `frame_count`: Number of frames per sample (usually 1).
/// - `compressorname`: 32-byte field with compressor name (length-prefixed).
/// - `depth`: Color depth in bits (typically 0x0018 = 24 for color video).
#[derive(Debug)]
pub struct VisualSampleEntryView<'a> {
    base: SampleEntry,
    /// Video width in pixels.
    pub width: u16,
    /// Video height in pixels.
    pub height: u16,
    /// Horizontal resolution in pixels per inch (16.16 fixed-point).
    pub horizresolution: U16F16,
    /// Vertical resolution in pixels per inch (16.16 fixed-point).
    pub vertresolution: U16F16,
    /// Number of frames per sample (usually 1).
    pub frame_count: u16,
    /// Compressor name field.
    pub compressorname: CompressorName,
    /// Color depth in bits. 0x0018 (24) indicates color video without alpha.
    pub depth: u16,
    content: &'a [u8],
}

/// Wire-format layout constants for the Visual Sample Entry fixed fields.
mod layout {
    use super::SampleEntry;

    pub(super) const PRE_DEFINED_0: usize = 2;
    pub(super) const RESERVED_0: usize = 2;
    pub(super) const PRE_DEFINED_1: usize = 12;
    pub(super) const RESERVED_1: usize = 4;
    pub(super) const PRE_DEFINED_2: usize = 2;

    /// Total size in bytes of the Visual Sample Entry fixed fields
    /// (SampleEntry base + visual-specific fields, excluding trailing child boxes).
    pub(super) const FIXED_FIELDS_SIZE: usize = SampleEntry::size()
        + PRE_DEFINED_0
        + RESERVED_0
        + PRE_DEFINED_1
        + 2  // width
        + 2  // height
        + 4  // horizresolution
        + 4  // vertresolution
        + RESERVED_1
        + 2  // frame_count
        + 32 // compressorname
        + 2  // depth
        + PRE_DEFINED_2;
}

impl<'a> VisualSampleEntryView<'a> {
    /// Returns a reference to the base `SampleEntry` fields.
    pub fn sample_entry(&self) -> &SampleEntry {
        &self.base
    }

    /// Returns an iterator over the child boxes contained within this visual sample entry.
    pub(crate) fn boxes_in(&self) -> BoxIter<'a> {
        BoxIter::new(self.content)
    }

    /// Returns the Colour Information Box (`colr`) if present within this visual sample entry.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed or too short.
    pub fn colr(&self) -> Result<Option<ColrBoxView<'a>>> {
        for b in self.boxes_in() {
            let b = b?;
            if b.boxtype() == BoxType::COLR {
                let colr = ColrBoxView::decode(b.into_payload())?;
                return Ok(Some(colr));
            }
        }

        Ok(None)
    }

    /// Returns the Clean Aperture Box (`clap`) if present within this visual sample entry.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed or too short.
    pub fn clap(&self) -> Result<Option<ClapBox>> {
        for b in self.boxes_in() {
            let b = b?;
            if b.boxtype() == BoxType::CLAP {
                let clap = ClapBox::decode(b.into_payload())?;
                return Ok(Some(clap));
            }
        }

        Ok(None)
    }

    /// Returns the Pixel Aspect Ratio Box (`pasp`) if present within this visual sample entry.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed or too short.
    pub fn pasp(&self) -> Result<Option<PaspBox>> {
        for b in self.boxes_in() {
            let b = b?;
            if b.boxtype() == BoxType::PASP {
                let pasp = PaspBox::decode(b.into_payload())?;
                return Ok(Some(pasp));
            }
        }

        Ok(None)
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<Self> {
        let base = SampleEntry::parse_in(cur)?;

        // Skip pre_defined (2 bytes)
        cur.advance(layout::PRE_DEFINED_0)?;
        // Skip reserved (2 bytes)
        cur.advance(layout::RESERVED_0)?;
        // Skip pre_defined (12 bytes)
        cur.advance(layout::PRE_DEFINED_1)?;

        let width = cur.read_u16_be()?;
        let height = cur.read_u16_be()?;

        let horizresolution = U16F16::from_raw(cur.read_u32_be()?);
        let vertresolution = U16F16::from_raw(cur.read_u32_be()?);

        // Skip reserved (4 bytes)
        cur.advance(layout::RESERVED_1)?;

        let frame_count = cur.read_u16_be()?;

        // compressorname is a 32-byte field: first byte is length, followed by 31 bytes of data
        let compressorname = CompressorName::from_raw(cur.read_array::<32>()?);

        let depth = cur.read_u16_be()?;

        // Skip pre_defined (2 bytes)
        cur.advance(layout::PRE_DEFINED_2)?;

        let content = cur.take(cur.remaining())?;

        Ok(VisualSampleEntryView {
            base,
            width,
            height,
            horizresolution,
            vertresolution,
            frame_count,
            compressorname,
            depth,
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

    use crate::boxes::bmff::colr::ColrBox;

    /// Owned version of [`VisualSampleEntryView`], with owned data and child boxes.
    #[derive(Debug, Clone)]
    pub struct VisualSampleEntry {
        /// Base sample entry fields (data reference index).
        pub base: SampleEntry,
        /// Video width in pixels.
        pub width: u16,
        /// Video height in pixels.
        pub height: u16,
        /// Horizontal resolution in pixels per inch (16.16 fixed-point).
        pub horizresolution: U16F16,
        /// Vertical resolution in pixels per inch (16.16 fixed-point).
        pub vertresolution: U16F16,
        /// Number of frames per sample (usually 1).
        pub frame_count: u16,
        /// Compressor name field.
        pub compressorname: CompressorName,
        /// Color depth in bits. 0x0018 (24) indicates color video without alpha.
        pub depth: u16,
        /// Optional Colour Information Box (`colr`) if present.
        pub colr: Option<ColrBox>,
        /// Optional Clean Aperture Box (`clap`) if present.
        pub clap: Option<ClapBox>,
        /// Optional Pixel Aspect Ratio Box (`pasp`) if present.
        pub pasp: Option<PaspBox>,
    }

    impl Default for VisualSampleEntry {
        fn default() -> Self {
            VisualSampleEntry {
                base: SampleEntry {
                    data_reference_index: 1,
                },
                width: 0,
                height: 0,
                horizresolution: U16F16::from_raw(0x00480000), // 72 dpi
                vertresolution: U16F16::from_raw(0x00480000),  // 72 dpi
                frame_count: 1,
                compressorname: CompressorName::new(""),
                depth: 0x0018, // 24 bits (color video without alpha)
                colr: None,
                clap: None,
                pasp: None,
            }
        }
    }

    impl VisualSampleEntry {
        pub(crate) fn try_parse_view<'a>(
            view: &VisualSampleEntryView<'a>,
        ) -> Result<(Self, Vec<RawBoxRef<'a>>)> {
            let mut colr = None;
            let mut clap = None;
            let mut pasp = None;
            let mut rest = Vec::new();

            for b in view.boxes_in() {
                let b = b?;
                match b.boxtype() {
                    BoxType::COLR => {
                        if colr.is_some() {
                            return Err(Error::new(ErrorKind::BoxDuplicate {
                                duplicate: BoxType::COLR,
                            }));
                        }
                        colr = Some(ColrBox::decode(b.into_payload())?);
                    }
                    BoxType::CLAP => {
                        if clap.is_some() {
                            return Err(Error::new(ErrorKind::BoxDuplicate {
                                duplicate: BoxType::CLAP,
                            }));
                        }
                        clap = Some(ClapBox::decode(b.into_payload())?);
                    }
                    BoxType::PASP => {
                        if pasp.is_some() {
                            return Err(Error::new(ErrorKind::BoxDuplicate {
                                duplicate: BoxType::PASP,
                            }));
                        }
                        pasp = Some(PaspBox::decode(b.into_payload())?);
                    }
                    _ => rest.push(b),
                }
            }

            Ok((
                VisualSampleEntry {
                    base: view.base,
                    width: view.width,
                    height: view.height,
                    horizresolution: view.horizresolution,
                    vertresolution: view.vertresolution,
                    frame_count: view.frame_count,
                    compressorname: view.compressorname,
                    depth: view.depth,
                    colr,
                    clap,
                    pasp,
                },
                rest,
            ))
        }

        pub(crate) fn encode_len(&self) -> usize {
            let mut len = layout::FIXED_FIELDS_SIZE;

            if let Some(colr) = &self.colr {
                len += boxed_len(colr);
            }

            if let Some(clap) = &self.clap {
                len += boxed_len(clap);
            }

            if let Some(pasp) = &self.pasp {
                len += boxed_len(pasp);
            }

            len
        }

        pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            self.base.write_in(cur)?;

            // pre_defined (2 bytes)
            cur.write_slice(&[0u8; layout::PRE_DEFINED_0])?;
            // reserved (2 bytes)
            cur.write_slice(&[0u8; layout::RESERVED_0])?;
            // pre_defined (12 bytes)
            cur.write_slice(&[0u8; layout::PRE_DEFINED_1])?;

            cur.write_u16_be(self.width)?;
            cur.write_u16_be(self.height)?;

            cur.write_u32_be(self.horizresolution.to_raw())?;
            cur.write_u32_be(self.vertresolution.to_raw())?;

            // reserved (4 bytes)
            cur.write_slice(&[0u8; layout::RESERVED_1])?;

            cur.write_u16_be(self.frame_count)?;

            cur.write_array(&self.compressorname.into_raw())?;

            cur.write_u16_be(self.depth)?;

            // pre_defined (2 bytes, -1)
            cur.write_slice(&[0xFF, 0xFF])?;

            if let Some(colr) = &self.colr {
                write_box_in(cur, colr)?;
            }

            if let Some(clap) = &self.clap {
                write_box_in(cur, clap)?;
            }

            if let Some(pasp) = &self.pasp {
                write_box_in(cur, pasp)?;
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

    fn raw_data() -> [u8; 78] {
        let mut data = [0u8; 78];
        // reserved (6 bytes)
        // data_reference_index (2 bytes)
        data[6] = 0x00;
        data[7] = 0x01; // data_reference_index = 1
        // pre_defined (2 bytes)
        // reserved (2 bytes)
        // pre_defined (12 bytes)
        // width (2 bytes) at offset 24
        data[24] = 0x05;
        data[25] = 0x00; // width = 1280
        // height (2 bytes) at offset 26
        data[26] = 0x02;
        data[27] = 0xD0; // height = 720
        // horizresolution (4 bytes) at offset 28
        data[28] = 0x00;
        data[29] = 0x48;
        data[30] = 0x00;
        data[31] = 0x00; // 72.0 dpi
        // vertresolution (4 bytes) at offset 32
        data[32] = 0x00;
        data[33] = 0x48;
        data[34] = 0x00;
        data[35] = 0x00; // 72.0 dpi
        // reserved (4 bytes)
        // frame_count (2 bytes) at offset 40
        data[40] = 0x00;
        data[41] = 0x01; // frame_count = 1
        // compressorname (32 bytes) at offset 42
        // depth (2 bytes) at offset 74
        data[74] = 0x00;
        data[75] = 0x18; // depth = 24
        // pre_defined (2 bytes) at offset 76
        data[76] = 0xFF;
        data[77] = 0xFF; // pre_defined = -1
        data
    }

    #[test]
    fn test_visual_sample_entry_view_decode() {
        let data = raw_data();
        let mut cur = ReadCursor::new(&data);
        let view = VisualSampleEntryView::parse_in(&mut cur).unwrap();

        assert_eq!(view.sample_entry().data_reference_index, 1);
        assert_eq!(view.width, 1280);
        assert_eq!(view.height, 720);
        assert_eq!(view.horizresolution, U16F16::from_raw(0x00480000));
        assert_eq!(view.vertresolution, U16F16::from_raw(0x00480000));
        assert_eq!(view.frame_count, 1);
        assert_eq!(view.depth, 0x0018);
    }

    #[test]
    fn test_visual_sample_entry_view_truncated() {
        let data: [u8; 30] = [0x00; 30];
        let mut cur = ReadCursor::new(&data);
        let result = VisualSampleEntryView::parse_in(&mut cur);
        assert!(result.is_err());
    }

    #[test]
    fn test_visual_sample_entry_view_pasp() {
        let mut data = Vec::new();
        data.extend_from_slice(&raw_data());
        // pasp box: size(4) + type(4) + h_spacing(4) + v_spacing(4) = 16 bytes
        data.extend_from_slice(&[
            0x00, 0x00, 0x00, 0x10, // size = 16
            b'p', b'a', b's', b'p', // type = "pasp"
            0x00, 0x00, 0x00, 0x0A, // h_spacing = 10
            0x00, 0x00, 0x00, 0x0B, // v_spacing = 11
        ]);

        let mut cur = ReadCursor::new(&data);
        let view = VisualSampleEntryView::parse_in(&mut cur).unwrap();
        let pasp = view.pasp().unwrap().unwrap();

        assert_eq!(pasp.h_spacing, 10);
        assert_eq!(pasp.v_spacing, 11);
    }

    #[test]
    fn test_visual_sample_entry_view_no_child_boxes() {
        let data = raw_data();
        let mut cur = ReadCursor::new(&data);
        let view = VisualSampleEntryView::parse_in(&mut cur).unwrap();

        assert!(view.pasp().unwrap().is_none());
        assert!(view.clap().unwrap().is_none());
        assert!(view.colr().unwrap().is_none());
    }

    #[test]
    fn test_compressor_name_new() {
        let name = CompressorName::new("VideoHandler");
        assert_eq!(name.as_str(), Some("VideoHandler"));
    }

    #[test]
    fn test_compressor_name_truncate() {
        let long = "a]234567890123456789012345678901234567890";
        let name = CompressorName::new(long);
        // Truncated to 31 bytes
        assert_eq!(name.as_str().unwrap().len(), 31);
    }

    #[test]
    fn test_compressor_name_empty() {
        let name = CompressorName::new("");
        assert_eq!(name.as_str(), Some(""));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_visual_sample_entry_try_from() {
        let data = raw_data();
        let mut cur = ReadCursor::new(&data);
        let view = VisualSampleEntryView::parse_in(&mut cur).unwrap();
        let (owned, _) = VisualSampleEntry::try_parse_view(&view).unwrap();

        assert_eq!(owned.base.data_reference_index, 1);
        assert_eq!(owned.width, 1280);
        assert_eq!(owned.height, 720);
        assert_eq!(owned.frame_count, 1);
        assert_eq!(owned.depth, 0x0018);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_visual_sample_entry_round_trip() {
        let mut original = Vec::new();
        original.extend_from_slice(&raw_data());
        // Add pasp child box
        original.extend_from_slice(&[
            0x00, 0x00, 0x00, 0x10, // size = 16
            b'p', b'a', b's', b'p', // type = "pasp"
            0x00, 0x00, 0x00, 0x01, // h_spacing = 1
            0x00, 0x00, 0x00, 0x01, // v_spacing = 1
        ]);

        let mut cur = ReadCursor::new(&original);
        let view = VisualSampleEntryView::parse_in(&mut cur).unwrap();
        let (owned, _) = VisualSampleEntry::try_parse_view(&view).unwrap();

        let mut encoded = vec![0u8; owned.encode_len()];
        let mut write_cur = crate::cursor::WriteCursor::new(&mut encoded);
        owned.write_in(&mut write_cur).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }
}
