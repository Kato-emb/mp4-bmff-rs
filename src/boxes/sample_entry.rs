use core::mem;

use crate::cursor::ReadCursor;
use crate::types::*;

use crate::error::*;

/// Fields common to all Sample Entry boxes.
#[derive(Debug, Clone, Copy)]
pub struct SampleEntry {
    /// The data reference index.
    pub data_reference_index: u16,
}

impl SampleEntry {
    const RESERVED_SIZE: usize = 6;

    pub(crate) fn parse_in(cur: &mut ReadCursor<'_>) -> Result<Self> {
        // Skip reserved bytes
        cur.advance(Self::RESERVED_SIZE)?;
        let data_reference_index = cur.read_u16_be()?;

        Ok(SampleEntry {
            data_reference_index,
        })
    }
}

/// Visual Sample Entry box (`avc1`, `mp4v`, etc.).
#[derive(Debug, Clone, Copy)]
pub struct VisualSampleEntry {
    base: SampleEntry,
    /// The width of the video in pixels.
    pub width: u16,
    /// The height of the video in pixels.
    pub height: u16,
    /// The horizontal resolution.
    pub horizresolution: U16F16,
    /// The vertical resolution.
    pub vertresolution: U16F16,
    /// The number of frames.
    pub frame_count: u16,
    compressorname_len: u8,
    compressorname: [u8; 32],
    /// The color depth.
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
            compressorname_len: 0,
            compressorname: [0; 32],
            depth: 0x0018,
        }
    }
}

impl VisualSampleEntry {
    const PRE_DEFINED_SIZE_1: usize = mem::size_of::<u16>();
    const RESERVED_SIZE_1: usize = mem::size_of::<u16>();
    const PRE_DEFINED_SIZE_2: usize = mem::size_of::<[u32; 3]>();
    const RESERVED_SIZE_2: usize = mem::size_of::<u32>();
    const PRE_DEFINED_SIZE_3: usize = mem::size_of::<i16>();

    /// Returns a reference to the base `SampleEntry`.
    pub fn sample_entry(&self) -> &SampleEntry {
        &self.base
    }

    /// Returns the compressor name as a string slice, if valid UTF-8.
    pub fn compressorname(&self) -> Option<&str> {
        let len = self.compressorname_len as usize;
        let name_bytes = &self.compressorname[..len];
        core::str::from_utf8(name_bytes).ok()
    }

    /// Sets the compressor name. Truncates if longer than 32 bytes.
    pub fn set_compressorname(&mut self, name: &str) {
        let bytes = name.as_bytes();
        let len = bytes.len().min(32);
        self.compressorname_len = len as u8;
        self.compressorname[..len].copy_from_slice(&bytes[..len]);
        if len < 32 {
            for b in &mut self.compressorname[len..] {
                *b = 0;
            }
        }
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'_>) -> Result<Self> {
        let base = SampleEntry::parse_in(cur)?;

        // Skip pre_defined (2 bytes)
        cur.advance(Self::PRE_DEFINED_SIZE_1)?;
        // Skip reserved (2 bytes)
        cur.advance(Self::RESERVED_SIZE_1)?;
        // Skip pre_defined (12 bytes)
        cur.advance(Self::PRE_DEFINED_SIZE_2)?;

        let width = cur.read_u16_be()?;
        let height = cur.read_u16_be()?;

        let horizresolution = U16F16::from_raw(cur.read_u32_be()?);
        let vertresolution = U16F16::from_raw(cur.read_u32_be()?);

        // Skip reserved (4 bytes)
        cur.advance(Self::RESERVED_SIZE_2)?;

        let frame_count = cur.read_u16_be()?;

        let compressorname_len = cur.read_u8()?;
        let compressorname = cur.read_array::<32>()?;

        let depth = cur.read_u16_be()?;

        // Skip pre_defined (2 bytes)
        cur.advance(Self::PRE_DEFINED_SIZE_3)?;

        Ok(VisualSampleEntry {
            base,
            width,
            height,
            horizresolution,
            vertresolution,
            frame_count,
            compressorname_len,
            compressorname,
            depth,
        })
    }
}

/// Audio Sample Entry box (`mp4a`, etc.).
#[derive(Debug, Clone, Copy)]
pub struct AudioSampleEntry {
    base: SampleEntry,
    /// The number of audio channels.
    pub channelcount: u16,
    /// The number of bits per sample.
    pub samplesize: u16,
    /// The sample rate.
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
    const RESERVED_SIZE_1: usize = mem::size_of::<[u32; 2]>();
    const PRE_DEFINED_SIZE: usize = mem::size_of::<u16>();
    const RESERVED_SIZE_2: usize = mem::size_of::<u16>();

    /// Returns a reference to the base `SampleEntry`.
    pub fn sample_entry(&self) -> &SampleEntry {
        &self.base
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'_>) -> Result<Self> {
        let base = SampleEntry::parse_in(cur)?;

        // Skip reserved (4 bytes)
        cur.advance(Self::RESERVED_SIZE_1)?;

        let channelcount = cur.read_u16_be()?;
        let samplesize = cur.read_u16_be()?;

        // Skip pre_defined (2 bytes)
        cur.advance(Self::PRE_DEFINED_SIZE)?;
        // Skip reserved (2 bytes)
        cur.advance(Self::RESERVED_SIZE_2)?;

        let samplerate = U16F16::from_raw(cur.read_u32_be()?);

        Ok(AudioSampleEntry {
            base,
            channelcount,
            samplesize,
            samplerate,
        })
    }
}
