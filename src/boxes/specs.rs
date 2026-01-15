//! ISO Base Media File Format box specifications.
//!
//! This module provides parsers and types for specific box types defined in
//! ISO/IEC 14496-12 and related specifications.
//!
//! ## Box types, structure, and cross-reference
//! M* -> mandatory box
//! I* -> implemented
//! ```text
//! |                             |M|I|Description
//! +----+----+----+----+----+----+-+-+--------------------------------
//! |ftyp|    |    |    |    |    |*|○|file type and compatibility
//! |pdin|    |    |    |    |    | | |progressive download information
//! |moov|    |    |    |    |    |*|#|container for all the metadata
//! |    |mvhd|    |    |    |    |*|○|movie header, overall declarations
//! |    |trak|    |    |    |    |*|#|container for an individual track or stream
//! |    |    |tkhd|    |    |    |*|○|track header, overall information about the track
//! |    |    |tref|    |    |    | | |track reference container
//! |    |    |trgr|    |    |    | | |track grouping indication
//! |    |    |edts|    |    |    | | |edit list container
//! |    |    |    |elst|    |    | | |an edit list
//! |    |    |mdia|    |    |    |*|#|container for the media information in a track
//! |    |    |    |mdhd|    |    |*| |media header, overall information about the media
//! |    |    |    |hdlr|    |    |*| |handler, declares the media (handler) type
//! |    |    |    |minf|    |    |*| |media information container
//! |    |    |    |    |vmhd|    | | |video media header, overall information (video track only)
//! |    |    |    |    |smhd|    | | |sound media header, overall information (sound track only)
//! |    |    |    |    |hmhd|    | | |hint media header, overall information (hint track only)
//! |    |    |    |    |nmhd|    | | |Null media header, overall information (some tracks only)
//! |    |    |    |    |dinf|    |*|#|data information box, container
//! |    |    |    |    |    |dref|*|○|data reference box, declares source(s) of media data in track
//! |    |    |    |    |stbl|    |*| |sample table box, container for the time/space map
//! |    |    |    |    |    |stsd|*| |sample descriptions (codec types, initialization etc.)
//! |    |    |    |    |    |stts|*| |(decoding) time-to-sample
//! |    |    |    |    |    |ctts| | |(composition) time to sample
//! |    |    |    |    |    |cslg| | |composition to decode timeline mapping
//! |    |    |    |    |    |stsc|*| |sample-to-chunk, partial data-offset information
//! |    |    |    |    |    |stsz| | |sample sizes (framing)
//! |    |    |    |    |    |stz2| | |compact sample sizes (framing)
//! |    |    |    |    |    |stco|*| |chunk offset, partial data-offset information
//! |    |    |    |    |    |co64| | |64-bit chunk offset
//! |    |    |    |    |    |stss| | |sync sample table
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
//! |    |mvex|    |    |    |    | | |movie extends box
//! |    |    |mehd|    |    |    | | |movie extends header box
//! |    |    |trex|    |    |    |*| |track extends defaults
//! |    |    |leva|    |    |    | | |level assignment
//! |moof|    |    |    |    |    | | |movie fragment
//! |    |mfhd|    |    |    |    |*| |movie fragment header
//! |    |traf|    |    |    |    | | |track fragment
//! |    |    |tfhd|    |    |    |*| |track fragment header
//! |    |    |trun|    |    |    | | |track fragment run
//! |    |    |sbgp|    |    |    | | |sample-to-group
//! |    |    |sgpd|    |    |    | | |sample group description
//! |    |    |subs|    |    |    | | |sub-sample information
//! |    |    |saiz|    |    |    | | |sample auxiliary information sizes
//! |    |    |saio|    |    |    | | |sample auxiliary information offsets
//! |    |    |tfdt|    |    |    | | |track fragment decode time
//! |mfra|    |    |    |    |    | | |movie fragment random access
//! |    |tfra|    |    |    |    | | |track fragment random access
//! |    |mfro|    |    |    |    |*| |movie fragment random access offset
//! |mdat|    |    |    |    |    | | |media data container
//! |free|    |    |    |    |    | | |free space
//! |skip|    |    |    |    |    | | |free space
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
//! |styp|    |    |    |    |    | | |segment type
//! |sidx|    |    |    |    |    | | |segment index
//! |ssix|    |    |    |    |    | | |subsegment index
//! |prft|    |    |    |    |    | | |producer reference time
//! +----+----+----+----+----+----+-+-+--------------------------------
//! |stsd|    |    |    |    |    | | |sample descriptions
//! |    |avcc|    |    |    |    | |-|
//! |    |    |avc1|    |    |    | | |Advanced Video Coding
//! |    |hevx|    |    |    |    | |-|
//! |    |    |hev1|    |    |    | | |HEVC video with parameter sets in the Sample Entry or samples
//! |    |vpxx|    |    |    |    | |-|
//! |    |    |vp08|    |    |    | | |VP8 video
//! |    |    |vp09|    |    |    | | |VP9 video
//! |    |av1 |    |    |    |    | | |AV1 video
//! |    |opus|    |    |    |    | | |Opus audio coding
//! |    |mp4v|    |    |    |    | | |MPEG-4 Visual
//! |    |mp4a|    |    |    |    | | |MPEG-4 Audio
//! |    |mp4s|    |    |    |    | | |MPEG-4 System Stream
//! |    |pasp|    |    |    |    | | |Pixel Aspect Ratio
//! |    |btrt|    |    |    |    | | |Bitrate
//! +----+----+----+----+----+----+-+-+--------------------------------
//! |    |    |    |    |    |    | | |
//! ```

mod dinf;
mod dref;
mod free;
mod ftyp;
mod mdat;
mod moov;
mod mvhd;
mod tkhd;
mod trak;

pub use mvhd::{
    MvhdBox, //
    MvhdFlags,
    MvhdSpec,
};
pub use tkhd::{
    TkhdBox, //
    TkhdFlags,
    TkhdSpec,
};

pub use dinf::DinfBoxRef;
pub use dref::{
    DrefBoxRef, //
    DrefEntryRef,
};
pub use dref::{
    UrlBoxRef, //
    UrlFlags,
    UrlSpec,
};
pub use dref::{
    UrnBoxRef, //
    UrnFlags,
    UrnSpec,
};
pub use free::FreeBoxRef;
pub use ftyp::FtypBoxRef;
pub use mdat::MdatBoxRef;
pub use moov::MoovBoxRef;
pub use trak::TrakBoxRef;

#[cfg(feature = "alloc")]
mod owned {
    pub use super::dinf::DinfBox;
    pub use super::dref::{
        DrefBox, //
        DrefEntry,
        UrlBox,
        UrnBox,
    };
    pub use super::ftyp::FtypBox;
    pub use super::moov::MoovBox;
    pub use super::trak::TrakBox;
}

#[cfg(feature = "alloc")]
pub use owned::*;
