//! Benchmark tests for mp4-bmff codec operations.

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use mp4_bmff::boxes::bmff::{
    FtypBox, FtypBoxView, MoovBox, MoovBoxView, SttsBox, SttsBoxView, SttsEntry, SttsFlags,
};
use mp4_bmff::types::FourCC;
use mp4_bmff::{BoxDecode, BoxEncode, BoxHeader, RawBoxRef};

// =============================================================================
// Test Data Helpers
// =============================================================================

fn make_box(boxtype: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let size = (8 + payload.len()) as u32;
    let mut data = Vec::new();
    data.extend_from_slice(&size.to_be_bytes());
    data.extend_from_slice(boxtype);
    data.extend_from_slice(payload);
    data
}

fn make_ftyp_payload(num_brands: usize) -> Vec<u8> {
    let mut data = Vec::new();
    // major_brand
    data.extend_from_slice(b"isom");
    // minor_version
    data.extend_from_slice(&512u32.to_be_bytes());
    // compatible_brands
    for i in 0..num_brands {
        let brand = format!("br{:02}", i % 100);
        data.extend_from_slice(brand.as_bytes());
    }
    data
}

fn make_stts_payload(num_entries: usize) -> Vec<u8> {
    let mut data = Vec::new();
    // version + flags
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    // entry_count
    data.extend_from_slice(&(num_entries as u32).to_be_bytes());
    // entries
    for i in 0..num_entries {
        data.extend_from_slice(&((i as u32) + 1).to_be_bytes()); // sample_count
        data.extend_from_slice(&1000u32.to_be_bytes()); // sample_delta
    }
    data
}

fn make_mvhd_payload() -> Vec<u8> {
    let mut data = Vec::new();
    // FullBoxHeader: version=0, flags=0
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    // creation_time, modification_time
    data.extend_from_slice(&0u32.to_be_bytes());
    data.extend_from_slice(&0u32.to_be_bytes());
    // timescale
    data.extend_from_slice(&1000u32.to_be_bytes());
    // duration
    data.extend_from_slice(&5000u32.to_be_bytes());
    // rate (1.0)
    data.extend_from_slice(&0x00010000i32.to_be_bytes());
    // volume (1.0)
    data.extend_from_slice(&0x0100u16.to_be_bytes());
    // reserved (10 bytes)
    data.extend_from_slice(&[0u8; 10]);
    // matrix (identity)
    data.extend_from_slice(&0x00010000i32.to_be_bytes());
    data.extend_from_slice(&0i32.to_be_bytes());
    data.extend_from_slice(&0i32.to_be_bytes());
    data.extend_from_slice(&0i32.to_be_bytes());
    data.extend_from_slice(&0x00010000i32.to_be_bytes());
    data.extend_from_slice(&0i32.to_be_bytes());
    data.extend_from_slice(&0i32.to_be_bytes());
    data.extend_from_slice(&0i32.to_be_bytes());
    data.extend_from_slice(&0x40000000i32.to_be_bytes());
    // pre_defined (24 bytes)
    data.extend_from_slice(&[0u8; 24]);
    // next_track_id
    data.extend_from_slice(&2u32.to_be_bytes());
    data
}

fn make_tkhd_payload(track_id: u32) -> Vec<u8> {
    let mut data = Vec::new();
    data.push(0);
    data.extend_from_slice(&[0, 0, 3]);
    data.extend_from_slice(&0u32.to_be_bytes());
    data.extend_from_slice(&0u32.to_be_bytes());
    data.extend_from_slice(&track_id.to_be_bytes());
    data.extend_from_slice(&0u32.to_be_bytes());
    data.extend_from_slice(&1000u32.to_be_bytes());
    data.extend_from_slice(&[0u8; 8]);
    data.extend_from_slice(&[0u8; 4]);
    data.extend_from_slice(&[0x01, 0x00]);
    data.extend_from_slice(&[0u8; 2]);
    data.extend_from_slice(&0x00010000i32.to_be_bytes());
    data.extend_from_slice(&0i32.to_be_bytes());
    data.extend_from_slice(&0i32.to_be_bytes());
    data.extend_from_slice(&0i32.to_be_bytes());
    data.extend_from_slice(&0x00010000i32.to_be_bytes());
    data.extend_from_slice(&0i32.to_be_bytes());
    data.extend_from_slice(&0i32.to_be_bytes());
    data.extend_from_slice(&0i32.to_be_bytes());
    data.extend_from_slice(&0x40000000i32.to_be_bytes());
    data.extend_from_slice(&(1920u32 << 16).to_be_bytes());
    data.extend_from_slice(&(1080u32 << 16).to_be_bytes());
    data
}

