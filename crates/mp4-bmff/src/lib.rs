//! # mp4-bmff
//!
//! A pure Rust implementation of ISO/IEC 14496-12
//! (ISO Base Media File Format, ISOBMFF).
//!
//! This crate provides zero-copy parsing and encoding of BMFF/MP4 box structures,
//! supporting both `no_std` environments (with `alloc`) and full standard library usage.
//!
//! # Quick Start
//!
//! ## Reading Boxes from Bytes
//!
//! ```
//! use mp4_bmff::{iter_boxes, BoxType};
//!
//! // Parse consecutive boxes from a byte slice
//! let data = [
//!     0x00, 0x00, 0x00, 0x10, // size = 16
//!     b'f', b't', b'y', b'p', // type = "ftyp"
//!     b'i', b's', b'o', b'm', // brand
//!     0x00, 0x00, 0x00, 0x00, // version
//! ];
//!
//! for result in iter_boxes(&data) {
//!     let raw_box = result.unwrap();
//!     println!("Found box: {} ({} bytes)", raw_box.boxtype(), raw_box.len());
//! }
//! ```
//!
//! ## Decoding Typed Boxes
//!
//! ```
//! use mp4_bmff::read_box;
//! use mp4_bmff::boxes::bmff::FtypBox;
//!
//! let data = [
//!     0x00, 0x00, 0x00, 0x14, // size = 20
//!     b'f', b't', b'y', b'p', // type = "ftyp"
//!     b'i', b's', b'o', b'm', // major_brand
//!     0x00, 0x00, 0x02, 0x00, // minor_version
//!     b'i', b's', b'o', b'2', // compatible_brands[0]
//! ];
//!
//! let ftyp = read_box::<FtypBox>(&data).unwrap();
//! assert_eq!(ftyp.major_brand.as_bytes(), b"isom");
//! ```
//!
//! ## Writing Boxes
//!
//! ```
//! use mp4_bmff::write_box;
//! use mp4_bmff::boxes::bmff::FreeBox;
//!
//! let free = FreeBox {
//!     data: vec![0x00; 4], // 4 bytes of padding
//! };
//!
//! let mut buf = vec![0u8; 128];
//! let written = write_box(&mut buf, &free).unwrap();
//! assert_eq!(written, 12); // 8-byte header + 4-byte payload
//! ```
//!
//! ## Stream-based I/O (requires `std` feature)
//!
//! ```no_run
//! use std::fs::File;
//! use mp4_bmff::io::BoxReader;
//!
//! let file = File::open("video.mp4").unwrap();
//! let mut reader = BoxReader::new(file);
//!
//! while let Ok(raw_box) = reader.read_box() {
//!     println!("Box: {} at offset {}", raw_box.boxtype(), raw_box.len());
//! }
//! ```
//!
//! # Crate Structure
//!
//! The crate is organized into layers, each building on the previous:
//!
//! | Layer | Modules | Description |
//! |-------|---------|-------------|
//! | 0 | [`types`] | Primitive types (FourCC, fixed-point, timestamps) |
//! | 1 | [`base`], [`codec`], [`error`], [`iter`] | Core BMFF structures (`no_std` compatible) |
//! | 2 | [`boxes`], [`formats`] | Typed box representations (View/Copy: `no_std`, Owned: `alloc`) |
//! | 3 | `io` | Stream-based I/O (requires `std`) |
//!
//! # Key Types
//!
//! ## Box Structures
//!
//! - [`RawBox`] / [`RawBoxRef`]: Untyped box with raw payload bytes
//! - [`BoxHeader`]: Box header containing size and type
//! - [`BoxType`]: Box type identifier (FourCC or UUID)
//! - [`BoxSize`]: Box size (compact, extended, or EOF)
//!
//! ## Codec Traits
//!
//! - [`BoxCodec`]: Associates a box type with a Rust type
//! - [`BoxDecode`]: Decodes a box from bytes
//! - [`BoxEncode`]: Encodes a box to bytes
//!
//! ## I/O Types (with `std` feature)
//!
//! - `io::BoxReader`: Reads boxes from any `std::io::Read`
//! - `io::BoxWriter`: Writes boxes to any `std::io::Write`
//!
//! # Feature Flags
//!
//! | Feature | Default | Description |
//! |---------|---------|-------------|
//! | `std` | ✓ | Enables standard library support (implies `alloc`) |
//! | `alloc` | | Enables heap allocation for typed box representations |
//!
//! ## `no_std` Support
//!
//! This crate supports `no_std` environments with the `alloc` feature:
//!
//! ```toml
//! [dependencies]
//! mp4-bmff = { version = "0.1", default-features = false, features = ["alloc"] }
//! ```
//!
//! In `no_std` mode, the `io` module is unavailable, but all parsing and
//! encoding functionality works with byte slices.
//!
//! # Box Type Reference
//!
//! ## ISO BMFF Boxes ([`boxes::bmff`])
//!
//! | Type | Description |
//! |------|-------------|
//! | `ftyp` | File type and compatibility |
//! | `moov` | Movie container (metadata) |
//! | `mdat` | Media data container |
//! | `free`/`skip` | Free space |
//! | `mvhd` | Movie header |
//! | `trak` | Track container |
//! | `moof` | Movie fragment |
//! | `mfra` | Movie fragment random access |
//!
//! See [`boxes::bmff`] for the complete list of supported boxes.
//!
//! # Error Handling
//!
//! All fallible operations return [`Result<T>`], which uses the crate's
//! [`Error`] type. Errors include context about where they occurred:
//!
//! ```
//! use mp4_bmff::{read_box, Error, ErrorKind};
//! use mp4_bmff::boxes::bmff::FtypBox;
//!
//! let truncated_data = [0x00, 0x00, 0x00, 0x20, b'f', b't', b'y', b'p'];
//!
//! match read_box::<FtypBox>(&truncated_data) {
//!     Ok(_) => println!("Parsed successfully"),
//!     Err(e) => {
//!         println!("Error: {}", e);
//!         if let ErrorKind::NotEnoughBytes { expected, remaining } = e.kind() {
//!             println!("  Expected {} bytes, got {}", expected, remaining);
//!         }
//!     }
//! }
//! ```

