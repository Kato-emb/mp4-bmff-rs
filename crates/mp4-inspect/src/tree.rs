//! Box structure tree representation.
//!
//! Builds a hierarchical tree of BMFF boxes from a byte slice,
//! recursing into known container boxes.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use mp4_bmff::BoxDecode;
use mp4_bmff::BoxType;
use mp4_bmff::boxes::bmff::StsdBoxView;
use mp4_bmff::error::Result;
use mp4_bmff::iter::BoxIter;
use mp4_bmff::iter_boxes;
use mp4_bmff::types::FourCC;

/// A node in the BMFF box tree.
///
/// Each node represents one box, with container boxes holding
/// their children in [`children`](BoxNode::children).
#[derive(Debug, Clone)]
pub struct BoxNode {
    /// Box type (FourCC).
    pub fourcc: FourCC,
    /// Byte offset from the start of the parsed data.
    pub offset: u64,
    /// Total box size in bytes (header + payload).
    pub size: u64,
    /// Header size in bytes (8, 16, or 32).
    pub header_size: u8,
    /// Child boxes (non-empty only for container boxes).
    pub children: Vec<BoxNode>,
}

/// Builds a box tree from a byte slice.
///
/// Top-level boxes are returned as a `Vec`. Container boxes
/// are recursed into, populating their `children`.
///
/// # Errors
///
/// Returns an error if a box header cannot be parsed.
pub fn build_tree(data: &[u8]) -> Result<Vec<BoxNode>> {
    collect_nodes(iter_boxes(data), 0)
}

fn collect_nodes(iter: BoxIter<'_>, base_offset: u64) -> Result<Vec<BoxNode>> {
    let mut nodes = Vec::new();
    let mut rel_offset = 0u64;

    for result in iter {
        let raw = result?;
        let size = raw.len() as u64;
        // header_len() is at most 32 (BoxHeader::MAX_HEADER_SIZE), fits in u8
        #[allow(clippy::cast_possible_truncation)]
        let header_size = raw.header().header_len() as u8;
        let fourcc = raw.boxtype().type_field();
        let abs_offset = base_offset + rel_offset;

        let children = if raw.boxtype() == BoxType::STSD {
            let stsd = StsdBoxView::decode(raw.payload())?;
            let payload_end = abs_offset + size;
            collect_nodes_rev(stsd.sample_entries(), payload_end)?
        } else if is_container(raw.boxtype()) {
            collect_nodes(iter_boxes(raw.payload()), abs_offset + u64::from(header_size))?
        } else {
            Vec::new()
        };

        nodes.push(BoxNode {
            fourcc,
            offset: abs_offset,
            size,
            header_size,
            children,
        });

        rel_offset += size;
    }

    Ok(nodes)
}

/// Collects nodes from an iterator, computing offsets backwards from the
/// known end position of the parent box.
///
/// This is used for boxes like `stsd` where the child data does not start
/// at a fixed offset from the payload start (the typed API provides the
/// iterator but not the raw byte offset). Instead we use the fact that
/// the entries are packed at the end of the parent box:
/// `entry_offset = parent_end - total_entries_size + relative_offset`.
fn collect_nodes_rev(iter: BoxIter<'_>, parent_end: u64) -> Result<Vec<BoxNode>> {
    let mut nodes = Vec::new();
    let mut total_entries_size = 0u64;

    for result in iter {
        let raw = result?;
        let size = raw.len() as u64;
        #[allow(clippy::cast_possible_truncation)]
        let header_size = raw.header().header_len() as u8;
        let fourcc = raw.boxtype().type_field();

        nodes.push(BoxNode {
            fourcc,
            offset: total_entries_size, // relative offset (fixed up below)
            size,
            header_size,
            children: Vec::new(),
        });

        total_entries_size += size;
    }

    // Fix up: convert relative offsets to absolute
    let entries_base = parent_end - total_entries_size;
    for node in &mut nodes {
        node.offset += entries_base;
    }

    Ok(nodes)
}

