use crate::cursor::ReadCursor;

use crate::error::*;

use crate::descriptor::DescriptorView;
use crate::descriptor::EsDescriptorView;
use crate::descriptor::Tag;

use super::FullBoxFlags;

/// A reference to an ESDS box's contents.
#[derive(Debug)]
pub struct EsdsBoxView<'a> {
    /// The version of this ESDS box.
    pub version: u8,
    /// The flags of this ESDS box.
    pub flags: EsdsFlags,
    /// The ES Descriptor contained in this ESDS box.
    pub esd: EsDescriptorView<'a>,
}

impl<'a> EsdsBoxView<'a> {
    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<Self> {
        let version = cur
            .read_u8()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let flags = EsdsFlags::from_bytes(
            cur.read_array()
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
        );
        let es_descr = DescriptorView::parse_in(cur)?;
        let esd = if es_descr.tag == Tag::ES_DESCR_TAG {
            EsDescriptorView::parse(es_descr.instance)?
        } else {
            return Err(Error::at(
                ErrorKind::Other {
                    description: "Expected ES Descriptor",
                },
                cur.position() as u64,
            ));
        };

        Ok(EsdsBoxView {
            version,
            flags,
            esd,
        })
    }

    /// Parses an `EsdsBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(payload);
        let this = EsdsBoxView::parse_in(&mut cur)?;

        Ok(this)
    }
}

/// Specification type for ESDS Box ('esds')
pub struct EsdsSpec;

/// The flags for the ESDS Box ('esds')
pub type EsdsFlags = FullBoxFlags<EsdsSpec>;

#[cfg(feature = "alloc")]
pub use owned::EsdsBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::descriptor::EsDescriptor;

    use super::*;

    /// An owned ESDS box.
    #[derive(Debug, Clone)]
    pub struct EsdsBox {
        /// The version of this ESDS box.
        pub version: u8,
        /// The flags of this ESDS box.
        pub flags: EsdsFlags,
        /// The ES Descriptor contained in this ESDS box.
        pub esd: EsDescriptor,
    }

    impl EsdsBox {
        /// Creates an owned ESDS box from a view.
        pub fn from_view(view: &EsdsBoxView) -> Result<Self> {
            Ok(Self {
                version: view.version,
                flags: view.flags,
                esd: EsDescriptor::from_view(&view.esd)?,
            })
        }
    }

    impl TryFrom<EsdsBoxView<'_>> for EsdsBox {
        type Error = Error;

        fn try_from(value: EsdsBoxView<'_>) -> Result<Self> {
            EsdsBox::from_view(&value)
        }
    }
}
