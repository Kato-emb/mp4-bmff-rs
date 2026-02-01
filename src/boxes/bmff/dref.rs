use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

use crate::cursor::ReadCursor;

define_box_flags!(
    /// Flags for the Data Entry URL Box (`url `).
    UrlFlags {
        /// Indicates that the data is in the same file as the containing box.
        SELF_CONTAINED = 0x000001,
    }
);

/// A reference to a Data Entry URL Box (`url `).
#[derive(Debug)]
pub struct UrlBoxView<'a> {
    /// The version of the box.
    pub version: u8,
    /// The flags of the box.
    pub flags: UrlFlags,
    /// The location string, if present.
    pub location: Option<&'a str>,
}

impl BoxCodec for UrlBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::URL_
    }
}

impl<'de> BoxDecode<'de> for UrlBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = UrlFlags::from_be_bytes(cur.read_array::<3>()?);

        let location = if !flags.contains(UrlFlags::SELF_CONTAINED) {
            let loc_bytes = cur.take_until(0)?; // Read until null terminator

            if !cur.is_empty() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxField {
                        field: "location",
                        reason: "extra data after null terminator",
                    },
                    BoxType::URL_,
                ));
            }

            let loc_str = core::str::from_utf8(loc_bytes).map_err(|_| {
                Error::in_box(
                    ErrorKind::InvalidBoxField {
                        field: "location",
                        reason: "not valid UTF-8",
                    },
                    BoxType::URL_,
                )
            })?;
            Some(loc_str)
        } else {
            None
        };

        Ok(UrlBoxView {
            version,
            flags,
            location,
        })
    }
}

define_box_flags!(
    /// Flags for the Data Entry URN Box (`urn `).
    UrnFlags {
        // Currently, no specific flags are defined for the URN box.
    }
);

/// A reference to a Data Entry URN Box (`urn `).
#[derive(Debug)]
pub struct UrnBoxView<'a> {
    /// The version of the box.
    pub version: u8,
    /// The flags of the box.
    pub flags: UrnFlags,
    /// The name string.
    pub name: &'a str,
    /// The location string, if present.
    pub location: Option<&'a str>,
}

impl BoxCodec for UrnBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::URN_
    }
}

impl<'de> BoxDecode<'de> for UrnBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = UrnFlags::from_be_bytes(cur.read_array::<3>()?);

        let name_bytes = cur.take_until(0)?; // Read until null terminator
        let name_str = core::str::from_utf8(name_bytes).map_err(|_| {
            Error::in_box(
                ErrorKind::InvalidBoxField {
                    field: "name",
                    reason: "not valid UTF-8",
                },
                BoxType::URN_,
            )
        })?;

        let location = if !cur.is_empty() {
            let loc_bytes = cur.take_until(0)?; // Read until null terminator

            let loc_str = core::str::from_utf8(loc_bytes).map_err(|_| {
                Error::in_box(
                    ErrorKind::InvalidBoxField {
                        field: "location",
                        reason: "not valid UTF-8",
                    },
                    BoxType::URN_,
                )
            })?;
            Some(loc_str)
        } else {
            None
        };

        Ok(UrnBoxView {
            version,
            flags,
            name: name_str,
            location,
        })
    }
}

/// A reference to a Data Entry Box, which can be either a URL or URN box.
#[derive(Debug)]
pub enum DataEntryBoxView<'a> {
    /// A Data Entry URL Box.
    Url(UrlBoxView<'a>),
    /// A Data Entry URN Box.
    Urn(UrnBoxView<'a>),
}

pub struct DataEntryBoxIter<'a> {
    iter: BoxIter<'a>,
    remaining: usize,
}

impl<'a> DataEntryBoxIter<'a> {
    pub fn new(data: &'a [u8], entry_count: u32) -> Self {
        DataEntryBoxIter {
            iter: BoxIter::new(data),
            remaining: entry_count as usize,
        }
    }
}

impl<'a> Iterator for DataEntryBoxIter<'a> {
    type Item = Result<DataEntryBoxView<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        let rawbox = match self.iter.next()? {
            Ok(b) => b,
            Err(e) => return Some(Err(e)),
        };

        let data_entry_box = match rawbox.boxtype() {
            BoxType::URL_ => {
                let url_box = UrlBoxView::decode(rawbox.into_payload()).ok()?;
                DataEntryBoxView::Url(url_box)
            }
            BoxType::URN_ => {
                let urn_box = UrnBoxView::decode(rawbox.into_payload()).ok()?;
                DataEntryBoxView::Urn(urn_box)
            }
            other => {
                return Some(Err(Error::in_box(
                    ErrorKind::InvalidBoxType {
                        reason: "Data Entry Box must be either URL or URN box",
                        got: other.type_field(),
                    },
                    BoxType::DREF,
                )));
            }
        };

