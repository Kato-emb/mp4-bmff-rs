use clap::Parser;

#[cfg(not(feature = "std"))]
fn main() {
    panic!("This example requires the 'std' feature to be enabled.");
}

#[cfg(feature = "std")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    run()
}

#[derive(Parser)]
struct Args {
    /// Input MP4 file
    #[arg(short, long)]
    input: String,
}

#[cfg(feature = "std")]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    use mp4_bmff::boxes::BoxIter;

    let args = Args::parse();

    let data = std::fs::read(args.input)?;

    println!("Parsing boxes in the input file...");
    let mut box_iter = BoxIter::new(&data);

    let start = std::time::Instant::now();

    while let Some(view) = box_iter.next() {
        let view = view?;
        println!(
            "Box: {:?}, Size: {}",
            view.header.boxtype(),
            view.header.boxsize(),
        );

        use mp4_bmff::boxes::specs::*;
        match view.header.boxtype().type_field().as_ascii() {
            Some("ftyp") => {
                let ftyp = FtypBoxRef::parse(view.payload)?;
                println!("  Major Brand: {:?}", ftyp.major_brand);
                println!("  Minor Version: {}", ftyp.minor_version);
                println!("  Compatible Brands:");
                for brand in ftyp.compatible_brands() {
                    println!("    {:?}", brand);
                }
            }
            Some("moov") => {
                let moov = MoovBoxRef::parse(view.payload)?;

                for child in moov.children() {
                    let child = child?;
                    println!(
                        "  Child Box: {:?}, Size: {}",
                        child.header.boxtype(),
                        child.header.boxsize()
                    );
                }
            }
            Some("free") => {
                let free = FreeBoxRef::parse(view.payload)?;
                println!("  Free Space Data Length: {}", free.data.len());
            }
            Some("mdat") => {
                let mdat = MdatBoxRef::parse(view.payload)?;
                println!("  Media Data Length: {}", mdat.data.len());
            }
            Some(typ) => {
                println!("  (No parser available for this box type '{}')", typ);
            }
            None => {
                println!("  (Non-ASCII box type)");
            }
        }
    }

    let duration = start.elapsed();
    println!("Finished parsing boxes in {:.2?}", duration);

    Ok(())
}
