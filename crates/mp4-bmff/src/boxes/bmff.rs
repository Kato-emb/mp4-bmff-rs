//! ISO/IEC 14496-12 (BMFF) Box Structures.
//!
//! This module provides types for working with boxes defined in ISO/IEC 14496-12,
//! the ISO Base Media File Format (BMFF). BMFF is the foundation for MP4, MOV,
//! and other related container formats.
//!
//! # Box Categories
//!
//! ## File Structure Boxes
//! - `FtypBox`/[`FtypBoxView`]: File Type Box - identifies file format and compatibility.
//! - `MdatBox`/[`MdatBoxView`]: Media Data Box - contains actual media samples.
//! - `FreeBox`/[`FreeBoxView`]: Free Space Box - padding/placeholder.
//! - `PdinBox`/[`PdinBoxView`]: Progressive Download Information Box.
//!
//! ## Movie Structure Boxes
//! - `MoovBox`/[`MoovBoxView`]: Movie Box - top-level container for movie metadata.
//! - [`MvhdBox`]: Movie Header Box - global movie information (timescale, duration).
//!
//! ## Track Structure Boxes
//! - `TrakBox`/[`TrakBoxView`]: Track Box - container for a single track.
//! - [`TkhdBox`]: Track Header Box - track properties (dimensions, volume).
//! - `TrefBox`/[`TrefBoxView`]: Track Reference Box - track relationships.
//! - `TrgrBox`/[`TrgrBoxView`]: Track Group Box - track grouping.
//! - `EdtsBox`/[`EdtsBoxView`]: Edit Box - edit list container.
//! - `ElstBox`/[`ElstBoxView`]: Edit List Box - time remapping entries.
//!
//! ## Media Structure Boxes
//! - `MdiaBox`/[`MdiaBoxView`]: Media Box - media information container.
//! - [`MdhdBox`]: Media Header Box - media timescale and duration.
//! - `HdlrBox`/[`HdlrBoxView`]: Handler Reference Box - media handler type.
//! - `MinfBox`/[`MinfBoxView`]: Media Information Box.
//! - [`VmhdBox`]: Video Media Header Box.
//! - [`SmhdBox`]: Sound Media Header Box.
//! - [`NmhdBox`]: Null Media Header Box (for hint/metadata tracks).
//! - `ElngBox`/[`ElngBoxView`]: Extended Language Tag Box.
//!
//! ## Sample Table Boxes
//! - `StblBox`/[`StblBoxView`]: Sample Table Box - sample metadata container.
//! - `StsdBox`/[`StsdBoxView`]: Sample Description Box - codec configurations.
//! - `SttsBox`/[`SttsBoxView`]: Decoding Time to Sample Box.
//! - `CttsBox`/[`CttsBoxView`]: Composition Time to Sample Box.
//! - [`CslgBox`]: Composition to Decode Box.
//! - `StscBox`/[`StscBoxView`]: Sample to Chunk Box.
//! - `StszBox`/[`StszBoxView`]: Sample Size Box.
//! - `Stz2Box`/[`Stz2BoxView`]: Compact Sample Size Box.
//! - `StcoBox`/[`StcoBoxView`]: Chunk Offset Box.
//! - `Co64Box`/[`Co64BoxView`]: Chunk Large Offset Box (for files >4GB).
//! - `StssBox`/[`StssBoxView`]: Sync Sample Box (keyframes).
//! - `StshBox`/[`StshBoxView`]: Shadow Sync Sample Box.
//! - `SdtpBox`/[`SdtpBoxView`]: Sample Dependency Type Box.
//! - `StdpBox`/[`StdpBoxView`]: Degradation Priority Box.
//!
//! ## Data Reference Boxes
//! - `DinfBox`/[`DinfBoxView`]: Data Information Box.
//! - `DrefBox`/[`DrefBoxView`]: Data Reference Box.
//! - `UrlBox`/[`UrlBoxView`]: URL Data Entry Box.
//! - `UrnBox`/[`UrnBoxView`]: URN Data Entry Box.
//!
//! ## Movie Fragment Boxes
//! - `MvexBox`/[`MvexBoxView`]: Movie Extends Box - enables fragmentation.
//! - [`MehdBox`]: Movie Extends Header Box.
//! - [`TrexBox`]: Track Extends Box - default sample values.
//! - `MoofBox`/[`MoofBoxView`]: Movie Fragment Box.
//! - [`MfhdBox`]: Movie Fragment Header Box.
//! - `TrafBox`/[`TrafBoxView`]: Track Fragment Box.
//! - [`TfhdBox`]: Track Fragment Header Box.
//! - [`TfdtBox`]: Track Fragment Decode Time Box.
//! - `TrunBox`/[`TrunBoxView`]: Track Run Box - sample timing/sizes.
//!
//! ## Random Access Boxes
//! - `MfraBox`/[`MfraBoxView`]: Movie Fragment Random Access Box.
//! - `TfraBox`/[`TfraBoxView`]: Track Fragment Random Access Box.
//! - [`MfroBox`]: Movie Fragment Random Access Offset Box.
//!
//! ## Segment Boxes
//! - `StypBox`/[`StypBoxView`]: Segment Type Box - segment branding.