        self.remaining -= 1;
        Some(Ok(data_entry_box))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for DataEntryBoxIter<'_> {}

define_box_flags!(
    /// Flags for the Data Reference Box (`dref`).
    DrefFlags {}
);

/// A reference to a Data Reference Box (`dref`).
#[derive(Debug)]
pub struct DrefBoxView<'a> {
    /// The version of the box.
    pub version: u8,
    /// The flags of the box.
    pub flags: DrefFlags,
    /// The number of entries in the box.
    pub entry_count: u32,
    data_entries: &'a [u8],
}

impl<'a> DrefBoxView<'a> {
    /// Returns an iterator over the data entry boxes in the Data Reference Box (`dref`).
    pub fn data_entries(&self) -> DataEntryBoxIter<'a> {
        DataEntryBoxIter::new(self.data_entries, self.entry_count)
    }
}

impl BoxCodec for DrefBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::DREF
    }
}

impl<'de> BoxDecode<'de> for DrefBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = DrefFlags::from_be_bytes(cur.read_array::<3>()?);

        let entry_count = cur.read_u32_be()?;
        let data_entries = cur.take(cur.remaining())?;

        Ok(DrefBoxView {
            version,
            flags,
            entry_count,
            data_entries,
        })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;
    use crate::lib::{
        String, //
        ToString,
    };

    use super::*;
    use crate::BoxEncode;

    use crate::codec::boxed_len;
    use crate::codec::write_box_in;
    use crate::cursor::WriteCursor;

    /// An owned Data Entry URL Box (`url `).
    #[derive(Debug, Clone)]
    pub struct UrlBox {
        /// The version of the box.
        pub version: u8,
        /// The flags of the box.
        pub flags: UrlFlags,
        /// The location string, if present.
        pub location: Option<String>,
    }

    impl From<&UrlBoxView<'_>> for UrlBox {
        fn from(view: &UrlBoxView<'_>) -> Self {
            UrlBox {
                version: view.version,
                flags: view.flags,
                location: view.location.map(|s| s.to_string()),
            }
        }
    }

    impl BoxCodec for UrlBox {
        fn boxtype(&self) -> BoxType {
            BoxType::URL_
        }
    }

    impl BoxDecode<'_> for UrlBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = UrlBoxView::decode(bytes)?;
            Ok(UrlBox::from(&view))
        }
    }

    impl BoxEncode for UrlBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            4 + if let Some(loc) = &self.location {
                loc.len() + 1 // +1 for null terminator
            } else {
                0
            }
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_be_bytes())?;

            if let Some(loc) = &self.location {
                cur.write_slice(loc.as_bytes())?;
                cur.write_u8(0)?; // Null terminator
            }

            Ok(cur.position())
        }
    }

    /// An owned Data Entry URN Box (`urn `).
    #[derive(Debug, Clone)]
    pub struct UrnBox {
        /// The version of the box.
        pub version: u8,
        /// The flags of the box.
        pub flags: UrnFlags,
        /// The name string.
        pub name: String,
        /// The location string, if present.
        pub location: Option<String>,
    }

    impl From<&UrnBoxView<'_>> for UrnBox {
        fn from(view: &UrnBoxView<'_>) -> Self {
            UrnBox {
                version: view.version,
                flags: view.flags,
                name: view.name.to_string(),
                location: view.location.map(|s| s.to_string()),
            }
        }
    }

    impl BoxCodec for UrnBox {
        fn boxtype(&self) -> BoxType {
            BoxType::URN_
        }
    }

    impl BoxDecode<'_> for UrnBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = UrnBoxView::decode(bytes)?;
            Ok(UrnBox::from(&view))
        }
    }

    impl BoxEncode for UrnBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            4 + self.name.len() + 1 // +1 for null terminator
                + if let Some(loc) = &self.location {
                    loc.len() + 1 // +1 for null terminator
                } else {
                    0
                }
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_be_bytes())?;

            cur.write_slice(self.name.as_bytes())?;
            cur.write_u8(0)?; // Null terminator

            if let Some(loc) = &self.location {
                cur.write_slice(loc.as_bytes())?;
                cur.write_u8(0)?; // Null terminator
            }

            Ok(cur.position())
        }
    }

    /// An owned Data Entry Box, which can be either a URL or URN box.
    #[derive(Debug, Clone)]
    pub enum DataEntryBox {
        /// A Data Entry URL Box.
        Url(UrlBox),
        /// A Data Entry URN Box.
        Urn(UrnBox),
    }

    impl From<&DataEntryBoxView<'_>> for DataEntryBox {
        fn from(view: &DataEntryBoxView<'_>) -> Self {
            match view {
                DataEntryBoxView::Url(url_view) => DataEntryBox::Url(UrlBox::from(url_view)),
                DataEntryBoxView::Urn(urn_view) => DataEntryBox::Urn(UrnBox::from(urn_view)),
            }
        }
    }

    /// An owned Data Reference Box (`dref`).
    #[derive(Debug, Clone)]
    pub struct DrefBox {
        /// The version of the box.
        pub version: u8,
        /// The flags of the box.
        pub flags: DrefFlags,
        /// The entries in the box.
        pub entries: Vec<DataEntryBox>,
    }

    impl TryFrom<&DrefBoxView<'_>> for DrefBox {
        type Error = Error;

        fn try_from(view: &DrefBoxView<'_>) -> Result<Self> {
            let mut entries = Vec::with_capacity(view.entry_count as usize);

            for entry_result in view.data_entries() {
                let entry_view = entry_result?;
                entries.push(DataEntryBox::from(&entry_view));
            }

            Ok(DrefBox {
                version: view.version,
                flags: view.flags,
                entries,
            })
        }
    }

    impl BoxCodec for DrefBox {
        fn boxtype(&self) -> BoxType {
            BoxType::DREF
        }
    }

    impl BoxDecode<'_> for DrefBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = DrefBoxView::decode(bytes)?;
            DrefBox::try_from(&view)
        }
    }

    impl BoxEncode for DrefBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            let mut len = 1 + 3 + 4; // version + flags + entry_count

            for entry in &self.entries {
                match entry {
                    DataEntryBox::Url(url_box) => {
                        len += boxed_len(url_box);
                    }
                    DataEntryBox::Urn(urn_box) => {
                        len += boxed_len(urn_box);
                    }
                }
            }

            len
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_be_bytes())?;

            cur.write_u32_be(self.entries.len() as u32)?;

            for entry in &self.entries {
                match entry {
                    DataEntryBox::Url(url_box) => write_box_in(&mut cur, url_box)?,
                    DataEntryBox::Urn(urn_box) => write_box_in(&mut cur, urn_box)?,
                }
            }

            Ok(cur.position())
        }
    }
}

