/// Descriptor Tag (ISO/IEC 14496-1 Table 1 - List of Class Tags for Descriptors)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tag(pub(crate) u8);

impl Tag {
    /// Descriptor Tag for Elementary Stream Descriptor
    pub const ES_DESCR_TAG: Tag = Tag(0x03);
    /// Descriptor Tag for Decoder Config Descriptor
    pub const DECODER_CONFIG_DESCR_TAG: Tag = Tag(0x04);
    /// Descriptor Tag for Decoder Specific Info
    pub const DECODER_SPECIFIC_INFO_TAG: Tag = Tag(0x05);
    /// Descriptor Tag for SL Config Descriptor
    pub const SL_CONFIG_DESCR_TAG: Tag = Tag(0x06);
    /// Descriptor Tag for Extension Profile Level Descriptor
    pub const EXTENSION_PROFILE_LEVEL_DESCR_TAG: Tag = Tag(0x13);
    /// Descriptor Tag for Profile Level Indication Index Descriptor
    pub const PROFILE_LEVEL_INDICATION_INDEX_DESCR_TAG: Tag = Tag(0x14);
}