mod common;

mod free;
mod ftyp;
mod mdat;
mod pdin;

mod moov;
mod mvhd;

mod edts;
mod elst;
mod tkhd;
mod trak;
mod tref;
mod trgr;

mod hdlr;
mod mdhd;
mod mdia;

mod elng;
mod minf;
mod nmhd;
mod smhd;
mod vmhd;

mod co64;
mod cslg;
mod ctts;
mod sdtp;
mod stbl;
mod stco;
mod stdp;
mod stsc;
mod stsd;
mod stsh;
mod stss;
mod stsz;
mod stts;
mod stz2;

mod dinf;
mod dref;

mod mehd;
mod mvex;
mod trex;

mod mfhd;
mod moof;

mod tfdt;
mod tfhd;
mod traf;
mod trun;

mod mfra;
mod mfro;
mod tfra;

mod styp;

pub use common::{
    IsLeading, //
    SampleDependsOn,
    SampleFlags,
    SampleHasRedundancy,
    SampleIsDependedOn,
};

// Variable-size boxes - View types
pub use co64::Co64BoxView;
pub use ctts::CttsBoxView;
pub use dref::DrefBoxView;
pub use dref::{DataEntryBoxView, UrlBoxView, UrnBoxView};
pub use elng::ElngBoxView;
pub use elst::ElstBoxView;
pub use free::FreeBoxView;
pub use ftyp::FtypBoxView;
pub use hdlr::HdlrBoxView;
pub use mdat::MdatBoxView;
pub use pdin::PdinBoxView;
pub use sdtp::SdtpBoxView;
pub use stco::StcoBoxView;
pub use stdp::StdpBoxView;
pub use stsc::StscBoxView;
pub use stsd::StsdBoxView;
pub use stsh::StshBoxView;
pub use stss::StssBoxView;
pub use stsz::StszBoxView;
pub use stts::SttsBoxView;
pub use styp::StypBoxView;
pub use stz2::Stz2BoxView;
pub use tfra::TfraBoxView;
pub use tref::TrefTypeBoxView;
pub use trun::TrunBoxView;

// Container boxes - View types
pub use dinf::DinfBoxView;
pub use edts::EdtsBoxView;
pub use mdia::MdiaBoxView;
pub use mfra::MfraBoxView;
pub use minf::MinfBoxView;
pub use moof::MoofBoxView;
pub use moov::MoovBoxView;
pub use mvex::MvexBoxView;
pub use stbl::StblBoxView;
pub use traf::TrafBoxView;
pub use trak::TrakBoxView;
pub use tref::TrefBoxView;
pub use trgr::TrgrBoxView;

// Fixed-size boxes (Copy types, no View/Owned distinction)
pub use cslg::CslgBox;
pub use mdhd::MdhdBox;
pub use mehd::MehdBox;
pub use mfhd::MfhdBox;
pub use mfro::MfroBox;
pub use mvhd::MvhdBox;
pub use nmhd::NmhdBox;
pub use smhd::SmhdBox;
pub use tfdt::TfdtBox;
pub use tfhd::TfhdBox;
pub use tkhd::TkhdBox;
pub use trex::TrexBox;
pub use trgr::TrgrTypeBox;
pub use vmhd::VmhdBox;

