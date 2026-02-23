//! Sample table statistics.
//!
//! Extracts per-track sample table statistics from `moov` and `moof` boxes,
//! using the mp4-bmff typed API.

use alloc::vec::Vec;
use core::fmt;

use mp4_bmff::boxes::bmff::{MoofBoxView, MoovBoxView, StblBoxView, TrafBoxView};
use mp4_bmff::error::Result;
use mp4_bmff::types::FourCC;

/// Per-track sample table statistics.
#[derive(Debug, Clone)]
pub struct SampleTableStats {
    /// Track ID from `tkhd`.
    pub track_id: u32,
    /// Handler type from `hdlr` (e.g. `vide`, `soun`).
    pub handler_type: FourCC,
    /// Media timescale from `mdhd` (time units per second).
    pub timescale: u32,

    /// Total number of samples.
    pub sample_count: u32,
    /// Total size of all samples in bytes.
    pub total_size: u64,
    /// Minimum sample size.
    pub min_sample_size: u32,
    /// Maximum sample size.
    pub max_sample_size: u32,
    /// Uniform sample size if all samples are the same size, otherwise `None`.
    pub uniform_sample_size: Option<u32>,

    /// Number of chunks.
    pub chunk_count: u32,

    /// Total duration in timescale units.
    pub duration_ticks: u64,
    /// Number of distinct delta values (stts entry count).
    pub distinct_deltas: u32,
    /// Minimum sample delta.
    pub min_delta: u32,
    /// Maximum sample delta.
    pub max_delta: u32,

    /// Number of sync samples (`None` if stss is absent, meaning all samples are sync).
    pub sync_sample_count: Option<u32>,

    /// Whether a ctts box is present.
    pub has_ctts: bool,
}

impl SampleTableStats {
    /// Duration in seconds as a float.
    pub fn duration_seconds(&self) -> f64 {
        if self.timescale == 0 {
            return 0.0;
        }
        #[allow(clippy::cast_precision_loss)]
        {
            self.duration_ticks as f64 / f64::from(self.timescale)
        }
    }

    /// Average sample size in bytes.
    pub fn avg_sample_size(&self) -> f64 {
        if self.sample_count == 0 {
            return 0.0;
        }
        #[allow(clippy::cast_precision_loss)]
        {
            self.total_size as f64 / f64::from(self.sample_count)
        }
    }
}

/// Analyzes sample tables for all tracks in a `moov` box.
///
/// # Errors
///
/// Returns an error if the moov box children are malformed.
pub fn analyze(moov: &MoovBoxView<'_>) -> Result<Vec<SampleTableStats>> {
    let mut results = Vec::new();

    for trak_result in moov.traks() {
        let trak = trak_result?;
        let tkhd = trak.tkhd()?;
        let mdia = trak.mdia()?;
        let mdhd = mdia.mdhd()?;
        let hdlr = mdia.hdlr()?;
        let stbl = mdia.minf()?.stbl()?;

        let stats = analyze_stbl(&stbl, tkhd.track_id, hdlr.handler_type, mdhd.timescale)?;
        results.push(stats);
    }

    Ok(results)
}

