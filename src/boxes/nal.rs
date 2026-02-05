//! ISO/IEC 14496-15 Carriage of NAL unit structured video

#[cfg(feature = "avc")]
pub mod avc;

define_box_types!(
    /// AVC Configuration Box
    AVCC = b"avcC",
    /// MPEG-4 Extension Descriptors Box
    M4DS = b"m4ds",
    /// AVC Sample Entry Box
    AVC1 = b"avc1",
    /// AVC3 Sample Entry Box
    AVC3 = b"avc3",
    /// AVC2 Sample Entry Box
    AVC2 = b"avc2",
    /// AVC4 Sample Entry Box
    AVC4 = b"avc4",
);
