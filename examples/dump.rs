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
    let args = Args::parse();

    let data = std::fs::read(args.input)?;

    println!("Parsing boxes in the input file...");
    let box_iter = mp4_bmff::BoxIter::new(&data);

    let start = std::time::Instant::now();

    for view in box_iter {
        let view = view?;
        println!(
            "Box: {:?}, Size: {}",
            view.header.boxtype(),
            view.header.boxsize(),
        );

        match view.header.boxtype().type_field().as_ascii() {
            Some("ftyp") => {
                let ftyp = mp4_bmff::boxes::FtypBoxView::parse(view.payload)?;
                println!("  Major Brand: {:?}", ftyp.major_brand);
                println!("  Minor Version: {}", ftyp.minor_version);
                println!("  Compatible Brands:");
                for brand in ftyp.compatible_brands() {
                    println!("    {:?}", brand);
                }
            }
            Some("moov") => {
                let moov = mp4_bmff::boxes::MoovBoxView::parse(view.payload)?;

                for trak in moov.traks() {
                    let trak = trak?;
                    println!("    Track Box:");
                    for trak_child in trak.children() {
                        let trak_child = trak_child?;
                        println!(
                            "      Track Child Box: {:?}, Size: {}",
                            trak_child.header.boxtype(),
                            trak_child.header.boxsize()
                        );
                    }
                }
            }
            Some("free") => {
                let free = mp4_bmff::boxes::FreeBoxView::parse(view.payload)?;
                println!("  Free Space Data Length: {}", free.data.len());
            }
            Some("mdat") => {
                let mdat = mp4_bmff::boxes::MdatBoxView::parse(view.payload)?;
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
