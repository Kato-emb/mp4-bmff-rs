use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;

use super::FullBoxFlags;

/// Composition to Decode Timeline Mapping Box (`cslg`).
///
/// This box provides the offset shift between composition time and decode time.
#[derive(Debug, Clone, Copy)]
pub struct CslgBox {
    /// The version of the box (0 or 1).
    pub version: u8,
    /// The flags of the box.
    pub flags: CslgFlags,
    /// The shift to be applied to the composition time to get decode time.
    pub composition_to_dts_shift: i64,
    /// The smallest composition offset in the sample table.
    pub least_decode_to_display_delta: i64,
    /// The largest composition offset in the sample table.
    pub greatest_decode_to_display_delta: i64,
    /// The smallest composition time of any sample.
    pub composition_start_time: i64,
    /// The largest composition time plus duration of any sample.
    pub composition_end_time: i64,
}

impl BoxCodec for CslgBox {
    fn boxtype(&self) -> BoxType {
        BoxType::CSLG
    }
}

impl BoxDecode<'_> for CslgBox {
    fn decode(bytes: &[u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = CslgFlags::from_bytes(cur.read_array()?);

        let (
            composition_to_dts_shift,
            least_decode_to_display_delta,
            greatest_decode_to_display_delta,
            composition_start_time,
            composition_end_time,
        ) = match version {
            0 => (
                cur.read_i32_be()? as i64,
                cur.read_i32_be()? as i64,
                cur.read_i32_be()? as i64,
                cur.read_i32_be()? as i64,
                cur.read_i32_be()? as i64,
            ),
            1 => (
                cur.read_i64_be()?,
                cur.read_i64_be()?,
                cur.read_i64_be()?,
                cur.read_i64_be()?,
                cur.read_i64_be()?,
            ),
            v => {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxVersion {
                        reason: "cslg version must be 0 or 1",
                        got: v,
                    },
                    BoxType::CSLG,
                ));
            }
        };

        Ok(CslgBox {
            version,
            flags,
            composition_to_dts_shift,
            least_decode_to_display_delta,
            greatest_decode_to_display_delta,
            composition_start_time,
            composition_end_time,
        })
    }
}

impl TryFrom<&[u8]> for CslgBox {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self> {
        CslgBox::decode(value)
    }
}

impl BoxEncode for CslgBox {
    fn encode(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

        cur.write_u8(self.version)?;
        cur.write_array(&self.flags.to_bytes())?;

        if self.version == 0 {
            cur.write_i32_be(self.composition_to_dts_shift as i32)?;
            cur.write_i32_be(self.least_decode_to_display_delta as i32)?;
            cur.write_i32_be(self.greatest_decode_to_display_delta as i32)?;
            cur.write_i32_be(self.composition_start_time as i32)?;
            cur.write_i32_be(self.composition_end_time as i32)?;
        } else {
            cur.write_u64_be(self.composition_to_dts_shift as u64)?;
            cur.write_u64_be(self.least_decode_to_display_delta as u64)?;
            cur.write_u64_be(self.greatest_decode_to_display_delta as u64)?;
            cur.write_u64_be(self.composition_start_time as u64)?;
            cur.write_u64_be(self.composition_end_time as u64)?;
        }

        Ok(cur.position())
    }
}

/// Specification for the Composition to Decode Timeline Mapping Box (`cslg`).
pub struct CslgSpec;

/// Flags for the Composition to Decode Timeline Mapping Box (`cslg`).
pub type CslgFlags = FullBoxFlags<CslgSpec>;

#[cfg(test)]
mod tests {
    use crate::RawBoxRef;

    use super::*;

    fn make_full_box_header(version: u8, flags: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(version);
        data.extend_from_slice(&flags.to_be_bytes()[1..4]);
        data
    }

