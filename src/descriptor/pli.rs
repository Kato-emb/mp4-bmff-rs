use crate::cursor::ReadCursor;

use crate::error::*;

/// Extension Profile Level Descriptor (7.2.6.19 Extension Profile Level Descriptor)
#[derive(Debug, Clone, Copy)]
pub struct ExtensionProfileLevelDescriptor {
    /// Profile Level Indication Index
    pub profile_level_indication_index: u8,
    /// Object Descriptor Profile Level Indication
    pub od_profile_level_indication: u8,
    /// Scene Profile Level Indication
    pub scene_profile_level_indication: u8,
    /// Audio Profile Level Indication
    pub audio_profile_level_indication: u8,
    /// Visual Profile Level Indication
    pub visual_profile_level_indication: u8,
    /// Graphics Profile Level Indication
    pub graphics_profile_level_indication: u8,
    /// MPEG-J Profile Level Indication
    pub mpegj_profile_level_indication: u8,
    /// Text Profile Level Indication
    pub text_profile_level_indication: u8,
    /// 3D-Graphics Profile Level Indication
    pub _3dc_profile_level_indication: u8,
}

impl ExtensionProfileLevelDescriptor {
    pub(crate) fn parse_in(cur: &mut ReadCursor<'_>) -> Result<Self> {
        let profile_level_indication_index = cur.read_u8()?;
        let od_profile_level_indication = cur.read_u8()?;
        let scene_profile_level_indication = cur.read_u8()?;
        let audio_profile_level_indication = cur.read_u8()?;
        let visual_profile_level_indication = cur.read_u8()?;
        let graphics_profile_level_indication = cur.read_u8()?;
        let mpegj_profile_level_indication = cur.read_u8()?;
        let text_profile_level_indication = cur.read_u8()?;
        let _3dc_profile_level_indication = cur.read_u8()?;

        Ok(ExtensionProfileLevelDescriptor {
            profile_level_indication_index,
            od_profile_level_indication,
            scene_profile_level_indication,
            audio_profile_level_indication,
            visual_profile_level_indication,
            graphics_profile_level_indication,
            mpegj_profile_level_indication,
            text_profile_level_indication,
            _3dc_profile_level_indication,
        })
    }

    /// Parses ExtensionProfileLevelDescriptor from a byte slice
    pub fn parse(instance: &[u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(instance);
        Self::parse_in(&mut cur)
    }
}

/// Profile Level Indication Index Descriptor (7.2.6.20 Profile Level Indication Index Descriptor)
#[derive(Debug, Clone, Copy)]
pub struct ProfileLevelIndicationIndexDescriptor {
    /// Profile Level Indication Index
    pub profile_level_indication_index: u8,
}

impl ProfileLevelIndicationIndexDescriptor {
    pub(crate) fn parse_in(cur: &mut ReadCursor<'_>) -> Result<Self> {
        let profile_level_indication_index = cur.read_u8()?;

        Ok(ProfileLevelIndicationIndexDescriptor {
            profile_level_indication_index,
        })
    }

    /// Parses ProfileLevelIndicationIndexDescriptor from a byte slice
    pub fn parse(instance: &[u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(instance);
        Self::parse_in(&mut cur)
    }
}
