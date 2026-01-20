#[cfg(feature = "std")]
use clap::Parser;

#[cfg(feature = "std")]
use std::path::PathBuf;

#[cfg(feature = "std")]
use mp4_bmff::{
    BoxFrame, BoxIter, BoxType, Error,
    boxes::{FreeBoxView, FtypBoxView, HdlrBoxView, MdhdBox, MvhdBox, StypBoxView, TkhdBox},
};

#[cfg(feature = "std")]
use mp4_bmff::types::{FourCC, QuickTimeDateTime};

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
    name = "mp4-dump",
    about = "Dump the ISO-BMFF (MP4) box hierarchy for a file",
    version,
    author
)]
struct Args {
    /// Input MP4 file to inspect.
    #[arg(short, long)]
    input: PathBuf,
}

#[cfg(feature = "std")]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let data = std::fs::read(&args.input)?;

    println!("Dumping {} ({} bytes)\n", args.input.display(), data.len());

    dump_boxes(&data, 0, 0).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    Ok(())
}

#[cfg(feature = "std")]
fn dump_boxes(data: &[u8], depth: usize, base_offset: u64) -> mp4_bmff::Result<()> {
    let mut offset_in_slice = 0usize;

    for view_result in BoxIter::new(data) {
        let view = view_result?;

        let boxed_size = view
            .boxsize()
            .value()
            .unwrap_or((data.len() - offset_in_slice) as u64);
        let start_offset = base_offset + offset_in_slice as u64;

        print_box(&view, depth, start_offset, boxed_size)?;

        offset_in_slice += boxed_size as usize;
    }

    Ok(())
}

#[cfg(feature = "std")]
fn print_box(view: &BoxFrame<'_>, depth: usize, offset: u64, size: u64) -> mp4_bmff::Result<()> {
    let indent = "  ".repeat(depth);
    let payload_len = view.payload().len() as u64;
    let header_len = view.header_len() as u64;
    debug_assert_eq!(payload_len + header_len, size);

    println!(
        "{indent}{ty}  offset=0x{offset:08X}  size={}  header={} bytes  payload={} bytes",
        describe_size(view, size),
        header_len,
        payload_len,
        ty = view.boxtype(),
    );

    print_box_details(view, depth)?;

    if should_recurse(view.boxtype()) && !view.payload().is_empty() {
        let child_base = offset + header_len;
        dump_boxes(view.payload(), depth + 1, child_base)?;
    }

    Ok(())
}

