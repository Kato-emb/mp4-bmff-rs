//! fMP4 → non-fragmented MP4 assembler.
//!
//! Reads an `init` segment plus a sorted list of media fragments from a
//! directory, concatenates the fragment bytes into a single MP4 file, and
//! finalizes a non-fragmented `moov` that points into the concatenated mdat.
//!
//! Usage:
//!
//! ```text
//! fmp4-asm <input_dir> <output.mp4>
//! ```
//!
//! The input directory must contain exactly one initialization file whose
//! extension matches an init segment (`mp4`, `cmfi`, `m4i`). All other regular
//! files whose extensions look like fMP4 fragments (`m4s`, `m4v`, `m4a`,
//! `cmfv`, `cmfa`, `cmft`) are treated as media fragments and processed in
//! lexicographic filename order.
//!
//! # I/O strategy
//!
//! Fragments are processed in a streaming manner: top-level boxes are read
//! one at a time from each fragment file, and only `moof` boxes are buffered
//! in memory (so they can be parsed). All other boxes — most notably `mdat`
//! payloads, which are typically the bulk of a fragment — are stream-copied
//! to the output file using a fixed-size buffer. Peak memory usage per
//! fragment is therefore bounded by the largest single `moof`, not the
//! fragment file size.
//!
//! After every fragment is appended, the placeholder reserved at the start
//! of the output is rewritten as an `mdat` largesize header that swallows
//! all of the appended bytes; the `moov` composed from accumulated tables
//! follows.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use mp4_bmff::boxes::bmff::{MoofBoxView, MoovBoxView, TrexBox};
use mp4_bmff::{BoxDecode, BoxHeader, BoxType, iter_boxes};

use mp4_bmff_util::io::BoxWriter;
use mp4_bmff_util::multiplex::container::{ChunkLayout, SampleSpec, TrackId};
use mp4_bmff_util::multiplex::{compose_moov, decompose_moof, decompose_moov};

