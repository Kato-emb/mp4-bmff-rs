//! BMFF box core infrastructure.
//!
//! This module provides the core types for parsing and writing BMFF boxes.
//! All types in this module are designed to work in `no_std` and `no_alloc`
//! environments, using zero-copy parsing with borrowed data.
//!
//! ## Box types, structure, and cross-reference
//! M* -> mandatory box
//! I* -> implemented
//! ```text
//! |                             |M|I|Description
//! +----+----+----+----+----+----+-+-+--------------------------------
//! |ftyp|    |    |    |    |    |*|○|file type and compatibility
//! |pdin|    |    |    |    |    | | |progressive download information
//! |moov|    |    |    |    |    |*|○|container for all the metadata
//! |    |mvhd|    |    |    |    |*|○|movie header, overall declarations
//! |    |trak|    |    |    |    |*|○|container for an individual track or stream
//! |    |    |tkhd|    |    |    |*|○|track header, overall information about the track
//! |    |    |tref|    |    |    | |○|track reference container
//! |    |    |trgr|    |    |    | | |track grouping indication
//! |    |    |edts|    |    |    | | |edit list container
//! |    |    |    |elst|    |    | | |an edit list
//! |    |    |mdia|    |    |    |*|○|container for the media information in a track
//! |    |    |    |mdhd|    |    |*|○|media header, overall information about the media
//! |    |    |    |hdlr|    |    |*|○|handler, declares the media (handler) type
//! |    |    |    |minf|    |    |*|○|media information container
//! |    |    |    |    |vmhd|    | |○|video media header, overall information (video track only)
//! |    |    |    |    |smhd|    | |○|sound media header, overall information (sound track only)
//! |    |    |    |    |hmhd|    | |○|hint media header, overall information (hint track only)
//! |    |    |    |    |nmhd|    | |○|Null media header, overall information (some tracks only)
//! |    |    |    |    |dinf|    |*|○|data information box, container
//! |    |    |    |    |    |dref|*|○|data reference box, declares source(s) of media data in track
//! |    |    |    |    |stbl|    |*|○|sample table box, container for the time/space map
//! |    |    |    |    |    |stsd|*|○|sample descriptions (codec types, initialization etc.)
//! |    |    |    |    |    |stts|*|○|(decoding) time-to-sample
//! |    |    |    |    |    |ctts| |○|(composition) time to sample
//! |    |    |    |    |    |cslg| |○|composition to decode timeline mapping
//! |    |    |    |    |    |stsc|*|○|sample-to-chunk, partial data-offset information
//! |    |    |    |    |    |stsz| |○|sample sizes (framing)
//! |    |    |    |    |    |stz2| | |compact sample sizes (framing)
//! |    |    |    |    |    |stco|*|○|chunk offset, partial data-offset information
//! |    |    |    |    |    |co64| |○|64-bit chunk offset
//! |    |    |    |    |    |stss| |○|sync sample table
//! |    |    |    |    |    |stsh| | |shadow sync sample table
//! |    |    |    |    |    |padb| | |sample padding bits
//! |    |    |    |    |    |stdp| | |sample degradation priority
//! |    |    |    |    |    |sdtp| | |independent and disposable samples
//! |    |    |    |    |    |sbgp| | |sample-to-group
//! |    |    |    |    |    |sgpd| | |sample group description
//! |    |    |    |    |    |subs| | |sub-sample information
//! |    |    |    |    |    |saiz| | |sample auxiliary information sizes
//! |    |    |    |    |    |saio| | |sample auxiliary information offsets
//! |    |    |    |elng|    |    | | |Extended Language Tag
//! |    |    |udta|    |    |    | | |user-data
//! |    |mvex|    |    |    |    | |○|movie extends box
//! |    |    |mehd|    |    |    | |○|movie extends header box
//! |    |    |trex|    |    |    |*|○|track extends defaults
//! |    |    |leva|    |    |    | | |level assignment
//! |moof|    |    |    |    |    | |○|movie fragment
//! |    |mfhd|    |    |    |    |*|○|movie fragment header
//! |    |traf|    |    |    |    | |○|track fragment
//! |    |    |tfhd|    |    |    |*|○|track fragment header
//! |    |    |trun|    |    |    | |○|track fragment run
//! |    |    |sbgp|    |    |    | |○|sample-to-group
//! |    |    |sgpd|    |    |    | | |sample group description
//! |    |    |subs|    |    |    | | |sub-sample information
//! |    |    |saiz|    |    |    | | |sample auxiliary information sizes
//! |    |    |saio|    |    |    | | |sample auxiliary information offsets
//! |    |    |tfdt|    |    |    | |○|track fragment decode time
//! |mfra|    |    |    |    |    | |○|movie fragment random access
//! |    |tfra|    |    |    |    | |○|track fragment random access
//! |    |mfro|    |    |    |    |*|○|movie fragment random access offset
//! |mdat|    |    |    |    |    | |○|media data container
//! |free|    |    |    |    |    | |○|free space
//! |skip|    |    |    |    |    | |○|free space
//! |    |udta|    |    |    |    | | |user-data
//! |    |    |cprt|    |    |    | | |copyright etc.
//! |    |    |tsel|    |    |    | | |track selection box
//! |    |    |strk|    |    |    | | |sub track box
//! |    |    |    |stri|    |    | | |sub track information box
//! |    |    |    |strd|    |    | | |sub track definition box
//! |meta|    |    |    |    |    | | |metadata
//! |    |hdlr|    |    |    |    |*| |handler, declares the metadata (handler) type
//! |    |dinf|    |    |    |    | | |data information box, container
//! |    |    |dref|    |    |    | | |data reference box, declares source(s) of metadata items
//! |    |iloc|    |    |    |    | | |item location
//! |    |ipro|    |    |    |    | | |item protection
//! |    |    |sinf|    |    |    | | |protection scheme information box
//! |    |    |    |frma|    |    | | |original format box
//! |    |    |    |schm|    |    | | |scheme type box
//! |    |    |    |schi|    |    | | |scheme information box
//! |    |iinf|    |    |    |    | | |item information
//! |    |xml |    |    |    |    | | |XML container
//! |    |bxml|    |    |    |    | | |binary XML container
//! |    |pitm|    |    |    |    | | |primary item reference
//! |    |fiin|    |    |    |    | | |file delivery item information
//! |    |    |paen|    |    |    | | |partition entry
//! |    |    |    |fire|    |    | | |file reservoir
//! |    |    |    |fpar|    |    | | |file partition
//! |    |    |    |fecr|    |    | | |FEC reservoir
//! |    |    |segr|    |    |    | | |file delivery session group
//! |    |    |gitn|    |    |    | | |group id to name
//! |    |idat|    |    |    |    | | |item data
//! |    |iref|    |    |    |    | | |item reference
//! |meco|    |    |    |    |    | | |additional metadata container
//! |    |mere|    |    |    |    | | |metabox relation
//! |styp|    |    |    |    |    | |○|segment type
//! |sidx|    |    |    |    |    | | |segment index
//! |ssix|    |    |    |    |    | | |subsegment index
//! |prft|    |    |    |    |    | | |producer reference time
//! +----+----+----+----+----+----+-+-+--------------------------------
//! |stsd|    |    |    |    |    | |○|sample descriptions
//! |    |hint|    |    |    |    | | |hint track
//! |    |btrt|    |    |    |    | | |Bitrate
//! |    |metx|    |    |    |    | | |XML metadata
//! |    |mett|    |    |    |    | | |Text metadata
//! |    |uri |    |    |    |    | | |URI
//! |    |uriI|    |    |    |    | | |URI initialization
//! |    |urim|    |    |    |    | | |URI metadata
//! |    |    |    |    |    |    | | |URI metadata
//! |    |pasp|    |    |    |    | | |Pixel Aspect Ratio
//! |    |clap|    |    |    |    | | |Clean Aperture
//! |    |avcc|    |    |    |    | |-|ISO/IEC 14496-15 AVC Configuration Box
//! |    |    |avc1|    |    |    | |○|
//! |    |    |avc2|    |    |    | | |
//! |    |    |avc3|    |    |    | |○|
//! |    |    |avc4|    |    |    | | |
//! |    |hevx|    |    |    |    | |-|
//! |    |    |hev1|    |    |    | | |HEVC video with parameter sets in the Sample Entry or samples
//! |    |vpxx|    |    |    |    | |-|
//! |    |    |vp08|    |    |    | | |VP8 video
//! |    |    |vp09|    |    |    | | |VP9 video
//! |    |av1 |    |    |    |    | | |AV1 video
//! |    |opus|    |    |    |    | | |Opus audio coding
//! |    |mp4v|    |    |    |    | | |MPEG-4 Visual
//! |    |mp4a|    |    |    |    | |○|MPEG-4 Audio
//! |    |mp4s|    |    |    |    | | |MPEG-4 System Stream
//! +----+----+----+----+----+----+-+-+--------------------------------
//! |    |    |    |    |    |    | | |
//! ```

