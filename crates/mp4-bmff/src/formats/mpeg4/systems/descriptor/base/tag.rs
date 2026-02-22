//! Descriptor tag type for MPEG-4 Systems descriptors.

/// Descriptor tag identifying the type of an MPEG-4 Systems descriptor.
///
/// Each descriptor in MPEG-4 Systems is identified by a single-byte tag
/// that indicates the descriptor type and determines how the instance
/// data should be parsed.
///
/// # Predefined Tags
///
/// | Constant | Value | Description |
/// |----------|-------|-------------|
/// | `ES_DESCR_TAG` | 0x03 | Elementary Stream Descriptor |
/// | `DECODER_CONFIG_DESCR_TAG` | 0x04 | Decoder Configuration Descriptor |
/// | `DECODER_SPECIFIC_INFO_TAG` | 0x05 | Decoder Specific Info |
/// | `SL_CONFIG_DESCR_TAG` | 0x06 | Sync Layer Configuration |
/// | `EXTENSION_PROFILE_LEVEL_DESCR_TAG` | 0x13 | Extension Profile Level |
/// | `PROFILE_LEVEL_INDICATION_INDEX_DESCR_TAG` | 0x14 | Profile Level Index |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tag(pub(crate) u8);

impl Tag {
    /// Elementary Stream Descriptor tag (0x03).
    pub const ES_DESCR_TAG: Tag = Tag(0x03);
    /// Decoder Configuration Descriptor tag (0x04).
    pub const DECODER_CONFIG_DESCR_TAG: Tag = Tag(0x04);
    /// Decoder Specific Info tag (0x05).
    pub const DECODER_SPECIFIC_INFO_TAG: Tag = Tag(0x05);
    /// Sync Layer Configuration Descriptor tag (0x06).
    pub const SL_CONFIG_DESCR_TAG: Tag = Tag(0x06);
    /// Extension Profile Level Descriptor tag (0x13).
    pub const EXTENSION_PROFILE_LEVEL_DESCR_TAG: Tag = Tag(0x13);
    /// Profile Level Indication Index Descriptor tag (0x14).
    pub const PROFILE_LEVEL_INDICATION_INDEX_DESCR_TAG: Tag = Tag(0x14);
}
