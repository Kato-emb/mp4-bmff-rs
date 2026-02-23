use std::env;
use std::fs;
use std::process;

use mp4_bmff::boxes::bmff::{MoofBoxView, MoovBoxView};
use mp4_bmff::{BoxDecode, BoxType, iter_boxes};
use mp4_inspect::{stats, tree};

fn main() {
    let path = match env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("Usage: dump <file.mp4>");
            process::exit(1);
        }
    };

    let data = match fs::read(&path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error reading {path}: {e}");
            process::exit(1);
        }
    };

    // Build and print box tree
    let nodes = match tree::build_tree(&data) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("Error parsing boxes: {e}");
            process::exit(1);
        }
    };

    println!("=== Box Structure ===");
    print!("{}", tree::format_tree(&nodes));

    // Analyze via typed API
    let mut moof_index = 0u32;
    for result in iter_boxes(&data) {
        let raw = match result {
            Ok(r) => r,
            Err(_) => continue,
        };

        match raw.boxtype() {
            BoxType::MOOV => {
                let moov = match MoovBoxView::decode(raw.payload()) {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("Error decoding moov: {e}");
                        continue;
                    }
                };
                match stats::analyze(&moov) {
                    Ok(tracks) => {
                        println!("\n=== Sample Table Stats ===");
                        for track in &tracks {
                            print!("{track}");
                        }
                    }
                    Err(e) => eprintln!("Error analyzing moov: {e}"),
                }
            }
            BoxType::MOOF => {
                moof_index += 1;
                let moof = match MoofBoxView::decode(raw.payload()) {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("Error decoding moof #{moof_index}: {e}");
                        continue;
                    }
                };
                match stats::analyze_moof(&moof) {
                    Ok(frags) => {
                        println!("\n=== Fragment Stats (moof #{moof_index}) ===");
                        for frag in &frags {
                            print!("{frag}");
                        }
                    }
                    Err(e) => eprintln!("Error analyzing moof #{moof_index}: {e}"),
                }
            }
            _ => {}
        }
    }
}