fn make_mdhd_payload() -> Vec<u8> {
    let mut data = Vec::new();
    data.push(0);
    data.extend_from_slice(&[0, 0, 0]);
    data.extend_from_slice(&0u32.to_be_bytes());
    data.extend_from_slice(&0u32.to_be_bytes());
    data.extend_from_slice(&1000u32.to_be_bytes());
    data.extend_from_slice(&0u32.to_be_bytes());
    data.extend_from_slice(&0x55C4u16.to_be_bytes()); // 'und'
    data.extend_from_slice(&[0, 0]);
    data
}

fn make_hdlr_payload() -> Vec<u8> {
    let mut data = Vec::new();
    data.push(0);
    data.extend_from_slice(&[0, 0, 0]);
    data.extend_from_slice(&[0, 0, 0, 0]);
    data.extend_from_slice(b"vide");
    data.extend_from_slice(&[0u8; 12]);
    data.push(0);
    data
}

fn make_vmhd_payload() -> Vec<u8> {
    let mut data = Vec::new();
    data.push(0);
    data.extend_from_slice(&[0, 0, 1]);
    data.extend_from_slice(&[0, 0]);
    data.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
    data
}

fn make_url_payload() -> Vec<u8> {
    let mut data = Vec::new();
    data.push(0);
    data.extend_from_slice(&[0, 0, 1]);
    data
}

fn make_dref_payload() -> Vec<u8> {
    let mut data = Vec::new();
    data.push(0);
    data.extend_from_slice(&[0, 0, 0]);
    data.extend_from_slice(&1u32.to_be_bytes());
    data.extend_from_slice(&make_box(b"url ", &make_url_payload()));
    data
}

fn make_dinf_payload() -> Vec<u8> {
    make_box(b"dref", &make_dref_payload())
}

fn make_stbl_payload() -> Vec<u8> {
    let mut data = Vec::new();

    let mut stsd = Vec::new();
    stsd.push(0);
    stsd.extend_from_slice(&[0, 0, 0]);
    stsd.extend_from_slice(&0u32.to_be_bytes());
    data.extend_from_slice(&make_box(b"stsd", &stsd));

    let mut stts = Vec::new();
    stts.push(0);
    stts.extend_from_slice(&[0, 0, 0]);
    stts.extend_from_slice(&0u32.to_be_bytes());
    data.extend_from_slice(&make_box(b"stts", &stts));

    let mut stsc = Vec::new();
    stsc.push(0);
    stsc.extend_from_slice(&[0, 0, 0]);
    stsc.extend_from_slice(&0u32.to_be_bytes());
    data.extend_from_slice(&make_box(b"stsc", &stsc));

    let mut stco = Vec::new();
    stco.push(0);
    stco.extend_from_slice(&[0, 0, 0]);
    stco.extend_from_slice(&0u32.to_be_bytes());
    data.extend_from_slice(&make_box(b"stco", &stco));

    data
}

fn make_minf_payload() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&make_box(b"vmhd", &make_vmhd_payload()));
    data.extend_from_slice(&make_box(b"dinf", &make_dinf_payload()));
    data.extend_from_slice(&make_box(b"stbl", &make_stbl_payload()));
    data
}

fn make_mdia_payload() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&make_box(b"mdhd", &make_mdhd_payload()));
    data.extend_from_slice(&make_box(b"hdlr", &make_hdlr_payload()));
    data.extend_from_slice(&make_box(b"minf", &make_minf_payload()));
    data
}

fn make_trak_payload(track_id: u32) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&make_box(b"tkhd", &make_tkhd_payload(track_id)));
    data.extend_from_slice(&make_box(b"mdia", &make_mdia_payload()));
    data
}

fn make_moov_payload(num_tracks: u32) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&make_box(b"mvhd", &make_mvhd_payload()));
    for i in 1..=num_tracks {
        data.extend_from_slice(&make_box(b"trak", &make_trak_payload(i)));
    }
    data
}

// =============================================================================
// Benchmarks
// =============================================================================

/// Benchmark: BoxHeader parse
fn bench_header_parse(c: &mut Criterion) {
    let data = [
        0x00, 0x00, 0x00, 0x0C, // size = 12
        b'f', b't', b'y', b'p', // type = 'ftyp'
    ];

    c.bench_function("header_parse", |b| {
        b.iter(|| BoxHeader::parse(black_box(&data)))
    });
}

/// Benchmark: RawBoxRef parse
fn bench_rawbox_parse(c: &mut Criterion) {
    let payload = make_ftyp_payload(10);
    let data = make_box(b"ftyp", &payload);

    c.bench_function("rawbox_parse", |b| {
        b.iter(|| RawBoxRef::parse(black_box(&data)))
    });
}