#![cfg_attr(all(not(feature = "std"), not(test)), no_std)]
#![warn(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod lib {
    #[cfg(all(feature = "alloc", not(feature = "std")))]
    pub use alloc::{
        string::String, //
        string::ToString,
        vec,
        vec::Vec,
    };

    #[cfg(feature = "std")]
    pub use std::{
        string::String, //
        string::ToString,
        vec,
        vec::Vec,
    };
}

// =============================================================================
// Internal - Byte Slice Cursor Module
// =============================================================================
pub(crate) mod cursor;

// =============================================================================
// Layer 0 - Primitive Types
// =============================================================================
pub mod types;

// =============================================================================
// Layer 1 - ISO BMFF base structures (no_std)
// =============================================================================
pub mod base;
pub mod codec;
pub mod error;
pub mod iter;

// Re-export base structures
pub use base::header::{
    BoxHeader, //
    BoxSize,
    BoxType,
};
pub use base::rawbox::{
    RawBox, //
    RawBoxRef,
};

// Re-export error types
pub use error::{
    Error, //
    ErrorKind,
};

// Re-export codec traits
pub use codec::{
    BoxCodec, //
    BoxDecode,
    BoxEncode,
};

// Re-export codec helpers
pub use codec::read_box;
pub use codec::write_box;
pub use iter::iter_boxes;

// =============================================================================
// Layer 2 - Typed Box Representations (View/Copy: no_std, Owned: alloc)
// =============================================================================
pub mod boxes;
pub mod formats;

// Re-export owned box types
#[cfg(feature = "alloc")]
pub use base::rawbox::RawBoxOwned;

// =============================================================================
// Layer 3 -  I/O (std)
// =============================================================================
#[cfg(feature = "std")]
pub mod io;

// =============================================================================
// Prelude - Convenient imports
// =============================================================================
pub mod prelude;
