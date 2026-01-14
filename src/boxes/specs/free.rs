use crate::boxes::error::*;

/// A reference to a Free Space Box (`free`).
pub struct FreeBoxRef<'a> {
    /// The raw data of the Free Space Box (`free`).
    pub data: &'a [u8],
}

impl<'a> FreeBoxRef<'a> {
    /// Parses a `FreeBoxRef` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<FreeBoxRef<'a>> {
        Ok(FreeBoxRef { data: payload })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_free_box_ref_parse() {
        let data = b"example free space data";
        let free_box_ref = FreeBoxRef::parse(data).unwrap();
        assert_eq!(free_box_ref.data, data);
    }
}