#[cfg(feature = "alloc")]
pub use owned::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn url_raw_data() -> [u8; 4] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x01, // flags = SELF_CONTAINED
        ]
    }

    fn urn_raw_data() -> [u8; 13] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            b'u', b'r', b'n', b':', b't', b'e', b's', b't', // name = "urn:test"
            0x00, // null terminator
        ]
    }

    fn dref_raw_data() -> [u8; 20] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x01, // entry_count = 1
            // url box (self-contained)
            0x00, 0x00, 0x00, 0x0C, // size = 12
            b'u', b'r', b'l', b' ', // type = "url "
            0x00, // version = 0
            0x00, 0x00, 0x01, // flags = SELF_CONTAINED
        ]
    }

    #[test]
    fn test_url_box_view_decode() {
        let data = url_raw_data();
        let url = UrlBoxView::decode(&data).unwrap();

        assert_eq!(url.version, 0);
        assert!(url.flags.contains(UrlFlags::SELF_CONTAINED));
        assert!(url.location.is_none());
    }

    #[test]
    fn test_url_box_view_truncated() {
        let data: [u8; 2] = [0x00, 0x00];
        let result = UrlBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_urn_box_view_decode() {
        let data = urn_raw_data();
        let urn = UrnBoxView::decode(&data).unwrap();

        assert_eq!(urn.version, 0);
        assert_eq!(urn.name, "urn:test");
        assert!(urn.location.is_none());
    }

    #[test]
    fn test_urn_box_view_truncated() {
        let data: [u8; 2] = [0x00, 0x00];
        let result = UrnBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_dref_box_view_decode() {
        let data = dref_raw_data();
        let dref = DrefBoxView::decode(&data).unwrap();

        assert_eq!(dref.version, 0);
        assert_eq!(dref.flags.bits(), 0);
        assert_eq!(dref.entry_count, 1);

        let mut entries = dref.data_entries();
        let entry = entries.next().unwrap().unwrap();
        assert!(matches!(entry, DataEntryBoxView::Url(_)));
        assert!(entries.next().is_none());
    }

    #[test]
    fn test_dref_box_view_empty_entries() {
        let data: [u8; 8] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x00, // entry_count = 0
        ];

        let dref = DrefBoxView::decode(&data).unwrap();
        assert_eq!(dref.entry_count, 0);
        assert_eq!(dref.data_entries().count(), 0);
    }

    #[test]
    fn test_dref_box_view_truncated() {
        let data: [u8; 4] = [0x00; 4];
        let result = DrefBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_url_box_round_trip() {
        use crate::BoxEncode;

        let original = url_raw_data();
        let url = UrlBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; url.encoded_len()];
        url.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_urn_box_round_trip() {
        use crate::BoxEncode;

        let original = urn_raw_data();
        let urn = UrnBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; urn.encoded_len()];
        urn.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_dref_box_round_trip() {
        use crate::BoxEncode;

        let original = dref_raw_data();
        let dref = DrefBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; dref.encoded_len()];
        dref.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_dref_box_try_from() {
        let data = dref_raw_data();
        let view = DrefBoxView::decode(&data).unwrap();
        let owned = DrefBox::try_from(&view).unwrap();

        assert_eq!(owned.version, view.version);
        assert_eq!(owned.flags.bits(), view.flags.bits());
        assert_eq!(owned.entries.len(), view.entry_count as usize);
    }
}