const INIT_EXTS: &[&str] = &["mp4", "cmfi", "m4i"];
const FRAGMENT_EXTS: &[&str] = &["m4s", "m4v", "m4a", "cmfv", "cmfa", "cmft"];
/// Size of an extended-header (`largesize`) BMFF box header: 4-byte size
/// marker (= 1) + 4-byte type + 8-byte 64-bit size field.
const LARGESIZE_HEADER_LEN: usize = 16;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let input_dir = match args.next() {
        Some(p) => PathBuf::from(p),
        None => {
            eprintln!("Usage: fmp4-asm <input_dir> <output.mp4>");
            return ExitCode::from(1);
        }
    };
    let output_path = match args.next() {
        Some(p) => PathBuf::from(p),
        None => {
            eprintln!("Usage: fmp4-asm <input_dir> <output.mp4>");
            return ExitCode::from(1);
        }
    };

    match run(&input_dir, &output_path) {
        Ok(stats) => {
            println!(
                "Wrote {} bytes to {} ({} fragment(s), {} track(s))",
                stats.output_size,
                output_path.display(),
                stats.fragments,
                stats.tracks,
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

struct Stats {
    output_size: u64,
    fragments: usize,
    tracks: usize,
}

fn run(input_dir: &Path, output_path: &Path) -> Result<Stats, String> {
    let (init_path, fragment_paths) = collect_inputs(input_dir)?;
    println!("init     : {}", init_path.display());
    for f in &fragment_paths {
        println!("fragment : {}", f.display());
    }

    let init_bytes = fs::read(&init_path).map_err(|e| format!("read {init_path:?}: {e}"))?;

    let file = fs::File::create(output_path).map_err(|e| format!("create {output_path:?}: {e}"))?;
    let mut output = BufWriter::new(file);

    // 1) Copy ftyp from init, build Movie template + collect trex defaults.
    let (mut movie, trexs) = process_init(&init_bytes, &mut output)?;

    // 2) Reserve a 16-byte largesize `free` placeholder. After all fragments
    //    are written, this slot is overwritten with an `mdat` largesize
    //    header whose total_size covers everything we appended.
    let free_pos = stream_position(&mut output)?;
    write_placeholder_free(&mut output)?;

    // 3) Stream-process each fragment file: only `moof` boxes are buffered;
    //    other boxes (especially `mdat`) are copied with a fixed-size buffer.
    let mut accum: HashMap<TrackId, (SampleSpec, ChunkLayout)> = HashMap::new();
    for path in &fragment_paths {
        process_fragment_file(path, &mut output, &trexs, &mut accum)
            .map_err(|e| format!("fragment {}: {e}", path.display()))?;
    }

    // 4) Apply accumulated tables back onto the Movie's tracks.
    let track_count = apply_accumulated_tables(&mut movie, accum)?;

    // 5) Compose final non-fragmented moov and append it via util's BoxWriter
    //    (writes header + payload directly to the underlying stream without
    //    materializing a single contiguous buffer).
    let moov = compose_moov(&movie).map_err(|e| format!("compose_moov: {e}"))?;
    let moov_pos = stream_position(&mut output)?;
    let mut box_writer = BoxWriter::new(&mut output);
    box_writer
        .write_box(&moov)
        .map_err(|e| format!("write moov: {e}"))?;

    // 6) Rewrite the placeholder as an `mdat` largesize header that covers
    //    every byte we appended between `free_pos` and `moov_pos`. Sample
    //    chunk_offsets emitted by `decompose_moof` already point inside this
    //    region, so the resulting movie is valid even though the original
    //    moof / mdat headers remain in place inside this big mdat.
    finalize_mdat_header(&mut output, free_pos, moov_pos)?;

    let output_size = stream_position(&mut output)?;
    output.flush().map_err(|e| format!("flush: {e}"))?;
    Ok(Stats {
        output_size,
        fragments: fragment_paths.len(),
        tracks: track_count,
    })
}

/// Walks `dir`, returns the init segment path plus a sorted list of fragments.
fn collect_inputs(dir: &Path) -> Result<(PathBuf, Vec<PathBuf>), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("read_dir {dir:?}: {e}"))?;

    let mut init: Option<PathBuf> = None;
    let mut fragments: Vec<PathBuf> = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| format!("dir entry: {e}"))?;
        let path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|e| format!("metadata {path:?}: {e}"))?;
        if !metadata.is_file() {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        if INIT_EXTS.iter().any(|e| *e == ext) {
            if let Some(prev) = &init {
                return Err(format!(
                    "multiple init segments found: {} and {}",
                    prev.display(),
                    path.display()
                ));
            }
            init = Some(path);
        } else if FRAGMENT_EXTS.iter().any(|e| *e == ext) {
            fragments.push(path);
        }
    }

    let init = init.ok_or_else(|| {
        format!("no init segment found in {dir:?} (looked for extensions: {INIT_EXTS:?})")
    })?;
    if fragments.is_empty() {
        return Err(format!("no fragment segments found in {dir:?}"));
    }

    fragments.sort();
    Ok((init, fragments))
}

/// Parses the init segment, copies its `ftyp` to the output, and returns the
/// Movie template plus the `trex` defaults discovered inside its `mvex`.
fn process_init<W: Write>(
    bytes: &[u8],
    output: &mut W,
) -> Result<(mp4_bmff_util::multiplex::container::Movie, Vec<TrexBox>), String> {
    let mut ftyp_slice: Option<&[u8]> = None;
    let mut moov_payload: Option<&[u8]> = None;

    let mut cursor = 0usize;
    for raw in iter_boxes(bytes) {
        let raw = raw.map_err(|e| format!("init parse: {e}"))?;
        let len = raw.len();
        let total = &bytes[cursor..cursor + len];

        match raw.boxtype() {
            BoxType::FTYP => ftyp_slice = Some(total),
            BoxType::MOOV => moov_payload = Some(raw.into_payload()),
            _ => {}
        }
        cursor += len;
    }

    let ftyp = ftyp_slice.ok_or_else(|| "init: ftyp missing".to_string())?;
    let moov_payload = moov_payload.ok_or_else(|| "init: moov missing".to_string())?;

    output
        .write_all(ftyp)
        .map_err(|e| format!("write ftyp: {e}"))?;

    let moov_view =
        MoovBoxView::decode(moov_payload).map_err(|e| format!("init moov decode: {e}"))?;

    let mut trexs = Vec::new();
    if let Some(mvex) = moov_view.mvex().map_err(|e| format!("init mvex: {e}"))? {
        for trex in mvex.trexs() {
            trexs.push(trex.map_err(|e| format!("init trex: {e}"))?);
        }
    }

    let movie = decompose_moov(&moov_view).map_err(|e| format!("decompose_moov: {e}"))?;

    Ok((movie, trexs))
}

