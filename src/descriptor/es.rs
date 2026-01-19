use crate::cursor::ReadCursor;

use crate::error::*;

use super::DecoderConfigDescriptorView;
use super::DescriptorView;
use super::DescrptorIter;
use super::Tag;

/// Elementary Stream Descriptor (7.2.6.5 ES_Descriptor)
#[derive(Debug)]
pub struct EsDescriptorView<'a> {
    /// Elementary Stream ID
    pub es_id: u16,
    /// Flags
    pub stream_dependence_flag: bool,
    /// URL Flag
    pub url_flag: bool,
    /// OCR Stream Flag
    pub ocr_stream_flag: bool,
    /// Stream Priority
    pub stream_priority: u8,
    /// Optional fields
    pub depends_on_es_id: Option<u16>,
    /// URL String
    pub url_string: Option<&'a str>,
    /// Optional OCR ES ID
    pub ocr_es_id: Option<u16>,
    /// Decoder Config Descriptor
    pub decoder_config_descriptor: DecoderConfigDescriptorView<'a>,
    /// SL Config Descriptor
    pub sl_config_descriptor: DescriptorView<'a>,
    /// Other extensions
    extensions: &'a [u8],
}

impl<'a> EsDescriptorView<'a> {
    /// Returns an iterator over the extension descriptors
    pub fn extensions(&self) -> DescrptorIter<'_> {
        DescrptorIter::new(self.extensions)
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<Self> {
        let es_id = cur.read_u16_be()?;

        let flags_byte = cur.read_u8()?;
        let stream_dependence_flag = (flags_byte & 0b10000000) != 0;
        let url_flag = (flags_byte & 0b01000000) != 0;
        let ocr_stream_flag = (flags_byte & 0b00100000) != 0;
        let stream_priority = flags_byte & 0b00011111;

        let depends_on_es_id = if stream_dependence_flag {
            Some(cur.read_u16_be()?)
        } else {
            None
        };

        let url_string = if url_flag {
            let url_length = cur.read_u8()? as usize;
            let url_bytes = cur.take(url_length)?;
            Some(core::str::from_utf8(url_bytes).map_err(|_| {
                Error::at(
                    ErrorKind::Other {
                        description: "Invalid UTF-8 in URL string",
                    },
                    cur.position() as u64 - url_length as u64,
                )
            })?)
        } else {
            None
        };

        let ocr_es_id = if ocr_stream_flag {
            Some(cur.read_u16_be()?)
        } else {
            None
        };

        let view = DescriptorView::parse_in(cur)?;

        let decoder_config_descriptor = if view.tag == Tag::DECODER_CONFIG_DESCR_TAG {
            DecoderConfigDescriptorView::parse(view.instance)?
        } else {
            return Err(Error::at(
                ErrorKind::Other {
                    description: "Expected Decoder Config Descriptor",
                },
                cur.position() as u64,
            ));
        };

        let view = DescriptorView::parse_in(cur)?;

        let sl_config_descriptor = if view.tag == Tag::SL_CONFIG_DESCR_TAG {
            view
        } else {
            return Err(Error::at(
                ErrorKind::Other {
                    description: "Expected SL Config Descriptor",
                },
                cur.position() as u64,
            ));
        };

        let extensions = cur.take(cur.remaining())?;

        Ok(EsDescriptorView {
            es_id,
            stream_dependence_flag,
            url_flag,
            ocr_stream_flag,
            stream_priority,
            depends_on_es_id,
            url_string,
            ocr_es_id,
            decoder_config_descriptor,
            sl_config_descriptor,
            extensions,
        })
    }

    /// Parses EsDescriptor from a byte slice
    pub fn parse(instance: &'a [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(instance);
        Self::parse_in(&mut cur)
    }
}

#[cfg(feature = "alloc")]
pub use owned::EsDescriptor;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::String;

    use super::*;
    use crate::descriptor::DecoderConfigDescriptor;
    use crate::descriptor::DescriptorOwned;

    /// Owned Elementary Stream Descriptor
    #[derive(Debug, Clone)]
    pub struct EsDescriptor {
        /// Elementary Stream ID
        pub es_id: u16,
        /// Flags
        pub stream_dependence_flag: bool,
        /// URL Flag
        pub url_flag: bool,
        /// OCR Stream Flag
        pub ocr_stream_flag: bool,
        /// Stream Priority
        pub stream_priority: u8,
        /// Optional fields
        pub depends_on_es_id: Option<u16>,
        /// URL String
        pub url_string: Option<String>,
        /// Optional OCR ES ID
        pub ocr_es_id: Option<u16>,
        /// Decoder Config Descriptor
        pub decoder_config_descriptor: DecoderConfigDescriptor,
        /// SL Config Descriptor
        pub sl_config_descriptor: DescriptorOwned,
        /// Other extensions
        pub extensions: Vec<DescriptorOwned>,
    }

    impl EsDescriptor {
        /// Creates an owned EsDescriptor from a view
        pub fn from_view(view: &EsDescriptorView<'_>) -> Result<Self> {
            let url_string = view.url_string.map(|s| s.to_owned());

            let decoder_config_descriptor =
                DecoderConfigDescriptor::from_view(&view.decoder_config_descriptor)?;

            let sl_config_descriptor = view.sl_config_descriptor.to_owned();

            let extensions = view
                .extensions()
                .map(|res| res.map(|d| d.to_owned()))
                .collect::<Result<Vec<_>>>()?;

            Ok(EsDescriptor {
                es_id: view.es_id,
                stream_dependence_flag: view.stream_dependence_flag,
                url_flag: view.url_flag,
                ocr_stream_flag: view.ocr_stream_flag,
                stream_priority: view.stream_priority,
                depends_on_es_id: view.depends_on_es_id,
                url_string,
                ocr_es_id: view.ocr_es_id,
                decoder_config_descriptor,
                sl_config_descriptor,
                extensions,
            })
        }
    }
}