// Re-export fullbox flags
pub use co64::Co64Flags;
pub use cslg::CslgFlags;
pub use ctts::CttsFlags;
pub use dref::DrefFlags;
pub use dref::UrlFlags;
pub use dref::UrnFlags;
pub use elng::ElngFlags;
pub use elst::ElstFlags;
pub use hdlr::HdlrFlags;
pub use mdhd::MdhdFlags;
pub use mehd::MehdFlags;
pub use mfhd::MfhdFlags;
pub use mfro::MfroFlags;
pub use mvhd::MvhdFlags;
pub use nmhd::NmhdFlags;
pub use sdtp::SdtpFlags;
pub use smhd::SmhdFlags;
pub use stco::StcoFlags;
pub use stdp::StdpFlags;
pub use stsc::StscFlags;
pub use stsd::StsdFlags;
pub use stsh::StshFlags;
pub use stss::StssFlags;
pub use stsz::StszFlags;
pub use stts::SttsFlags;
pub use stz2::Stz2Flags;
pub use tfdt::TfdtFlags;
pub use tfhd::TfhdFlags;
pub use tfra::TfraFlags;
pub use tkhd::TkhdFlags;
pub use trex::TrexFlags;
pub use trun::TrunFlags;
pub use vmhd::VmhdFlags;

// Re-export entry structs
pub use co64::{Co64Entry, Co64EntryIter};
pub use ctts::{CttsEntry, CttsEntryIter};
pub use elst::ElstEntry;
pub use pdin::{PdinEntry, PdinEntryIter};
pub use sdtp::{SdtpEntry, SdtpEntryIter};
pub use stco::{StcoEntry, StcoEntryIter};
pub use stdp::{StdpEntry, StdpEntryIter};
pub use stsc::{StscEntry, StscEntryIter};
pub use stsh::{StshEntry, StshEntryIter};
pub use stss::{StssEntry, StssEntryIter};
pub use stsz::{StszEntry, StszEntryIter};
pub use stts::{SttsEntry, SttsEntryIter};
pub use stz2::{Stz2Entry, Stz2EntryIter};
pub use tfra::{TfraEntry, TfraEntryIter};
pub use trun::{TrunEntry, TrunEntryIter};

#[cfg(feature = "alloc")]
mod owned_exports {
    use super::*;

    // Variable-size boxes - Owned types
    pub use co64::Co64Box;
    pub use ctts::CttsBox;
    pub use dref::DrefBox;
    pub use dref::{DataEntryBox, UrlBox, UrnBox};
    pub use elng::ElngBox;
    pub use elst::ElstBox;
    pub use free::FreeBox;
    pub use ftyp::FtypBox;
    pub use hdlr::HdlrBox;
    pub use mdat::MdatBox;
    pub use pdin::PdinBox;
    pub use sdtp::SdtpBox;
    pub use stco::StcoBox;
    pub use stdp::StdpBox;
    pub use stsc::StscBox;
    pub use stsd::StsdBox;
    pub use stsh::StshBox;
    pub use stss::StssBox;
    pub use stsz::StszBox;
    pub use stts::SttsBox;
    pub use styp::StypBox;
    pub use stz2::Stz2Box;
    pub use tfra::TfraBox;
    pub use tref::{TrefBox, TrefTypeBox};
    pub use trgr::TrgrBox;
    pub use trun::TrunBox;

    // Container boxes - Owned types
    pub use dinf::DinfBox;
    pub use edts::EdtsBox;
    pub use mdia::MdiaBox;
    pub use mfra::MfraBox;
    pub use minf::{MediaHeaderBox, MinfBox};
    pub use moof::MoofBox;
    pub use moov::MoovBox;
    pub use mvex::MvexBox;
    pub use stbl::StblBox;
    pub use traf::TrafBox;
    pub use trak::TrakBox;

    pub use stbl::ChunkOffset;
    pub use stbl::SampleSize;
}

#[cfg(feature = "alloc")]
pub use owned_exports::*;

