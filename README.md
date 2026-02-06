# mp4-bmff

[![Crates.io](https://img.shields.io/crates/v/mp4-bmff.svg)](https://crates.io/crates/mp4-bmff)
[![Documentation](https://docs.rs/mp4-bmff/badge.svg)](https://docs.rs/mp4-bmff)
[![License](https://img.shields.io/crates/l/mp4-bmff.svg)](LICENSE)

A pure Rust, zero-copy implementation of ISO Base Media File Format (ISO/IEC 14496-12).

## Features

- **Zero-copy parsing** - View types parse directly from byte slices without allocation
- **`no_std` support** - Core functionality works without standard library
- **Layered design** - Use only what you need, from primitives to full I/O
- **Type-safe** - Strongly typed box structures with compile-time guarantees

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
mp4-bmff = "0.1"
```

For `no_std` environments:

```toml
[dependencies]
mp4-bmff = { version = "0.1", default-features = false, features = ["alloc"] }
```

## Quick Start

### Parse boxes from bytes

```rust
use mp4_bmff::{iter_boxes, BoxType};

let data: &[u8] = /* MP4 data */;

for result in iter_boxes(data) {
    let raw_box = result?;
    println!("{}: {} bytes", raw_box.boxtype(), raw_box.len());
}
```

### Decode typed boxes

```rust
use mp4_bmff::read_box;
use mp4_bmff::boxes::bmff::FtypBox;

let data: &[u8] = /* ftyp box data */;

let ftyp = read_box::<FtypBox>(data)?;
println!("Major brand: {}", ftyp.major_brand);
```

### Write boxes

```rust
use mp4_bmff::write_box;
use mp4_bmff::boxes::bmff::FreeBox;

let free = FreeBox { data: vec![0u8; 16] };

let mut buf = vec![0u8; 256];
let written = write_box(&mut buf, &free)?;
```

### Stream-based I/O

```rust
use std::fs::File;
use mp4_bmff::io::BoxReader;

let file = File::open("video.mp4")?;
for result in BoxReader::new(file) {
    let raw_box = result?;
    println!("{}: {} bytes", raw_box.boxtype(), raw_box.len());
}
```

## Feature Flags

| Feature   | Default | Description                                |
| --------- | ------- | ------------------------------------------ |
| `std`     | ✓       | Standard library support (implies `alloc`) |
| `alloc`   |         | Heap allocation for owned box types        |
| `mp4`     | ✓       | ISO/IEC 14496-14 boxes                     |
| `avc`     | ✓       | AVC/H.264 support                          |
| `hevc`    |         | HEVC/H.265 support                         |
| `systems` |         | MPEG-4 Systems descriptors                 |

## Architecture

The crate is organized into layers:

| Layer | Modules                          | Feature                      | Description                                  |
| ----- | -------------------------------- | ---------------------------- | -------------------------------------------- |
| 0     | `types`                          | -                            | Primitives (FourCC, fixed-point, timestamps) |
| 1     | `base`, `codec`, `error`, `iter` | -                            | Core BMFF structures                         |
| 2     | `boxes`, `formats`               | View/Copy: -, Owned: `alloc` | Typed box representations                    |
| 3     | `io`                             | `std`                        | Stream-based I/O                             |

### Box Type Variants

- **View types** (`*BoxView<'a>`) - Zero-copy references, `no_std` compatible
- **Copy types** (`MvhdBox`, etc.) - Fixed-size boxes, `no_std` compatible
- **Owned types** (`*Box`) - Heap-allocated, requires `alloc`

## Supported Boxes

ISO/IEC 14496-12 (BMFF) boxes:

- File structure: `ftyp`, `mdat`, `free`, `skip`, `pdin`
- Movie: `moov`, `mvhd`, `trak`, `tkhd`, `mdia`, `mdhd`, `hdlr`, `minf`, `stbl`
- Sample tables: `stsd`, `stts`, `ctts`, `stsc`, `stsz`, `stco`, `co64`, `stss`, `sdtp`
- Fragments: `mvex`, `moof`, `mfhd`, `traf`, `tfhd`, `tfdt`, `trun`
- Random access: `mfra`, `tfra`, `mfro`
- Segments: `styp`

See [documentation](https://docs.rs/mp4-bmff) for the complete list.

## Roadmap

- [ ] **HEVC/H.265 support** - `hvc1`, `hev1` sample entries and HEVCDecoderConfigurationRecord
- [ ] **Additional BMFF boxes**
  - [ ] `meta` - Metadata container
  - [ ] `iloc` - Item location
  - [ ] `iinf` - Item information
  - [ ] `pitm` - Primary item
  - [ ] `iref` - Item reference
  - [ ] `iprp` - Item properties
  - [ ] `udta` - User data
  - [ ] `cprt` - Copyright
  - [ ] `sbgp` - Sample to group
  - [ ] `sgpd` - Sample group description
  - [ ] `subs` - Sub-sample information
  - [ ] `sidx` - Segment index
  - [ ] `ssix` - Subsegment index
  - [ ] `prft` - Producer reference time
- [ ] **Extended codec support**
  - [ ] VVC/H.266
  - [ ] AV1

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
