//! ISO/IEC 14496-12 Box Structures

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

// Variable-size boxes - View types
pub use ctts::CttsBoxView;
pub use dref::{
    DataEntryBoxView, //
    DrefBoxView,
    UrlBoxView,
    UrnBoxView,
};
pub use elng::ElngBoxView;
pub use elst::ElstBoxView;
pub use free::FreeBoxView;
pub use ftyp::FtypBoxView;
pub use hdlr::HdlrBoxView;
pub use mdat::MdatBoxView;
pub use mvex::MvexBoxView;
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
pub use tfra::TfraBoxView;
pub use tref::{
    TrefBoxView, //
    TrefTypeBoxView,
};
pub use trgr::TrgrBoxView;
pub use trun::TrunBoxView;

// Container boxes - View types
pub use dinf::DinfBoxView;
pub use edts::EdtsBoxView;
pub use mdia::MdiaBoxView;
pub use mfra::MfraBoxView;
pub use minf::MinfBoxView;
pub use moof::MoofBoxView;
pub use moov::MoovBoxView;
pub use stbl::StblBoxView;
pub use traf::TrafBoxView;
pub use trak::TrakBoxView;

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
pub use cslg::CslgFlags;
pub use ctts::CttsFlags;
pub use dref::{
    DrefFlags, //
    UrlFlags,
    UrnFlags,
};
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
pub use tfdt::TfdtFlags;
pub use tfhd::TfhdFlags;
pub use tfra::TfraFlags;
pub use tkhd::TkhdFlags;
pub use trex::TrexFlags;
pub use trun::TrunFlags;
pub use vmhd::VmhdFlags;

// Re-export entry structs
pub use ctts::CttsEntry;
pub use elst::ElstEntry;
pub use pdin::PdinEntry;
pub use sdtp::SdtpEntry;
pub use stco::StcoEntry;
pub use stdp::StdpEntry;
pub use stsc::StscEntry;
pub use stsh::StshEntry;
pub use stss::StssEntry;
pub use stsz::StszEntry;
pub use stts::SttsEntry;
pub use tfra::{
    TfraEntry, //
    TfraEntryIter,
};

pub use trun::{
    TrunSample, //
    TrunSampleIter,
};

#[cfg(feature = "alloc")]
mod owned_exports {
    use super::*;

    // Variable-size boxes - Owned types
    pub use ctts::CttsBox;
    pub use dref::{
        DataEntryBox, //
        DrefBox,
        UrlBox,
        UrnBox,
    };
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
    pub use tfra::TfraBox;
    pub use tref::{
        TrefBox, //
        TrefTypeBox,
    };
    pub use trgr::TrgrBox;
    pub use trun::TrunBox;

    // Container boxes - Owned types
    pub use dinf::DinfBox;
    pub use edts::EdtsBox;
    pub use mdia::MdiaBox;
    pub use mfra::MfraBox;
    pub use minf::{
        MediaHeaderBox, //
        MinfBox,
    };
    pub use moof::MoofBox;
    pub use moov::MoovBox;
    pub use mvex::MvexBox;
    pub use stbl::StblBox;
    pub use traf::TrafBox;
    pub use trak::TrakBox;
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
