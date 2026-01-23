//! Example: Remux MP4 file using BoxReader and BoxWriter
//!
//! This example demonstrates how to read boxes from an MP4 file, decode them
//! into typed boxes, and write them back to a new file. This verifies the
//! round-trip capability of the typed box implementations.
//!
//! Usage:
//!   cargo run --example remux -- -i input.mp4 -o output.mp4

#[cfg(feature = "std")]
use clap::Parser;

#[cfg(feature = "std")]
use std::fs::File;
#[cfg(feature = "std")]
use std::io::{BufReader, BufWriter};
#[cfg(feature = "std")]
use std::path::PathBuf;

#[cfg(feature = "std")]
use mp4_bmff::io::{BoxReader, BoxWriter};
#[cfg(feature = "std")]
use mp4_bmff::{BoxDecode, BoxType};

#[cfg(feature = "std")]
use mp4_bmff::boxes::{FreeBox, FtypBox, MdatBox, MoofBox, MoovBox, StypBox};

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
    name = "mp4-remux",
    about = "Remux an MP4 file by reading and writing boxes through typed representations",
    version,
    author
)]
struct Args {
    /// Input MP4 file path.
    #[arg(short, long)]
    input: PathBuf,

    /// Output MP4 file path.
    #[arg(short, long, default_value = "remuxed.mp4")]
    output: PathBuf,
}

#[cfg(feature = "std")]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Open input file with BoxReader
    let input_file = File::open(&args.input)?;
    let buf_reader = BufReader::new(input_file);
    let reader = BoxReader::new(buf_reader);

    // Create output file with BoxWriter
    let output_file = File::create(&args.output)?;
    let buf_writer = BufWriter::new(output_file);
    let mut writer = BoxWriter::new(buf_writer);

    println!(
        "Remuxing {} -> {}",
        args.input.display(),
        args.output.display()
    );

    let mut box_count = 0u64;
    let mut total_bytes = 0u64;

    // Read and write each box through typed representation
    for result in reader {
        let raw_box = result?;

        let box_type = raw_box.boxtype();
        let box_len = raw_box.len() as u64;

        // Decode to typed box and write back
        match box_type {
            BoxType::FTYP => {
                let typed = FtypBox::decode(raw_box.payload())?;
                println!("  [{box_count}] {box_type} (typed): {box_len} bytes");
                println!("         major_brand: {}", typed.major_brand);
                writer.write_box(&typed)?;
            }
            BoxType::STYP => {
                let typed = StypBox::decode(raw_box.payload())?;
                println!("  [{box_count}] {box_type} (typed): {box_len} bytes");
                println!("         major_brand: {}", typed.major_brand);
                writer.write_box(&typed)?;
            }
            BoxType::MOOV => {
                let typed = MoovBox::decode(raw_box.payload())?;
                println!("  [{box_count}] {box_type} (typed): {box_len} bytes");
                println!("         tracks: {}", typed.traks.len());
                println!("         timescale: {}", typed.mvhd.timescale);
                println!("         duration: {}", typed.mvhd.duration);
                writer.write_box(&typed)?;
            }
            BoxType::MOOF => {
                let typed = MoofBox::decode(raw_box.payload())?;
                println!("  [{box_count}] {box_type} (typed): {box_len} bytes");
                println!("         sequence_number: {}", typed.mfhd.sequence_number);
                println!("         traf count: {}", typed.trafs.len());
                writer.write_box(&typed)?;
            }
            BoxType::MDAT => {
                let typed = MdatBox::decode(raw_box.payload())?;
                println!("  [{box_count}] {box_type} (typed): {box_len} bytes");
                println!("         data: {} bytes", typed.data.len());
                writer.write_box(&typed)?;
            }
            BoxType::FREE | BoxType::SKIP => {
                let typed = FreeBox::decode(raw_box.payload())?;
                println!("  [{box_count}] {box_type} (typed): {box_len} bytes");
                writer.write_box(&typed)?;
            }
            _ => {
                // For unsupported box types, write raw
                println!("  [{box_count}] {box_type} (raw): {box_len} bytes");
                writer.write_raw_box(&raw_box)?;
            }
        }

        box_count += 1;
        total_bytes += box_len;
    }

    // Flush the writer
    let buf_writer = writer.into_inner();
    buf_writer.into_inner()?;

    println!("\nCompleted: {box_count} boxes, {total_bytes} bytes written");

    Ok(())
}