/// Streams a single fragment file: reads its top-level boxes one at a time,
/// buffering only `moof` boxes (which need to be parsed), and stream-copying
/// every other box (notably `mdat`) directly to the output.
fn process_fragment_file<W: Write + Seek>(
    path: &Path,
    output: &mut W,
    trexs: &[TrexBox],
    accum: &mut HashMap<TrackId, (SampleSpec, ChunkLayout)>,
) -> Result<(), String> {
    let file = fs::File::open(path).map_err(|e| format!("open: {e}"))?;
    let mut reader = BufReader::new(file);

    loop {
        // Peek base header (8 bytes). `Read::read` is allowed to return less,
        // so use read_exact-with-eof to detect end-of-file cleanly.
        let mut base = [0u8; 8];
        match read_exact_or_eof(&mut reader, &mut base).map_err(|e| format!("read header: {e}"))? {
            ReadResult::Eof => break,
            ReadResult::Partial => {
                return Err("trailing bytes after last box (truncated header)".to_string());
            }
            ReadResult::Ok => {}
        }

        // Determine if the header has a largesize and/or uuid suffix.
        let extra = BoxHeader::additional_header_bytes(&base);
        let mut header_buf = [0u8; BoxHeader::MAX_HEADER_SIZE];
        header_buf[..8].copy_from_slice(&base);
        if extra > 0 {
            reader
                .read_exact(&mut header_buf[8..8 + extra])
                .map_err(|e| format!("read header tail: {e}"))?;
        }
        let header_len = 8 + extra;

        let header = BoxHeader::parse(&header_buf[..header_len])
            .map_err(|e| format!("parse header: {e}"))?;
        let total_size = header
            .boxsize()
            .value()
            .ok_or_else(|| "to-EOF box not supported in fragments".to_string())?;
        let payload_len = total_size
            .checked_sub(header_len as u64)
            .ok_or_else(|| "header_len > total_size".to_string())?;

        // Position in the OUTPUT where this box starts.
        let box_pos_in_output = stream_position(output).map_err(|e| e.to_string())?;

        // Always write the header through.
        output
            .write_all(&header_buf[..header_len])
            .map_err(|e| format!("write header: {e}"))?;

        if header.boxtype() == BoxType::MOOF {
            // Buffer the moof payload so we can parse it, then write it.
            let mut moof_payload = vec![0u8; payload_len as usize];
            reader
                .read_exact(&mut moof_payload)
                .map_err(|e| format!("read moof: {e}"))?;
            output
                .write_all(&moof_payload)
                .map_err(|e| format!("write moof: {e}"))?;

            let moof =
                MoofBoxView::decode(&moof_payload).map_err(|e| format!("moof decode: {e}"))?;
            let entries = decompose_moof(&moof, box_pos_in_output, trexs)
                .map_err(|e| format!("decompose_moof: {e}"))?;

            for (id, spec, layout) in entries {
                let entry = accum
                    .entry(id)
                    .or_insert_with(|| (SampleSpec::default(), ChunkLayout::default()));
                entry.0 = entry.0.concat(&spec);
                entry.1 = entry.1.concat(&layout);
            }
        } else {
            // Other boxes (mdat, styp, free, ...) are stream-copied without
            // ever being held in memory beyond the copy buffer.
            stream_copy(&mut reader, output, payload_len)
                .map_err(|e| format!("copy {}: {e}", header.boxtype()))?;
        }
    }

    Ok(())
}

