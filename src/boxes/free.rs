use crate::cursor::ReadCursor;

use crate::error::*;

/// A reference to a Free Space Box (`free`).
pub struct FreeBoxView<'a> {
    /// The raw data of the Free Space Box (`free`).
    pub data: &'a [u8],
}

impl<'a> FreeBoxView<'a> {
    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<FreeBoxView<'a>> {
        let data = cur.take(cur.remaining())?;
        Ok(FreeBoxView { data })
    }

    /// Parses a `FreeBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<FreeBoxView<'a>> {
        let mut cursor = ReadCursor::new(payload);
        let this = FreeBoxView::parse_in(&mut cursor)?;

        Ok(this)
    }
}

#[cfg(feature = "alloc")]
pub use owned::FreeBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use super::*;

    /// An owned Free Space Box (`free`).
    #[derive(Debug, Clone)]
    pub struct FreeBox {
        /// The raw data of the Free Space Box (`free`).
        pub data: Vec<u8>,
    }

    impl FreeBox {
        /// Creates a `FreeBox` from a `FreeBoxView`.
        pub fn from_view(view: &FreeBoxView<'_>) -> Self {
            FreeBox {
                data: view.data.to_vec(),
            }
        }

        /// Parses a `FreeBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<Self> {
            let free_view = FreeBoxView::parse(payload)?;
            Ok(Self::from_view(&free_view))
        }
    }

    impl FreeBoxView<'_> {
        /// Converts this `FreeBoxView` into an owned `FreeBox`.
        pub fn to_owned(&self) -> FreeBox {
            FreeBox::from_view(self)
        }
    }

    impl From<FreeBoxView<'_>> for FreeBox {
        fn from(view: FreeBoxView) -> Self {
            Self::from_view(&view)
        }
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