mod avcc;
mod co64;
mod cslg;
mod ctts;
mod dinf;
mod dref;
mod esds;
mod free;
mod ftyp;
mod hdlr;
mod hmhd;
mod mdat;
mod mdhd;
mod mdia;
mod mehd;
mod mfhd;
mod mfra;
mod mfro;
mod minf;
mod moof;
mod moov;
mod mp4a;
mod mvex;
mod mvhd;
mod nmhd;
mod sbgp;
mod smhd;
mod stbl;
mod stco;
mod stsc;
mod stsd;
mod stss;
mod stsz;
mod stts;
mod styp;
mod tfdt;
mod tfhd;
mod tfra;
mod tkhd;
mod traf;
mod trak;
mod tref;
mod trex;
mod trun;
mod vmhd;

mod fullbox;
mod sample_entry;

pub use fullbox::FullBoxFlags;

// Variable-size boxes - View types
pub use co64::Co64BoxView;
pub use ctts::CttsBoxView;
pub use dref::DrefBoxView;
pub use dref::UrlBoxView;
pub use dref::UrnBoxView;
pub use esds::EsdsBoxView;
pub use free::FreeBoxView;
pub use ftyp::FtypBoxView;
pub use hdlr::HdlrBoxView;
pub use mdat::MdatBoxView;
pub use mp4a::Mp4aBoxView;
pub use sbgp::SbgpBoxView;
pub use stco::StcoBoxView;
pub use stsc::StscBoxView;
pub use stsd::StsdBoxView;
pub use stss::StssBoxView;
pub use stsz::StszBoxView;
pub use stts::SttsBoxView;
pub use styp::StypBoxView;
pub use tfra::TfraBoxView;
pub use tfra::TfraEntryIter;
pub use trun::TrunBoxView;
pub use trun::TrunSampleIter;

