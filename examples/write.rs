//! Example: Writing MP4 boxes using BoxWriter
//!
//! This example demonstrates how to create and write MP4 boxes to a file.
//!
//! Usage:
//!   cargo run --example write -- -o output.mp4

#[cfg(feature = "std")]
use clap::Parser;

#[cfg(feature = "std")]
use std::fs::File;
#[cfg(feature = "std")]
use std::io::BufWriter;
#[cfg(feature = "std")]
use std::path::PathBuf;

#[cfg(feature = "std")]
use mp4_bmff::boxes::FtypBox;
#[cfg(feature = "std")]
use mp4_bmff::io::BoxWriter;
#[cfg(feature = "std")]
use mp4_bmff::types::FourCC;

#[cfg(not(feature = "std"))]
fn main() {
    panic!("This example requires the 'std' feature to be enabled.");
}

#[cfg(feature = "std")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    run()
}

#[cfg(feature = "std")]
#[derive(Parser, Debug)]
#[command(
    name = "mp4-write",
    about = "Write a minimal MP4 file with ftyp box",
    version,
    author
)]
struct Args {
    /// Output MP4 file path.
    #[arg(short, long, default_value = "output.mp4")]
    output: PathBuf,
}

#[cfg(feature = "std")]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Create the output file
    let file = File::create(&args.output)?;
    let buf_writer = BufWriter::new(file);
    let mut writer = BoxWriter::new(buf_writer);

    // Create an ftyp box
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

    println!("Writing ftyp box:");
    println!("  major_brand: {}", ftyp.major_brand);
    println!("  minor_version: {}", ftyp.minor_version);
    println!(
        "  compatible_brands: {}",
        ftyp.compatible_brands
            .iter()
            .map(|b| b.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Write the ftyp box
    writer.write_box(&ftyp)?;

    // Flush and get the inner writer
    let buf_writer = writer.into_inner();
    let file = buf_writer.into_inner()?;
    let metadata = file.metadata()?;

    println!(
        "\nWrote {} bytes to {}",
        metadata.len(),
        args.output.display()
    );

    Ok(())
}