#[cfg(feature = "std")]
fn print_box_details(view: &BoxFrame<'_>, depth: usize) -> mp4_bmff::Result<()> {
    let indent = "  ".repeat(depth + 1);

    match view.boxtype() {
        BoxType::FTYP => match FtypBoxView::parse(view.payload()) {
            Ok(ftyp) => {
                println!("{indent}major_brand: {}", ftyp.major_brand);
                println!("{indent}minor_version: {}", ftyp.minor_version);
                if let Some(list) = format_fourcc_list(ftyp.compatible_brands()) {
                    println!("{indent}compatible_brands: {list}");
                }
            }
            Err(err) => print_parse_error(&indent, "ftyp", &err),
        },
        BoxType::STYP => match StypBoxView::parse(view.payload()) {
            Ok(styp) => {
                println!("{indent}major_brand: {}", styp.major_brand);
                println!("{indent}minor_version: {}", styp.minor_version);
                if let Some(list) = format_fourcc_list(styp.compatible_brands()) {
                    println!("{indent}compatible_brands: {list}");
                }
            }
            Err(err) => print_parse_error(&indent, "styp", &err),
        },
        BoxType::MVHD => match MvhdBox::parse(view.payload()) {
            Ok(mvhd) => {
                println!("{indent}timescale: {}", mvhd.timescale);
                println!(
                    "{indent}duration: {}",
                    describe_duration(mvhd.duration, mvhd.timescale)
                );
                println!(
                    "{indent}created: {}",
                    describe_timestamp(mvhd.creation_time)
                );
                println!(
                    "{indent}modified: {}",
                    describe_timestamp(mvhd.modification_time)
                );
                println!("{indent}next_track_id: {}", mvhd.next_track_id);
            }
            Err(err) => print_parse_error(&indent, "mvhd", &err),
        },
        BoxType::TKHD => match TkhdBox::parse(view.payload()) {
            Ok(tkhd) => {
                println!("{indent}track_id: {}", tkhd.track_id);
                println!("{indent}duration: {}", tkhd.duration);
                println!("{indent}layer: {}", tkhd.layer);
                println!("{indent}alternate_group: {}", tkhd.alternate_group);
                println!("{indent}volume: {}", tkhd.volume);
                println!("{indent}width: {}", tkhd.width);
                println!("{indent}height: {}", tkhd.height);
            }
            Err(err) => print_parse_error(&indent, "tkhd", &err),
        },
        BoxType::MDHD => match MdhdBox::parse(view.payload()) {
            Ok(mdhd) => {
                println!("{indent}timescale: {}", mdhd.timescale);
                println!(
                    "{indent}duration: {}",
                    describe_duration(mdhd.duration, mdhd.timescale)
                );
                println!("{indent}language: {}", mdhd.language);
                println!(
                    "{indent}created: {}",
                    describe_timestamp(mdhd.creation_time)
                );
                println!(
                    "{indent}modified: {}",
                    describe_timestamp(mdhd.modification_time)
                );
            }
            Err(err) => print_parse_error(&indent, "mdhd", &err),
        },
        BoxType::HDLR => match HdlrBoxView::parse(view.payload()) {
            Ok(hdlr) => {
                println!("{indent}handler_type: {}", hdlr.handler_type);
                println!("{indent}name: {}", hdlr.name);
            }
            Err(err) => print_parse_error(&indent, "hdlr", &err),
        },
        BoxType::FREE => match FreeBoxView::parse(view.payload()) {
            Ok(free) => println!("{indent}free_space: {} bytes", free.data.len()),
            Err(err) => print_parse_error(&indent, "free", &err),
        },
        BoxType::MDAT => {
            println!("{indent}media_data: {} bytes", view.payload().len());
        }
        _ => {}
    }

    Ok(())
}

#[cfg(feature = "std")]
fn should_recurse(box_type: BoxType) -> bool {
    matches!(
        box_type,
        BoxType::MOOV
            | BoxType::TRAK
            | BoxType::MDIA
            | BoxType::MINF
            | BoxType::DINF
            | BoxType::STBL
            | BoxType::MVEX
            | BoxType::MOOF
            | BoxType::TRAF
            | BoxType::MFRA
            | BoxType::TREF
    )
}

#[cfg(feature = "std")]
fn describe_size(view: &BoxFrame<'_>, actual: u64) -> String {
    if view.boxsize().is_eof() {
        format!("extends to EOF ({actual} bytes)")
    } else if view.boxsize().is_extended() {
        format!("{actual} bytes (64-bit size)")
    } else {
        format!("{actual} bytes")
    }
}

#[cfg(feature = "std")]
fn format_fourcc_list<'a>(brands: impl Iterator<Item = FourCC>) -> Option<String> {
    let list: Vec<String> = brands.map(|b| b.to_string()).collect();
    if list.is_empty() {
        None
    } else {
        Some(list.join(", "))
    }
}

#[cfg(feature = "std")]
fn describe_duration(duration: u64, timescale: u32) -> String {
    if timescale == 0 {
        return format!("{duration} (timescale=0)");
    }

    let seconds = duration as f64 / f64::from(timescale);

    if seconds >= 10.0 {
        format!("{duration} ({seconds:.2}s)")
    } else {
        format!("{duration} ({seconds:.4}s)")
    }
}

#[cfg(feature = "std")]
fn describe_timestamp(timestamp: QuickTimeDateTime) -> String {
    let quicktime = timestamp.to_quicktime_seconds();
    match timestamp.to_unix_seconds() {
        Some(unix) => format!("{unix} (QuickTime={quicktime})"),
        None => format!("before Unix epoch (QuickTime={quicktime})"),
    }
}

#[cfg(feature = "std")]
fn print_parse_error(indent: &str, name: &str, err: &Error) {
    println!("{indent}{name}: <failed to parse: {err}>");
}