fn analyze_stbl(
    stbl: &StblBoxView<'_>,
    track_id: u32,
    handler_type: FourCC,
    timescale: u32,
) -> Result<SampleTableStats> {
    // Sample sizes
    let stsz = stbl.stsz()?;
    let (sample_count, total_size, min_sample_size, max_sample_size, uniform_sample_size) =
        if stsz.sample_size != 0 {
            // Uniform size
            let total = u64::from(stsz.sample_count) * u64::from(stsz.sample_size);
            (
                stsz.sample_count,
                total,
                stsz.sample_size,
                stsz.sample_size,
                Some(stsz.sample_size),
            )
        } else {
            let mut total: u64 = 0;
            let mut min = u32::MAX;
            let mut max = 0u32;
            for entry in stsz.entries() {
                let s = entry.entry_size;
                total += u64::from(s);
                if s < min {
                    min = s;
                }
                if s > max {
                    max = s;
                }
            }
            if stsz.sample_count == 0 {
                min = 0;
            }
            (stsz.sample_count, total, min, max, None)
        };

    // Chunk count
    let stco = stbl.stco()?;
    let chunk_count = stco.entry_count;

    // Timing (stts)
    let stts = stbl.stts()?;
    let distinct_deltas = stts.entry_count;
    let mut duration_ticks: u64 = 0;
    let mut min_delta = u32::MAX;
    let mut max_delta = 0u32;
    for entry in stts.entries() {
        duration_ticks += u64::from(entry.sample_count) * u64::from(entry.sample_delta);
        if entry.sample_delta < min_delta {
            min_delta = entry.sample_delta;
        }
        if entry.sample_delta > max_delta {
            max_delta = entry.sample_delta;
        }
    }
    if distinct_deltas == 0 {
        min_delta = 0;
    }

    // Sync samples (stss) - absence means all samples are sync
    let sync_sample_count = stbl.stss()?.map(|stss| stss.entry_count);

    // Composition time offsets
    let has_ctts = stbl.ctts()?.is_some();

    Ok(SampleTableStats {
        track_id,
        handler_type,
        timescale,
        sample_count,
        total_size,
        min_sample_size,
        max_sample_size,
        uniform_sample_size,
        chunk_count,
        duration_ticks,
        distinct_deltas,
        min_delta,
        max_delta,
        sync_sample_count,
        has_ctts,
    })
}

impl fmt::Display for SampleTableStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Track {} ({}):", self.track_id, self.handler_type)?;
        writeln!(f, "  timescale:    {}", self.timescale)?;
        writeln!(
            f,
            "  duration:     {} ticks ({:.3} s)",
            self.duration_ticks,
            self.duration_seconds()
        )?;
        writeln!(f, "  samples:      {}", self.sample_count)?;
        writeln!(f, "  chunks:       {}", self.chunk_count)?;

        // Sample sizes
        if let Some(uniform) = self.uniform_sample_size {
            writeln!(f, "  sample size:  {} (uniform)", uniform)?;
        } else {
            writeln!(
                f,
                "  sample size:  min={}, max={}, avg={:.1}",
                self.min_sample_size,
                self.max_sample_size,
                self.avg_sample_size()
            )?;
        }
        writeln!(f, "  total size:   {} bytes", self.total_size)?;

        // Timing
        writeln!(
            f,
            "  stts entries: {} (delta min={}, max={})",
            self.distinct_deltas, self.min_delta, self.max_delta
        )?;

        // Sync samples
        match self.sync_sample_count {
            Some(count) => writeln!(f, "  sync samples: {}", count)?,
            None => writeln!(f, "  sync samples: all")?,
        }

        // ctts
        if self.has_ctts {
            writeln!(f, "  ctts:         present")?;
        }

        Ok(())
    }
}

/// A resolved sample from a track fragment.
///
/// Each field is resolved by combining per-sample values from `trun`
/// with defaults from `tfhd`.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedSample {
    /// Sample duration in timescale units.
    pub duration: Option<u32>,
    /// Sample size in bytes.
    pub size: Option<u32>,
    /// Whether this sample is a sync (random access) sample.
    pub is_sync: bool,
    /// Composition time offset relative to decode time.
    pub composition_time_offset: Option<i32>,
}

/// Per-track fragment statistics from a `moof` box.
#[derive(Debug, Clone)]
pub struct FragmentStats {
    /// Track ID from `tfhd`.
    pub track_id: u32,
    /// Base media decode time from `tfdt` (if present).
    pub base_decode_time: Option<u64>,
    /// Number of track run (`trun`) boxes.
    pub trun_count: u32,

    /// Resolved samples collected from all `trun` boxes.
    pub samples: Vec<ResolvedSample>,

