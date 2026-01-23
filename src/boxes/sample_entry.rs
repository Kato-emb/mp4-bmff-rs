use core::mem;

use crate::cursor::ReadCursor;
use crate::types::*;

use crate::error::*;

#[cfg(feature = "alloc")]
use crate::cursor::WriteCursor;

/// Fields common to all Sample Entry boxes.
#[derive(Debug, Clone, Copy)]
pub struct SampleEntry {
    /// The data reference index.
    pub data_reference_index: u16,
}

impl SampleEntry {
    const RESERVED_SIZE: usize = 6;

    /// Returns the size of the `SampleEntry` data.
    pub const fn size() -> usize {
        Self::RESERVED_SIZE + 2 // reserved + data_reference_index
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'_>) -> Result<Self> {
        // Skip reserved bytes
        cur.advance(Self::RESERVED_SIZE)?;
        let data_reference_index = cur.read_u16_be()?;

        Ok(SampleEntry {
            data_reference_index,
        })
    }

    #[cfg(feature = "alloc")]
    pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
        // Write reserved bytes (6 bytes of zeros)
        cur.write_slice(&[0u8; Self::RESERVED_SIZE])?;
        cur.write_u16_be(self.data_reference_index)?;
        Ok(())
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
    /// Compressor name field (32 bytes total: first byte is length, followed by 31 bytes of data).
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

    /// Returns the size of the `VisualSampleEntry` data.
    pub const fn size() -> usize {
        SampleEntry::size()
            + Self::PRE_DEFINED_SIZE_1
            + Self::RESERVED_SIZE_1
            + Self::PRE_DEFINED_SIZE_2
            + 2 // width
            + 2 // height
            + 4 // horizresolution
            + 4 // vertresolution
            + Self::RESERVED_SIZE_2
            + 2 // frame_count
            + 32 // compressorname
            + 2 // depth
            + Self::PRE_DEFINED_SIZE_3
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

        // compressorname is a 32-byte field: first byte is length, followed by 31 bytes of data
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
            compressorname,
            depth,
        })
    }

    #[cfg(feature = "alloc")]
    pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
        self.base.write_in(cur)?;

        // pre_defined (2 bytes)
        cur.write_slice(&[0u8; Self::PRE_DEFINED_SIZE_1])?;
        // reserved (2 bytes)
        cur.write_slice(&[0u8; Self::RESERVED_SIZE_1])?;
        // pre_defined (12 bytes)
        cur.write_slice(&[0u8; Self::PRE_DEFINED_SIZE_2])?;

        cur.write_u16_be(self.width)?;
        cur.write_u16_be(self.height)?;

        cur.write_u32_be(self.horizresolution.to_raw())?;
        cur.write_u32_be(self.vertresolution.to_raw())?;

        // reserved (4 bytes)
        cur.write_slice(&[0u8; Self::RESERVED_SIZE_2])?;

        cur.write_u16_be(self.frame_count)?;

        cur.write_array(&self.compressorname)?;

        cur.write_u16_be(self.depth)?;

        // pre_defined (2 bytes, -1)
        cur.write_slice(&[0xFF, 0xFF])?;

        Ok(())
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

    /// Returns the size of the `AudioSampleEntry` data.
    pub const fn size() -> usize {
        SampleEntry::size()
            + Self::RESERVED_SIZE_1
            + 2 // channelcount
            + 2 // samplesize
            + Self::PRE_DEFINED_SIZE
            + Self::RESERVED_SIZE_2
            + 4 // samplerate
    }

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

    #[cfg(feature = "alloc")]
    pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
        self.base.write_in(cur)?;

        // reserved (8 bytes)
        cur.write_slice(&[0u8; Self::RESERVED_SIZE_1])?;

        cur.write_u16_be(self.channelcount)?;
        cur.write_u16_be(self.samplesize)?;

        // pre_defined (2 bytes)
        cur.write_slice(&[0u8; Self::PRE_DEFINED_SIZE])?;
        // reserved (2 bytes)
        cur.write_slice(&[0u8; Self::RESERVED_SIZE_2])?;

        cur.write_u32_be(self.samplerate.to_raw())?;

        Ok(())
    }
}
