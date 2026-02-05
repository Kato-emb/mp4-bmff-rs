//! Stream-based I/O utilities for BMFF parsing and writing.
//!
//! This module provides types for reading and writing BMFF boxes from/to
//! any type implementing the standard library's [`Read`](std::io::Read) and
//! [`Write`](std::io::Write) traits.
//!
//! # Overview
//!
//! - [`BoxReader`]: Reads boxes from a stream (file, socket, etc.)
//! - [`BoxWriter`]: Writes boxes to a stream
//!
//! These types complement the in-memory [`read_box`](crate::codec::read_box) and
//! [`write_box`](crate::codec::write_box) functions by providing stream-based I/O.
//!
//! # Reading Boxes
//!
//! [`BoxReader`] wraps any [`Read`](std::io::Read) implementor and provides
//! methods to read boxes one at a time. It also implements [`Iterator`] for
//! convenient sequential reading.
//!
//! ```no_run
//! use std::fs::File;
//! use mp4_bmff::io::BoxReader;
//!
//! let file = File::open("video.mp4").unwrap();
//! let reader = BoxReader::new(file);
//!
//! for result in reader {
//!     let raw_box = result.unwrap();
//!     println!("Box type: {}", raw_box.boxtype());
//! }
//! ```
//!
//! # Writing Boxes
//!
//! [`BoxWriter`] wraps any [`Write`](std::io::Write) implementor and provides
//! methods to write encoded boxes or raw boxes.
//!
//! ```no_run
//! use std::fs::File;
//! use mp4_bmff::io::BoxWriter;
//! use mp4_bmff::BoxType;
//! use mp4_bmff::RawBox;
//!
//! let file = File::create("output.mp4").unwrap();
//! let mut writer = BoxWriter::new(file);
//!
//! let raw = RawBox::new(BoxType::FTYP, vec![/* payload */]);
//! writer.write_raw_box(&raw).unwrap();
//! ```
//!
//! # Feature Flag
//!
//! This module requires the `std` feature (enabled by default).

pub mod reader;
pub mod writer;

pub use reader::BoxReader;
pub use writer::BoxWriter;