define_box_types!(
    // =========================================================================
    // ISO 14496-12 (BMFF) - File Structure and general boxes
    // =========================================================================

    /// File Type Box
    FTYP = b"ftyp",
    /// Media Data Box
    MDAT = b"mdat",
    /// Free Space Box
    FREE = b"free",
    /// Skip Box
    SKIP = b"skip",
    /// Progressive Download Information Box
    PDIN = b"pdin",

    // =========================================================================
    // ISO 14496-12 (BMFF) - Movie Structure
    // =========================================================================

    /// Movie Box
    MOOV = b"moov",
    /// Movie Header Box
    MVHD = b"mvhd",

    // =========================================================================
    // ISO 14496-12 (BMFF) - Track Structure
    // =========================================================================

    /// Track Box
    TRAK = b"trak",
    /// Track Header Box
    TKHD = b"tkhd",
    /// Track Reference Box
    TREF = b"tref",
    /// Track Group Box
    TRGR = b"trgr",

    // =========================================================================
    // ISO 14496-12 (BMFF) - Track Media Structure
    // =========================================================================

    /// Media Box
    MDIA = b"mdia",
    /// Media Header Box
    MDHD = b"mdhd",
    /// Handler Reference Box
    HDLR = b"hdlr",
    /// Media Information Box
    MINF = b"minf",
    /// Null Media Header Box
    NMHD = b"nmhd",
    /// Extended language tag Box
    ELNG = b"elng",

    // =========================================================================
    // ISO 14496-12 (BMFF) - Sample Tables
    // =========================================================================

    /// Sample Table Box
    STBL = b"stbl",
    /// Sample Description Box
    STSD = b"stsd",
    /// Degradation Priority Box
    STDP = b"stdp",

    // =========================================================================
    // ISO 14496-12 (BMFF) - Sample Tables
    // =========================================================================

    /// Decoding Time to Sample Box
    STTS = b"stts",
    /// Composition Time to Sample Box
    CTTS = b"ctts",
    /// Composition to Decode Box
    CSLG = b"cslg",
    /// Sync Sample Box
    STSS = b"stss",
    /// Shadow Sync Sample Box
    STSH = b"stsh",
    /// Independent and Disposable Samples Box
    SDTP = b"sdtp",
    /// Edit Box
    EDTS = b"edts",
    /// Edit List Box
    ELST = b"elst",

    // =========================================================================
    // ISO 14496-12 (BMFF) - Track Data Layout Structures
    // =========================================================================

    /// Data Information Box
    DINF = b"dinf",
    /// Data Reference Box
    DREF = b"dref",
    /// URL Box
    URL_ = b"url ",
    /// URN Box
    URN_ = b"urn ",
    /// Sample Size Box
    STSZ = b"stsz",
    /// Compact Sample Size Box
    STZ2 = b"stz2",
    /// Sample To Chunk Box
    STSC = b"stsc",
    /// Chunk Offset Box
    STCO = b"stco",
    /// Chunk Large Offset Box
    CO64 = b"co64",

    // =========================================================================
    // ISO 14496-12 (BMFF) - Movie Fragments
    // =========================================================================

    /// Movie Extends Box
    MVEX = b"mvex",
    /// Movie Extends Header Box
    MEHD = b"mehd",
    /// Track Extends Box
    TREX = b"trex",
    /// Movie Fragment Box
    MOOF = b"moof",
    /// Movie Fragment Header Box
    MFHD = b"mfhd",
    /// Track Fragment Box
    TRAF = b"traf",
    /// Track Fragment Header Box
    TFHD = b"tfhd",
    /// Track Run Box
    TRUN = b"trun",
    /// Movie Fragment Random Access Box
    MFRA = b"mfra",
    /// Track Fragment Random Access Box
    TFRA = b"tfra",
    /// Movie Fragment Random Access Offset Box
    MFRO = b"mfro",
    /// Track Fragment Decode Time
    TFDT = b"tfdt",

    // =========================================================================
    // ISO 14496-12 (BMFF) - Segments
    // =========================================================================

    /// Segment Type Box
    STYP = b"styp",

    // =========================================================================
    // ISO 14496-12 (BMFF) - Media-specific definitions
    // =========================================================================

    /// Video media header
    VMHD = b"vmhd",
    /// Sound media header
    SMHD = b"smhd",

    // =========================================================================
    // ISO 14496-12 (BMFF) - Hint media
    // =========================================================================

    /// Hint media header
    HMHD = b"hmhd",
);
