//! Sample Entry base types and common fields.
//!
//! This module provides base structures for sample entries that appear in
//! the Sample Description Box (`stsd`). Sample entries describe the format
//! of media samples and reference data sources.
//!
//! # Base Types
//!
//! - [`SampleEntry`]: Common fields for all sample entry types.
//! - `VisualSampleEntry`: Base for video sample entries (requires feature).
//! - `AudioSampleEntry`: Base for audio sample entries (requires feature).
//!
//! These base types are extended by codec-specific sample entries such as
//! `avc1`, `mp4a`, `mp4v`, etc.

use crate::error::*;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

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

    /// Parses a `SampleEntry` from the given byte slice.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);
        Self::parse_in(&mut cur)
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'_>) -> Result<Self> {
        // Skip reserved bytes
        cur.advance(Self::RESERVED)?;
        let data_reference_index = cur.read_u16_be()?;

        Ok(SampleEntry {
            data_reference_index,
        })
    }

    /// Writes the `SampleEntry` to the given byte slice.
    pub fn write(&self, bytes: &mut [u8]) -> Result<()> {
        let mut cur = WriteCursor::new(bytes);
        self.write_in(&mut cur)
    }

    pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
        // Write reserved bytes (6 bytes of zeros)
        cur.reserve_zeros(Self::RESERVED)?;
        cur.write_u16_be(self.data_reference_index)?;
        Ok(())
    }
}

#[cfg(any(feature = "mp4", feature = "avc", feature = "hevc"))]
mod visual {
    use super::*;
    use crate::types::*;

    /// Base structure for Visual Sample Entries (`avc1`, `mp4v`, `hvc1`, etc.).
    ///
    /// Contains common fields for all video sample entries as defined in
    /// ISO/IEC 14496-12. Codec-specific sample entries extend this base
    /// with additional configuration boxes.
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
    #[derive(Debug, Clone, Copy)]
    pub struct VisualSampleEntry {
        base: SampleEntry,
        /// Video width in pixels.
        pub width: u16,
        /// Video height in pixels.
        pub height: u16,
        /// Horizontal resolution in pixels per inch (16.16 fixed-point).
        /// Default is 72 dpi (0x00480000).
        pub horizresolution: U16F16,
        /// Vertical resolution in pixels per inch (16.16 fixed-point).
        /// Default is 72 dpi (0x00480000).
        pub vertresolution: U16F16,
        /// Number of frames per sample. Usually 1, but may be greater
        /// for samples containing multiple video frames.
        pub frame_count: u16,
        /// Compressor name field (32 bytes total: first byte is length,
        /// followed by up to 31 bytes of name data).
        compressorname: [u8; 32],
        /// Color depth in bits. 0x0018 (24) indicates color video without alpha.
        pub depth: u16,
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
                compressorname: [0; 32],
                depth: 0x0018,
            }
        }
    }

    impl VisualSampleEntry {
        const PRE_DEFINED_0: usize = 2;
        const RESERVED_0: usize = 2;
        const PRE_DEFINED_1: usize = 12;
        const RESERVED_1: usize = 4;
        const PRE_DEFINED_2: usize = 2;

        /// Returns the size of the `VisualSampleEntry` data.
        pub const fn size() -> usize {
            SampleEntry::size()
                + Self::PRE_DEFINED_0
                + Self::RESERVED_0
                + Self::PRE_DEFINED_1
                + 2 // width
                + 2 // height
                + 4 // horizresolution
                + 4 // vertresolution
                + Self::RESERVED_1
                + 2 // frame_count
                + 32 // compressorname
                + 2 // depth
                + Self::PRE_DEFINED_2
        }

        /// Returns a reference to the base `SampleEntry`.
        pub fn sample_entry(&self) -> &SampleEntry {
            &self.base
        }

        /// Returns the compressor name as a string slice, if valid UTF-8.
        ///
        /// The compressorname field is 32 bytes: first byte is length, followed by 31 bytes of data.
        pub fn compressorname(&self) -> Option<&str> {
            let len = (self.compressorname[0] as usize).min(31);
            let name_bytes = &self.compressorname[1..1 + len];
            core::str::from_utf8(name_bytes).ok()
        }

        /// Sets the compressor name. Truncates if longer than 31 bytes.
        ///
        /// The compressorname field is 32 bytes: first byte is length, followed by 31 bytes of data.
        pub fn set_compressorname(&mut self, name: &str) {
            let bytes = name.as_bytes();
            let len = bytes.len().min(31);
            self.compressorname[0] = len as u8;
            self.compressorname[1..1 + len].copy_from_slice(&bytes[..len]);
            // Clear remaining bytes
            for b in &mut self.compressorname[1 + len..] {
                *b = 0;
            }
        }

        pub(crate) fn parse_in(cur: &mut crate::cursor::ReadCursor<'_>) -> Result<Self> {
            let base = SampleEntry::parse_in(cur)?;

            // Skip pre_defined (2 bytes)
            cur.advance(Self::PRE_DEFINED_0)?;
            // Skip reserved (2 bytes)
            cur.advance(Self::RESERVED_0)?;
            // Skip pre_defined (12 bytes)
            cur.advance(Self::PRE_DEFINED_1)?;

            let width = cur.read_u16_be()?;
            let height = cur.read_u16_be()?;

            let horizresolution = U16F16::from_raw(cur.read_u32_be()?);
            let vertresolution = U16F16::from_raw(cur.read_u32_be()?);

            // Skip reserved (4 bytes)
            cur.advance(Self::RESERVED_1)?;

            let frame_count = cur.read_u16_be()?;

            // compressorname is a 32-byte field: first byte is length, followed by 31 bytes of data
            let compressorname = cur.read_array::<32>()?;

            let depth = cur.read_u16_be()?;

            // Skip pre_defined (2 bytes)
            cur.advance(Self::PRE_DEFINED_2)?;

            Ok(VisualSampleEntry {
                base,
                width,
                height,
                horizresolution,
                vertresolution,
                frame_count,
                compressorname,
                depth,
            })
        }

        #[cfg(feature = "alloc")]
        pub(crate) fn write_in(&self, cur: &mut crate::cursor::WriteCursor<'_>) -> Result<()> {
            self.base.write_in(cur)?;

            // pre_defined (2 bytes)
            cur.write_slice(&[0u8; Self::PRE_DEFINED_0])?;
            // reserved (2 bytes)
            cur.write_slice(&[0u8; Self::RESERVED_0])?;
            // pre_defined (12 bytes)
            cur.write_slice(&[0u8; Self::PRE_DEFINED_1])?;

            cur.write_u16_be(self.width)?;
            cur.write_u16_be(self.height)?;

            cur.write_u32_be(self.horizresolution.to_raw())?;
            cur.write_u32_be(self.vertresolution.to_raw())?;

            // reserved (4 bytes)
            cur.write_slice(&[0u8; Self::RESERVED_1])?;

            cur.write_u16_be(self.frame_count)?;

            cur.write_array(&self.compressorname)?;

            cur.write_u16_be(self.depth)?;

            // pre_defined (2 bytes, -1)
            cur.write_slice(&[0xFF, 0xFF])?;

            Ok(())
        }
    }
}

