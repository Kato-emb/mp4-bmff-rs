use crate::error::*;

/// A reference to a Media Data Box (`mdat`).
pub struct MdatBoxView<'a> {
    /// The raw data of the Media Data Box (`mdat`).
    pub data: &'a [u8],
}

impl<'a> MdatBoxView<'a> {
    /// Parses an `MdatBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<MdatBoxView<'a>> {
        Ok(MdatBoxView { data: payload })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mdat_box_view_parse() {
        let data = b"example media data";
        let mdat_box_view = MdatBoxView::parse(data).unwrap();
        assert_eq!(mdat_box_view.data, data);
    }
}