    /// Total size of all samples in bytes (`None` if size is unavailable).
    pub total_size: Option<u64>,
    /// Total duration in timescale units (`None` if duration is unavailable).
    pub duration_ticks: Option<u64>,
    /// Number of sync samples.
    pub sync_sample_count: u32,
}

/// Analyzes track fragments in a `moof` box.
///
/// # Errors
///
/// Returns an error if the moof box children are malformed.
pub fn analyze_moof(moof: &MoofBoxView<'_>) -> Result<Vec<FragmentStats>> {
    let mut results = Vec::new();

    for traf_result in moof.trafs() {
        let traf = traf_result?;
        let stats = analyze_traf(&traf)?;
        results.push(stats);
    }

    Ok(results)
}

fn analyze_traf(traf: &TrafBoxView<'_>) -> Result<FragmentStats> {
    let tfhd = traf.tfhd()?;
    let tfdt = traf.tfdt()?;

    let mut trun_count = 0u32;
    let mut samples = Vec::new();
    let mut total_size: Option<u64> = Some(0);
    let mut duration_ticks: Option<u64> = Some(0);
    let mut sync_sample_count = 0u32;

    for trun_result in traf.truns() {
        let trun = trun_result?;
        trun_count += 1;

        for (i, sample_result) in trun.samples().enumerate() {
            let sample = sample_result?;

            // Resolve size: per-sample or default from tfhd
            let size = sample.size.or(tfhd.default_sample_size);
            match size {
                Some(s) => {
                    if let Some(ref mut t) = total_size {
                        *t += u64::from(s);
                    }
                }
                None => total_size = None,
            }

            // Resolve duration: per-sample or default from tfhd
            let duration = sample.duration.or(tfhd.default_sample_duration);
            match duration {
                Some(d) => {
                    if let Some(ref mut t) = duration_ticks {
                        *t += u64::from(d);
                    }
                }
                None => duration_ticks = None,
            }

            // Resolve sync flag: per-sample → first_sample → tfhd default
            let resolved_flags = if i == 0 {
                sample.flags.or(trun.first_sample_flags)
            } else {
                sample.flags
            }
            .or(tfhd.default_sample_flags);
            let is_sync = resolved_flags.is_none_or(|f| f.is_sync());

            if is_sync {
                sync_sample_count += 1;
            }

            samples.push(ResolvedSample {
                duration,
                size,
                is_sync,
                composition_time_offset: sample.composition_time_offset,
            });
        }
    }

    Ok(FragmentStats {
        track_id: tfhd.track_id,
        base_decode_time: tfdt.map(|t| t.base_media_decode_time),
        trun_count,
        samples,
        total_size,
        duration_ticks,
        sync_sample_count,
    })
}

impl FragmentStats {
    /// Total number of samples.
    pub fn sample_count(&self) -> u32 {
        #[allow(clippy::cast_possible_truncation)]
        {
            self.samples.len() as u32
        }
    }
}

impl fmt::Display for FragmentStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sample_count = self.sample_count();
        writeln!(f, "Track {} (fragment):", self.track_id)?;

        if let Some(bdt) = self.base_decode_time {
            writeln!(f, "  base decode time: {bdt}")?;
        }

        writeln!(f, "  trun boxes:   {}", self.trun_count)?;
        writeln!(f, "  samples:      {sample_count}")?;

        // Size
        if let Some(total) = self.total_size {
            writeln!(f, "  total size:   {total} bytes")?;
        }

        // Duration
        if let Some(ticks) = self.duration_ticks {
            writeln!(f, "  duration:     {ticks} ticks")?;
        }

        // Sync
        if self.sync_sample_count == sample_count {
            writeln!(f, "  sync samples: all")?;
        } else {
            writeln!(f, "  sync samples: {}", self.sync_sample_count)?;
        }

        Ok(())
    }
}