#[cfg(any(feature = "mp4", feature = "avc", feature = "hevc"))]
pub use visual::VisualSampleEntry;

#[cfg(feature = "mp4")]
mod audio {
    use super::*;
    use crate::types::*;

    /// Base structure for Audio Sample Entries (`mp4a`, etc.).
    ///
    /// Contains common fields for all audio sample entries as defined in
    /// ISO/IEC 14496-12. Codec-specific sample entries extend this base
    /// with additional configuration boxes (e.g., `esds` for AAC).
    ///
    /// # Structure
    ///
    /// - `base`: Common sample entry fields (data reference index).
    /// - `channelcount`: Number of audio channels (1=mono, 2=stereo, etc.).
    /// - `samplesize`: Bits per sample (typically 16).
    /// - `samplerate`: Audio sample rate in Hz (stored as 16.16 fixed-point).
    #[derive(Debug, Clone, Copy)]
    pub struct AudioSampleEntry {
        base: SampleEntry,
        /// Number of audio channels (1=mono, 2=stereo, 6=5.1, etc.).
        pub channelcount: u16,
        /// Bits per sample (typically 16 for PCM-like formats).
        pub samplesize: u16,
        /// Audio sample rate in Hz, stored as 16.16 fixed-point.
        /// For example, 44100 Hz is stored as 44100 << 16.
        pub samplerate: U16F16,
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
            }
        }
    }

    impl AudioSampleEntry {
        const RESERVED_0: usize = 8;
        const PRE_DEFINED: usize = 2;
        const RESERVED_1: usize = 2;

        /// Returns the size of the `AudioSampleEntry` data.
        pub const fn size() -> usize {
            SampleEntry::size()
                + Self::RESERVED_0
                + 2 // channelcount
                + 2 // samplesize
                + Self::PRE_DEFINED
                + Self::RESERVED_1
                + 4 // samplerate
        }

        /// Returns a reference to the base `SampleEntry`.
        pub fn sample_entry(&self) -> &SampleEntry {
            &self.base
        }

        pub(crate) fn parse_in(cur: &mut crate::cursor::ReadCursor<'_>) -> Result<Self> {
            let base = SampleEntry::parse_in(cur)?;

            // Skip reserved (4 bytes)
            cur.advance(Self::RESERVED_0)?;

            let channelcount = cur.read_u16_be()?;
            let samplesize = cur.read_u16_be()?;

            // Skip pre_defined (2 bytes)
            cur.advance(Self::PRE_DEFINED)?;
            // Skip reserved (2 bytes)
            cur.advance(Self::RESERVED_1)?;

            let samplerate = U16F16::from_raw(cur.read_u32_be()?);

            Ok(AudioSampleEntry {
                base,
                channelcount,
                samplesize,
                samplerate,
            })
        }

        #[cfg(feature = "alloc")]
        pub(crate) fn write_in(&self, cur: &mut crate::cursor::WriteCursor<'_>) -> Result<()> {
            self.base.write_in(cur)?;

            // reserved (8 bytes)
            cur.write_slice(&[0u8; Self::RESERVED_0])?;

            cur.write_u16_be(self.channelcount)?;
            cur.write_u16_be(self.samplesize)?;

            // pre_defined (2 bytes)
            cur.write_slice(&[0u8; Self::PRE_DEFINED])?;
            // reserved (2 bytes)
            cur.write_slice(&[0u8; Self::RESERVED_1])?;

            cur.write_u32_be(self.samplerate.to_raw())?;

            Ok(())
        }
    }
}

#[cfg(feature = "mp4")]
pub use audio::AudioSampleEntry;
