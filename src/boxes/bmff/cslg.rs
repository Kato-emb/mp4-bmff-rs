use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

define_box_flags!(
    /// Flags for the Composition to Decode Box (`cslg`).
    CslgFlags {}
);

/// Composition to Decode Box (`cslg`).
#[derive(Debug, Clone, Copy)]
pub struct CslgBox {
    /// Box version (0 or 1).
    pub version: u8,
    /// Box flags (should be 0).
    pub flags: CslgFlags,
    /// The composition to decode shift.
    pub composition_to_decode_shift: i64,
    /// The least decode to display delta.
    pub least_decode_to_display_delta: i64,
    /// The greatest decode to display delta.
    pub greatest_decode_to_display_delta: i64,
    /// The composition start time.
    pub composition_start_time: i64,
    /// The composition end time.
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

        // Read version (1 byte)
        let version = cur.read_u8()?;

        // Read flags (3 bytes)
        let flags = CslgFlags::from_be_bytes(cur.read_array::<3>()?);

        let composition_to_decode_shift: i64;
        let least_decode_to_display_delta: i64;
        let greatest_decode_to_display_delta: i64;
        let composition_start_time: i64;
        let composition_end_time: i64;

        match version {
            0 => {
                composition_to_decode_shift = cur.read_i32_be()? as i64;
                least_decode_to_display_delta = cur.read_i32_be()? as i64;
                greatest_decode_to_display_delta = cur.read_i32_be()? as i64;
                composition_start_time = cur.read_i32_be()? as i64;
                composition_end_time = cur.read_i32_be()? as i64;
            }
            1 => {
                composition_to_decode_shift = cur.read_i64_be()?;
                least_decode_to_display_delta = cur.read_i64_be()?;
                greatest_decode_to_display_delta = cur.read_i64_be()?;
                composition_start_time = cur.read_i64_be()?;
                composition_end_time = cur.read_i64_be()?;
            }
            _ => {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxVersion {
                        reason: "0 or 1 in this specification",
                        got: version,
                    },
                    BoxType::CSLG,
                ));
            }
        }

        Ok(CslgBox {
            version,
            flags,
            composition_to_decode_shift,
            least_decode_to_display_delta,
            greatest_decode_to_display_delta,
            composition_start_time,
            composition_end_time,
        })
    }
}

impl BoxEncode for CslgBox {
    #[inline]
    fn encoded_len(&self) -> usize {
        4 + if self.version == 0 { 5 * 4 } else { 5 * 8 }
    }

    fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

        // Write version (1 byte)
        cur.write_u8(self.version)?;
        // Write flags (3 bytes)
        cur.write_array(&self.flags.to_be_bytes())?;

        match self.version {
            0 => {
                cur.write_i32_be(self.composition_to_decode_shift as i32)?;
                cur.write_i32_be(self.least_decode_to_display_delta as i32)?;
                cur.write_i32_be(self.greatest_decode_to_display_delta as i32)?;
                cur.write_i32_be(self.composition_start_time as i32)?;
                cur.write_i32_be(self.composition_end_time as i32)?;
            }
            1 => {
                cur.write_i64_be(self.composition_to_decode_shift)?;
                cur.write_i64_be(self.least_decode_to_display_delta)?;
                cur.write_i64_be(self.greatest_decode_to_display_delta)?;
                cur.write_i64_be(self.composition_start_time)?;
                cur.write_i64_be(self.composition_end_time)?;
            }
            _ => {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxVersion {
                        reason: "0 or 1 in this specification",
                        got: self.version,
                    },
                    BoxType::CSLG,
                ));
            }
        }

        Ok(cur.position())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_data_v0() -> [u8; 24] {
        [
            0x00,                   // version = 0
            0x00, 0x00, 0x00,       // flags (3 bytes)
            0x00, 0x00, 0x00, 0x64, // composition_to_decode_shift = 100
            0xFF, 0xFF, 0xFF, 0x9C, // least_decode_to_display_delta = -100
            0x00, 0x00, 0x00, 0xC8, // greatest_decode_to_display_delta = 200
            0x00, 0x00, 0x00, 0x00, // composition_start_time = 0
            0x00, 0x00, 0x03, 0xE8, // composition_end_time = 1000
        ]
    }

    fn raw_data_v1() -> [u8; 44] {
        [
            0x01,                                           // version = 1
            0x00, 0x00, 0x00,                               // flags (3 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x64, // composition_to_decode_shift = 100
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x9C, // least_decode_to_display_delta = -100
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC8, // greatest_decode_to_display_delta = 200
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // composition_start_time = 0
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0xE8, // composition_end_time = 1000
        ]
    }

    #[test]
    fn test_cslg_box_decode_v0() {
        let data = raw_data_v0();
        let cslg = CslgBox::decode(&data).unwrap();

        assert_eq!(cslg.version, 0);
        assert_eq!(cslg.flags.bits(), 0);
        assert_eq!(cslg.composition_to_decode_shift, 100);
        assert_eq!(cslg.least_decode_to_display_delta, -100);
        assert_eq!(cslg.greatest_decode_to_display_delta, 200);
        assert_eq!(cslg.composition_start_time, 0);
        assert_eq!(cslg.composition_end_time, 1000);
    }

    #[test]
    fn test_cslg_box_decode_v1() {
        let data = raw_data_v1();
        let cslg = CslgBox::decode(&data).unwrap();

        assert_eq!(cslg.version, 1);
        assert_eq!(cslg.flags.bits(), 0);
        assert_eq!(cslg.composition_to_decode_shift, 100);
        assert_eq!(cslg.least_decode_to_display_delta, -100);
        assert_eq!(cslg.greatest_decode_to_display_delta, 200);
        assert_eq!(cslg.composition_start_time, 0);
        assert_eq!(cslg.composition_end_time, 1000);
    }

    #[test]
    fn test_cslg_box_invalid_version() {
        let mut data = raw_data_v0();
        data[0] = 2; // invalid version

        let result = CslgBox::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_cslg_box_truncated() {
        let data: [u8; 10] = [0x00; 10]; // too short

        let result = CslgBox::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_cslg_box_round_trip_v0() {
        let original = raw_data_v0();
        let cslg = CslgBox::decode(&original).unwrap();

        let mut encoded = [0u8; 24];
        cslg.encode_into(&mut encoded).unwrap();

        assert_eq!(encoded, original);
    }

    #[test]
    fn test_cslg_box_round_trip_v1() {
        let original = raw_data_v1();
        let cslg = CslgBox::decode(&original).unwrap();

        let mut encoded = [0u8; 44];
        cslg.encode_into(&mut encoded).unwrap();

        assert_eq!(encoded, original);
    }
}