/// Benchmark: View vs Owned decode comparison (ftyp)
fn bench_ftyp_view_vs_owned(c: &mut Criterion) {
    let mut group = c.benchmark_group("ftyp_decode");

    for num_brands in [4, 16, 64] {
        let payload = make_ftyp_payload(num_brands);

        group.bench_with_input(
            BenchmarkId::new("view", num_brands),
            &payload,
            |b, payload| b.iter(|| FtypBoxView::decode(black_box(payload))),
        );

        group.bench_with_input(
            BenchmarkId::new("owned", num_brands),
            &payload,
            |b, payload| b.iter(|| FtypBox::decode(black_box(payload))),
        );
    }

    group.finish();
}

/// Benchmark: FtypBox encode
fn bench_ftyp_encode(c: &mut Criterion) {
    let ftyp = FtypBox {
        major_brand: FourCC::new(*b"isom"),
        minor_version: 512,
        compatible_brands: vec![
            FourCC::new(*b"isom"),
            FourCC::new(*b"iso2"),
            FourCC::new(*b"avc1"),
            FourCC::new(*b"mp41"),
        ],
    };

    let mut buf = vec![0u8; ftyp.encoded_len()];

    c.bench_function("ftyp_encode", |b| {
        b.iter(|| ftyp.encode(black_box(&mut buf)))
    });
}

/// Benchmark: stts decode with varying entry counts
fn bench_stts_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("stts_decode");

    for num_entries in [100, 1000, 10000] {
        let payload = make_stts_payload(num_entries);

        group.bench_with_input(
            BenchmarkId::new("view", num_entries),
            &payload,
            |b, payload| b.iter(|| SttsBoxView::decode(black_box(payload))),
        );

        group.bench_with_input(
            BenchmarkId::new("owned", num_entries),
            &payload,
            |b, payload| b.iter(|| SttsBox::decode(black_box(payload))),
        );
    }

    group.finish();
}

/// Benchmark: stts encode
fn bench_stts_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("stts_encode");

    for num_entries in [100, 1000, 10000] {
        let entries: Vec<SttsEntry> = (0..num_entries)
            .map(|i| SttsEntry {
                sample_count: (i as u32) + 1,
                sample_delta: 1000,
            })
            .collect();

        let stts = SttsBox {
            version: 0,
            flags: SttsFlags::empty(),
            entries,
        };

        let mut buf = vec![0u8; stts.encoded_len()];

        group.bench_with_input(
            BenchmarkId::from_parameter(num_entries),
            &stts,
            |b, stts| b.iter(|| stts.encode(black_box(&mut buf))),
        );
    }

    group.finish();
}

/// Benchmark: stts entry iteration
fn bench_stts_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("stts_iteration");

    for num_entries in [100, 1000, 10000] {
        let payload = make_stts_payload(num_entries);
        let view = SttsBoxView::decode(&payload).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(num_entries),
            &view,
            |b, view| {
                b.iter(|| {
                    let mut sum = 0u64;
                    for entry in view.entries() {
                        sum += entry.sample_delta as u64;
                    }
                    black_box(sum)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark: moov hierarchy parse
fn bench_moov_hierarchy(c: &mut Criterion) {
    let mut group = c.benchmark_group("moov_hierarchy");

    for num_tracks in [1, 4, 8] {
        let payload = make_moov_payload(num_tracks);

        group.bench_with_input(
            BenchmarkId::new("view", num_tracks),
            &payload,
            |b, payload| {
                b.iter(|| {
                    let moov = MoovBoxView::decode(black_box(payload)).unwrap();
                    let _ = moov.mvhd().unwrap();
                    for trak in moov.traks() {
                        let trak = trak.unwrap();
                        let _ = trak.tkhd().unwrap();
                    }
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("owned", num_tracks),
            &payload,
            |b, payload| b.iter(|| MoovBox::decode(black_box(payload))),
        );
    }

    group.finish();
}

/// Benchmark: moov encode
fn bench_moov_encode(c: &mut Criterion) {
    let payload = make_moov_payload(2);
    let moov = MoovBox::decode(&payload).unwrap();

    let mut buf = vec![0u8; moov.encoded_len()];

    c.bench_function("moov_encode_2tracks", |b| {
        b.iter(|| moov.encode(black_box(&mut buf)))
    });
}

criterion_group!(
    benches,
    bench_header_parse,
    bench_rawbox_parse,
    bench_ftyp_view_vs_owned,
    bench_ftyp_encode,
    bench_stts_decode,
    bench_stts_encode,
    bench_stts_iteration,
    bench_moov_hierarchy,
    bench_moov_encode,
);

criterion_main!(benches);
