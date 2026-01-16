use crate::error::*;

/// A reference to a Free Space Box (`free`).
pub struct FreeBoxView<'a> {
    /// The raw data of the Free Space Box (`free`).
    pub data: &'a [u8],
}

impl<'a> FreeBoxView<'a> {
    /// Parses a `FreeBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<FreeBoxView<'a>> {
        Ok(FreeBoxView { data: payload })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_free_box_view_parse() {
        let data = b"example free space data";
        let free_box_view = FreeBoxView::parse(data).unwrap();
        assert_eq!(free_box_view.data, data);
    }
}