    fn make_cslg_payload_v0(
        composition_to_dts_shift: i32,
        least_decode_to_display_delta: i32,
        greatest_decode_to_display_delta: i32,
        composition_start_time: i32,
        composition_end_time: i32,
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(0, 0));
        payload.extend_from_slice(&composition_to_dts_shift.to_be_bytes());
        payload.extend_from_slice(&least_decode_to_display_delta.to_be_bytes());
        payload.extend_from_slice(&greatest_decode_to_display_delta.to_be_bytes());
        payload.extend_from_slice(&composition_start_time.to_be_bytes());
        payload.extend_from_slice(&composition_end_time.to_be_bytes());
        payload
    }

    fn make_cslg_payload_v1(
        composition_to_dts_shift: i64,
        least_decode_to_display_delta: i64,
        greatest_decode_to_display_delta: i64,
        composition_start_time: i64,
        composition_end_time: i64,
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_full_box_header(1, 0));
        payload.extend_from_slice(&composition_to_dts_shift.to_be_bytes());
        payload.extend_from_slice(&least_decode_to_display_delta.to_be_bytes());
        payload.extend_from_slice(&greatest_decode_to_display_delta.to_be_bytes());
        payload.extend_from_slice(&composition_start_time.to_be_bytes());
        payload.extend_from_slice(&composition_end_time.to_be_bytes());
        payload
    }

    #[test]
    fn parse_cslg_v0() {
        let payload = make_cslg_payload_v0(100, -50, 200, 0, 10000);
        let cslg = CslgBox::decode(&payload).unwrap();

        assert_eq!(cslg.version, 0);
        assert_eq!(cslg.composition_to_dts_shift, 100);
        assert_eq!(cslg.least_decode_to_display_delta, -50);
        assert_eq!(cslg.greatest_decode_to_display_delta, 200);
        assert_eq!(cslg.composition_start_time, 0);
        assert_eq!(cslg.composition_end_time, 10000);
    }

    #[test]
    fn parse_cslg_v0_negative_values() {
        let payload = make_cslg_payload_v0(-100, -200, -50, -1000, 5000);
        let cslg = CslgBox::decode(&payload).unwrap();

        assert_eq!(cslg.version, 0);
        assert_eq!(cslg.composition_to_dts_shift, -100);
        assert_eq!(cslg.least_decode_to_display_delta, -200);
        assert_eq!(cslg.greatest_decode_to_display_delta, -50);
        assert_eq!(cslg.composition_start_time, -1000);
        assert_eq!(cslg.composition_end_time, 5000);
    }

    #[test]
    fn parse_cslg_v1() {
        let payload = make_cslg_payload_v1(
            0x1_0000_0000,
            -0x1_0000_0000,
            0x2_0000_0000,
            0,
            0x3_0000_0000,
        );
        let cslg = CslgBox::decode(&payload).unwrap();

        assert_eq!(cslg.version, 1);
        assert_eq!(cslg.composition_to_dts_shift, 0x1_0000_0000);
        assert_eq!(cslg.least_decode_to_display_delta, -0x1_0000_0000);
        assert_eq!(cslg.greatest_decode_to_display_delta, 0x2_0000_0000);
        assert_eq!(cslg.composition_start_time, 0);
        assert_eq!(cslg.composition_end_time, 0x3_0000_0000);
    }

    #[test]
    fn parse_cslg_v0_truncated() {
        let mut payload = make_full_box_header(0, 0);
        payload.extend_from_slice(&100i32.to_be_bytes());
        // Missing other fields

        let result = CslgBox::decode(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn try_from_byte_slice() {
        let payload = make_cslg_payload_v0(50, 10, 100, 0, 5000);
        let cslg = CslgBox::try_from(payload.as_slice()).unwrap();

        assert_eq!(cslg.composition_to_dts_shift, 50);
    }

    #[test]
    fn try_from_box_view_success() {
        let payload = make_cslg_payload_v0(50, 10, 100, 0, 5000);

        let mut box_data = Vec::new();
        let size = 8 + payload.len() as u32;
        box_data.extend_from_slice(&size.to_be_bytes());
        box_data.extend_from_slice(b"cslg");
        box_data.extend_from_slice(&payload);

        let raw = RawBoxRef::parse(&box_data).unwrap();
        let cslg = CslgBox::decode(raw.payload()).unwrap();

        assert_eq!(cslg.composition_to_dts_shift, 50);
    }
}