fn apply_accumulated_tables(
    movie: &mut mp4_bmff_util::multiplex::container::Movie,
    mut accum: HashMap<TrackId, (SampleSpec, ChunkLayout)>,
) -> Result<usize, String> {
    let mut applied = 0usize;
    for track in movie.tracks_mut() {
        let id = track.track_id();
        let (spec, layout) = accum
            .remove(&id)
            .ok_or_else(|| format!("no fragment data for track {}", id.as_u32()))?;
        track.set_sample_spec(spec);
        track
            .set_chunk_layout(layout)
            .map_err(|e| format!("set_chunk_layout for track {}: {e}", id.as_u32()))?;
        applied += 1;
    }
    if !accum.is_empty() {
        let extras: Vec<u32> = accum.keys().map(|id| id.as_u32()).collect();
        return Err(format!(
            "fragments referenced unknown track IDs: {extras:?}"
        ));
    }
    Ok(applied)
}

/// Writes a fixed 16-byte `free` box using the largesize encoding so that it
/// can be replaced in place by an `mdat` largesize header (also 16 bytes)
/// regardless of the eventual mdat payload size.
fn write_placeholder_free<W: Write>(output: &mut W) -> Result<(), String> {
    let header = BoxHeader::new_extended(BoxType::FREE, 0);
    debug_assert_eq!(header.header_len(), LARGESIZE_HEADER_LEN);
    let mut buf = [0u8; LARGESIZE_HEADER_LEN];
    header
        .write(&mut buf)
        .map_err(|e| format!("placeholder header: {e}"))?;
    output
        .write_all(&buf)
        .map_err(|e| format!("write placeholder: {e}"))
}

fn finalize_mdat_header<W: Write + Seek>(
    output: &mut W,
    free_pos: u64,
    moov_pos: u64,
) -> Result<(), String> {
    let span = moov_pos
        .checked_sub(free_pos)
        .ok_or_else(|| "free_pos > moov_pos".to_string())?;
    let payload_len = span
        .checked_sub(LARGESIZE_HEADER_LEN as u64)
        .ok_or_else(|| format!("mdat span {span} < {LARGESIZE_HEADER_LEN} byte header"))?;

    let header = BoxHeader::new_extended(BoxType::MDAT, payload_len);
    debug_assert_eq!(header.header_len(), LARGESIZE_HEADER_LEN);
    let mut buf = [0u8; LARGESIZE_HEADER_LEN];
    header
        .write(&mut buf)
        .map_err(|e| format!("mdat header encode: {e}"))?;

    output
        .seek(SeekFrom::Start(free_pos))
        .map_err(|e| format!("seek to free_pos: {e}"))?;
    output
        .write_all(&buf)
        .map_err(|e| format!("rewrite mdat header: {e}"))?;
    output
        .seek(SeekFrom::End(0))
        .map_err(|e| format!("seek to end: {e}"))?;
    Ok(())
}

/// Copies exactly `len` bytes from `reader` to `writer` using a fixed buffer.
/// Avoids allocating a buffer that scales with `len`.
fn stream_copy<R: Read, W: Write>(reader: &mut R, writer: &mut W, len: u64) -> io::Result<()> {
    let mut taken = reader.take(len);
    let copied = io::copy(&mut taken, writer)?;
    if copied != len {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            format!("expected {len} bytes, copied {copied}"),
        ));
    }
    Ok(())
}

enum ReadResult {
    Ok,
    Partial,
    Eof,
}

/// `read_exact` that distinguishes clean EOF (nothing read) from partial
/// reads (some but not all bytes read).
fn read_exact_or_eof<R: Read>(reader: &mut R, buf: &mut [u8]) -> io::Result<ReadResult> {
    let mut filled = 0;
    while filled < buf.len() {
        match reader.read(&mut buf[filled..])? {
            0 => {
                if filled == 0 {
                    return Ok(ReadResult::Eof);
                } else {
                    return Ok(ReadResult::Partial);
                }
            }
            n => filled += n,
        }
    }
    Ok(ReadResult::Ok)
}

fn stream_position<W: Seek>(output: &mut W) -> Result<u64, String> {
    output
        .stream_position()
        .map_err(|e| format!("stream_position: {e}"))
}