/// Returns `true` if the given box type is a known pure container
/// whose payload consists entirely of child boxes.
///
/// Note: `stsd` is handled separately because it has a FullBox header
/// before its child boxes.
fn is_container(bt: BoxType) -> bool {
    matches!(
        bt,
        // ISO 14496-12 (BMFF) containers
        BoxType::MOOV
            | BoxType::TRAK
            | BoxType::MDIA
            | BoxType::MINF
            | BoxType::STBL
            | BoxType::DINF
            | BoxType::EDTS
            | BoxType::MVEX
            | BoxType::MOOF
            | BoxType::TRAF
            | BoxType::MFRA
            | BoxType::TREF
            | BoxType::TRGR
    )
}

impl fmt::Display for BoxNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_node(f, self, 0)
    }
}

fn write_node(f: &mut fmt::Formatter<'_>, node: &BoxNode, depth: usize) -> fmt::Result {
    let indent = depth * 2;
    writeln!(
        f,
        "{:indent$}[{}] {} bytes (offset {})",
        "",
        node.fourcc,
        node.size,
        node.offset,
        indent = indent,
    )?;
    for child in &node.children {
        write_node(f, child, depth + 1)?;
    }
    Ok(())
}

/// Formats a list of top-level box nodes as a tree string.
pub fn format_tree(nodes: &[BoxNode]) -> String {
    let mut s = String::new();
    for node in nodes {
        format_node(&mut s, node, 0);
    }
    s
}

fn format_node(s: &mut String, node: &BoxNode, depth: usize) {
    use core::fmt::Write;
    let indent = depth * 2;
    let _ = writeln!(
        s,
        "{:indent$}[{}] {} bytes (offset {})",
        "",
        node.fourcc,
        node.size,
        node.offset,
        indent = indent,
    );
    for child in &node.children {
        format_node(s, child, depth + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_boxes() {
        #[rustfmt::skip]
        let data = [
            // ftyp (size=8, header only)
            0x00, 0x00, 0x00, 0x08, b'f', b't', b'y', b'p',
            // mdat (size=12, 4-byte payload)
            0x00, 0x00, 0x00, 0x0C, b'm', b'd', b'a', b't',
            0x00, 0x00, 0x00, 0x00,
        ];

        let tree = build_tree(&data).unwrap();
        assert_eq!(tree.len(), 2);

        assert_eq!(tree[0].fourcc, FourCC::new(*b"ftyp"));
        assert_eq!(tree[0].offset, 0);
        assert_eq!(tree[0].size, 8);
        assert!(tree[0].children.is_empty());

        assert_eq!(tree[1].fourcc, FourCC::new(*b"mdat"));
        assert_eq!(tree[1].offset, 8);
        assert_eq!(tree[1].size, 12);
        assert!(tree[1].children.is_empty());
    }

    #[test]
    fn container_with_children() {
        // moov containing two child boxes
        #[rustfmt::skip]
        let data = [
            // moov (size=24)
            0x00, 0x00, 0x00, 0x18, b'm', b'o', b'o', b'v',
            // child1: free (size=8)
            0x00, 0x00, 0x00, 0x08, b'f', b'r', b'e', b'e',
            // child2: free (size=8)
            0x00, 0x00, 0x00, 0x08, b'f', b'r', b'e', b'e',
        ];

        let tree = build_tree(&data).unwrap();
        assert_eq!(tree.len(), 1);

        let moov = &tree[0];
        assert_eq!(moov.fourcc, FourCC::new(*b"moov"));
        assert_eq!(moov.size, 24);
        assert_eq!(moov.children.len(), 2);

        assert_eq!(moov.children[0].fourcc, FourCC::new(*b"free"));
        assert_eq!(moov.children[0].offset, 8);
        assert_eq!(moov.children[0].size, 8);

        assert_eq!(moov.children[1].fourcc, FourCC::new(*b"free"));
        assert_eq!(moov.children[1].offset, 16);
    }

    #[test]
    fn display_output() {
        #[rustfmt::skip]
        let data = [
            0x00, 0x00, 0x00, 0x18, b'm', b'o', b'o', b'v',
            0x00, 0x00, 0x00, 0x08, b'f', b'r', b'e', b'e',
            0x00, 0x00, 0x00, 0x08, b'f', b'r', b'e', b'e',
        ];

        let tree = build_tree(&data).unwrap();
        let output = format_tree(&tree);
        assert!(output.contains("[moov]"));
        assert!(output.contains("  [free]"));
    }
}