// Container boxes - View types
pub use dinf::DinfBoxView;
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

// Re-export tref related types
pub use tref::TrackReferenceTypeBoxView;

// Fixed-size boxes (Copy types, no View/Owned distinction)
pub use cslg::CslgBox;
pub use hmhd::HmhdBox;
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
pub use vmhd::VmhdBox;

// Re-export entry structs
pub use co64::Co64Entry;
pub use ctts::CttsEntry;
pub use sbgp::SbgpEntry;
pub use stco::StcoEntry;
pub use stsc::StscEntry;
pub use stss::StssEntry;
pub use stts::SttsEntry;
pub use tfra::TfraEntry;
pub use trun::TrunSample;

// Re-export entry views
pub use dref::DrefEntryView;
pub use stsd::StsdEntryView;

// Re-export box utils
pub use minf::MediaHeader;

// Re-export fullbox flags and specs
pub use co64::{Co64Flags, Co64Spec};
pub use cslg::{CslgFlags, CslgSpec};
pub use ctts::{CttsFlags, CttsSpec};
pub use dref::{DrefFlags, DrefSpec};
pub use dref::{UrlFlags, UrlSpec};
pub use dref::{UrnFlags, UrnSpec};
pub use esds::{EsdsFlags, EsdsSpec};
pub use hdlr::{HdlrFlags, HdlrSpec};
pub use hmhd::{HmhdFlags, HmhdSpec};
pub use mdhd::{MdhdFlags, MdhdSpec};
pub use mehd::{MehdFlags, MehdSpec};
pub use mfhd::{MfhdFlags, MfhdSpec};
pub use mfro::{MfroFlags, MfroSpec};
pub use mvhd::{MvhdFlags, MvhdSpec};
pub use nmhd::{NmhdFlags, NmhdSpec};
pub use sbgp::{SbgpFlags, SbgpSpec};
pub use smhd::{SmhdFlags, SmhdSpec};
pub use stco::{StcoFlags, StcoSpec};
pub use stsc::{StscFlags, StscSpec};
pub use stsd::{StsdFlags, StsdSpec};
pub use stss::{StssFlags, StssSpec};
pub use stsz::{StszFlags, StszSpec};
pub use stts::{SttsFlags, SttsSpec};
pub use tfdt::{TfdtFlags, TfdtSpec};
pub use tfhd::{TfhdFlags, TfhdSpec};
pub use tfra::{TfraFlags, TfraSpec};
pub use tkhd::{TkhdFlags, TkhdSpec};
pub use trex::{TrexFlags, TrexSpec};
pub use trun::{TrunFlags, TrunSpec};
pub use vmhd::{VmhdFlags, VmhdSpec};

