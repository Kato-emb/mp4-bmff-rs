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

#[cfg(feature = "alloc")]
pub use owned::MdatBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use super::*;

    /// An owned Media Data Box (`mdat`).
    #[derive(Debug, Clone)]
    pub struct MdatBox {
        /// The raw data of the Media Data Box (`mdat`).
        pub data: Vec<u8>,
    }

    impl MdatBox {
        /// Creates an `MdatBox` from an `MdatBoxView`.
        pub fn from_view(view: &MdatBoxView<'_>) -> Self {
            MdatBox {
                data: view.data.to_vec(),
            }
        }

        /// Parses an `MdatBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<Self> {
            let mdat_view = MdatBoxView::parse(payload)?;
            Ok(Self::from_view(&mdat_view))
        }
    }

    impl MdatBoxView<'_> {
        /// Converts this `MdatBoxView` into an owned `MdatBox`.
        pub fn to_owned(&self) -> MdatBox {
            MdatBox::from_view(self)
        }
    }

    impl From<MdatBoxView<'_>> for MdatBox {
        fn from(view: MdatBoxView) -> Self {
            Self::from_view(&view)
        }
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
