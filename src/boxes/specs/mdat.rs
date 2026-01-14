use crate::boxes::error::*;

/// A reference to a Media Data Box (`mdat`).
pub struct MdatBoxRef<'a> {
    /// The raw data of the Media Data Box (`mdat`).
    pub data: &'a [u8],
}

impl<'a> MdatBoxRef<'a> {
    /// Parses an `MdatBoxRef` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<MdatBoxRef<'a>> {
        Ok(MdatBoxRef { data: payload })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mdat_box_ref_parse() {
        let data = b"example media data";
        let mdat_box_ref = MdatBoxRef::parse(data).unwrap();
        assert_eq!(mdat_box_ref.data, data);
    }
}