// Re-export sample entry
pub use sample_entry::AudioSampleEntry;
pub use sample_entry::SampleEntry;
pub use sample_entry::VisualSampleEntry;

// Re-export avcc box and related types
pub use avcc::Avc1BoxView;
pub use avcc::Avc3BoxView;
pub use avcc::AvcCBoxView;
pub use avcc::NalUnitIter;

#[cfg(feature = "alloc")]
mod owned_exports {
    use super::*;

    // Variable-size boxes - Owned types
    pub use co64::Co64Box;
    pub use ctts::CttsBox;
    pub use dref::DrefBox;
    pub use dref::UrlBox;
    pub use dref::UrnBox;
    pub use esds::EsdsBox;
    pub use free::FreeBox;
    pub use ftyp::FtypBox;
    pub use hdlr::HdlrBox;
    pub use mdat::MdatBox;
    pub use mp4a::Mp4aBox;
    pub use sbgp::SbgpBox;
    pub use stco::StcoBox;
    pub use stsc::StscBox;
    pub use stsd::StsdBox;
    pub use stss::StssBox;
    pub use stsz::StszBox;
    pub use stts::SttsBox;
    pub use styp::StypBox;
    pub use tfra::TfraBox;
    pub use trun::TrunBox;

    // Re-export stsz SampleSizes enum
    pub use stsz::SampleSizes;

    // Container boxes - Owned types
    pub use dinf::DinfBox;
    pub use mdia::MdiaBox;
    pub use mfra::MfraBox;
    pub use minf::MinfBox;
    pub use moof::MoofBox;
    pub use moov::MoovBox;
    pub use mvex::MvexBox;
    pub use stbl::StblBox;
    pub use traf::TrafBox;
    pub use trak::TrakBox;
    pub use tref::TrackReferenceTypeBox;
    pub use tref::TrefBox;

    // Re-export entry owned types
    pub use dref::DrefEntry;
    pub use stsd::StsdEntry;

    // Re-export avcc owned types
    pub use avcc::Avc1Box;
    pub use avcc::Avc3Box;
    pub use avcc::AvcCBox;
}

#[cfg(feature = "alloc")]
pub use owned_exports::*;
