use crate::BoxIter;
use crate::BoxType;
use crate::cursor::ReadCursor;

use crate::BoxFrame;
use crate::error::*;

use super::FullBoxFlags;

/// A reference to a Data Reference Box (`dref`).
#[derive(Debug)]
pub enum DrefEntryView<'a> {
    /// A Data Entry URL Box (`url `).
    Url(UrlBoxView<'a>),
    /// A Data Entry URN Box (`urn `).
    Urn(UrnBoxView<'a>),
}

/// A reference to a Data Reference Box (`dref`).
#[derive(Debug)]
pub struct DrefBoxView<'a> {
    /// The version of the box.
    pub version: u8,
    /// The flags of the box.
    pub flags: DrefFlags,
    /// The number of entries in the box.
    pub entry_count: u32,
    /// The raw entries payload.
    entries: &'a [u8],
}

impl<'a> DrefBoxView<'a> {
    /// Returns an iterator over the entries in the Data Reference Box.
    pub fn entries(&self) -> impl Iterator<Item = Result<DrefEntryView<'a>>> + 'a {
        BoxIter::new(self.entries).map(|box_result| {
            let view = box_result?;
            match view.boxtype() {
                BoxType::URL_ => UrlBoxView::parse(view.payload()).map(DrefEntryView::Url),
                BoxType::URN_ => UrnBoxView::parse(view.payload()).map(DrefEntryView::Urn),
                other => Err(Error::in_box(
                    ErrorKind::InvalidBoxType {
                        reason: "Unexpected box type in entries",
                        got: other.type_field(),
                    },
                    BoxType::DREF,
                )),
            }
        })
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<DrefBoxView<'a>> {
        let version = cur.read_u8()?;
        let flags = DrefFlags::from_bytes(cur.read_array()?);

        let entry_count = cur.read_u32_be()?;

        let entries = cur.remaining_slice();
        for _ in 0..entry_count {
            BoxFrame::parse_in(cur)?;
        }

        if !cur.is_empty() {
            return Err(Error::at(
                ErrorKind::InvalidBoxSize {
                    reason: "Extra data after parsing all entries",
                    got: cur.remaining() as u64,
                },
                cur.position() as u64,
            ));
        }

        Ok(DrefBoxView {
            version,
            flags,
            entry_count,
            entries,
        })
    }

    /// Parses a `DrefBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(payload);
        let this = DrefBoxView::parse_in(&mut cur)?;

        Ok(this)
    }
}

/// Specification type for Data Reference Box (`dref`).
pub struct DrefSpec;

/// The flags for the Data Reference Box (`dref`).
pub type DrefFlags = FullBoxFlags<DrefSpec>;

/// A reference to a Data Entry URL Box (`url `).
#[derive(Debug)]
pub struct UrlBoxView<'a> {
    /// The version of the box.
    pub version: u8,
    /// The flags of the box.
    pub flags: UrlFlags,
    /// The location of the resource, if not self-contained.
    pub location: Option<&'a str>,
}

impl<'a> UrlBoxView<'a> {
    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<UrlBoxView<'a>> {
        let version = cur.read_u8()?;
        let flags = UrlFlags::from_bytes(cur.read_array()?);

        let location = if flags.contains(UrlFlags::SELF_CONTAINED) {
            None
        } else {
            let loc_bytes = cur.take_until(0)?;
            Some(str::from_utf8(loc_bytes).map_err(|_| {
                Error::at(
                    ErrorKind::InvalidBoxField {
                        field: "location",
                        reason: "invalid UTF-8 data",
                    },
                    cur.position() as u64,
                )
            })?)
        };

        Ok(UrlBoxView {
            version,
            flags,
            location,
        })
    }

    /// Parses an `UrlBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(payload);
        let this = UrlBoxView::parse_in(&mut cur)?;

        Ok(this)
    }
}

/// Specification type for Data Entry URL Box (`url `).
pub struct UrlSpec;

/// The flags for the Data Entry URL Box (`url `).
pub type UrlFlags = FullBoxFlags<UrlSpec>;

impl UrlFlags {
    /// Indicates that the resource is contained within the same file as the `mdat` box.
    pub const SELF_CONTAINED: Self = UrlFlags::from_bits_truncate(0x000001);
}

/// A reference to a Data Entry URN Box (`urn `).
#[derive(Debug)]
pub struct UrnBoxView<'a> {
    /// The version of the box.
    pub version: u8,
    /// The flags of the box.
    pub flags: UrnFlags,
    /// The name of the resource.
    pub name: &'a str,
    /// The location of the resource, if not self-contained.
    pub location: &'a str,
}

impl<'a> UrnBoxView<'a> {
    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<UrnBoxView<'a>> {
        let version = cur.read_u8()?;
        let flags = UrnFlags::from_bytes(cur.read_array()?);

        let name_bytes = cur.take_until(0)?;
        let name = str::from_utf8(name_bytes).map_err(|_| {
            Error::at(
                ErrorKind::InvalidBoxField {
                    field: "name",
                    reason: "invalid UTF-8 data",
                },
                cur.position() as u64,
            )
        })?;

        let location_bytes = cur.take_until(0)?;
        let location = str::from_utf8(location_bytes).map_err(|_| {
            Error::at(
                ErrorKind::InvalidBoxField {
                    field: "location",
                    reason: "invalid UTF-8 data",
                },
                cur.position() as u64,
            )
        })?;

        Ok(UrnBoxView {
            version,
            flags,
            name,
            location,
        })
    }

    /// Parses an `UrnBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(payload);
        let this = UrnBoxView::parse_in(&mut cur)?;

        Ok(this)
    }
}

/// Specification type for Data Entry URN Box (`urn `).
pub struct UrnSpec;

/// The flags for the Data Entry URN Box (`urn `).
pub type UrnFlags = FullBoxFlags<UrnSpec>;

#[cfg(feature = "alloc")]
pub use owned::{
    DrefBox, //
    DrefEntry,
    UrlBox,
    UrnBox,
};

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::String;

    use crate::cursor::WriteCursor;

    use crate::BoxFrameMut;
    use crate::base::frame::write_box_in;

    use super::*;

    /// An owned entry in the Data Reference Box (`dref`).
    #[derive(Debug, Clone)]
    pub enum DrefEntry {
        /// A Data Entry URL Box (`url `).
        Url(UrlBox),
        /// A Data Entry URN Box (`urn `).
        Urn(UrnBox),
    }

    impl From<DrefEntryView<'_>> for DrefEntry {
        fn from(view: DrefEntryView<'_>) -> Self {
            match view {
                DrefEntryView::Url(url_view) => DrefEntry::Url(UrlBox::from_view(&url_view)),
                DrefEntryView::Urn(urn_view) => DrefEntry::Urn(UrnBox::from_view(&urn_view)),
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
        /// The entries in the Data Reference Box.
        pub entries: Vec<DrefEntry>,
    }

    impl DrefBox {
        /// Creates a `DrefBox` from a `DrefBoxView`.
        pub fn from_view(view: &DrefBoxView) -> Result<Self> {
            let mut entries = Vec::with_capacity(view.entry_count as usize);

            for entry_view in view.entries() {
                let entry = entry_view?;
                entries.push(DrefEntry::from(entry));
            }

            Ok(DrefBox {
                version: view.version,
                flags: view.flags,
                entries,
            })
        }

        /// Parses a `DrefBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<Self> {
            let view = DrefBoxView::parse(payload)?;
            Self::from_view(&view)
        }

        /// Returns the size of the `DrefBox` data.
        #[inline]
        pub fn size(&self) -> usize {
            let mut size = 4 + 4; // version/flags + entry_count

            for entry in &self.entries {
                size += match entry {
                    DrefEntry::Url(url_box) => {
                        BoxFrameMut::required_len(BoxType::URL_, url_box.size())
                    }
                    DrefEntry::Urn(urn_box) => {
                        BoxFrameMut::required_len(BoxType::URN_, urn_box.size())
                    }
                };
            }

            size
        }

        pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_bytes())?;

            cur.write_u32_be(self.entries.len() as u32)?;

            for entry in &self.entries {
                match entry {
                    DrefEntry::Url(url_box) => {
                        write_box_in(cur, BoxType::URL_, url_box.size(), |p| url_box.write(p))?;
                    }
                    DrefEntry::Urn(urn_box) => {
                        write_box_in(cur, BoxType::URN_, urn_box.size(), |p| urn_box.write(p))?;
                    }
                }
            }

            if !cur.is_empty() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxSize {
                        reason: "Buffer larger than expected",
                        got: cur.remaining() as u64,
                    },
                    BoxType::DREF,
                ));
            }

            Ok(())
        }

        /// Writes this `DrefBox` into the given payload.
        pub fn write(&self, payload: &mut [u8]) -> Result<()> {
            let mut cursor = WriteCursor::new(payload);
            self.write_in(&mut cursor)
        }
    }

    impl TryFrom<&DrefBoxView<'_>> for DrefBox {
        type Error = Error;

        fn try_from(view: &DrefBoxView<'_>) -> Result<Self> {
            Self::from_view(view)
        }
    }

    /// An owned Data Entry URL Box (`url `).
    #[derive(Debug, Clone)]
    pub struct UrlBox {
        /// The version of the box.
        pub version: u8,
        /// The flags of the box.
        pub flags: UrlFlags,
        /// The location of the resource, if not self-contained.
        pub location: Option<String>,
    }

    impl UrlBox {
        /// Creates an `UrlBox` from an `UrlBoxView`.
        pub fn from_view(view: &UrlBoxView) -> Self {
            UrlBox {
                version: view.version,
                flags: view.flags,
                location: view.location.map(|s| s.to_string()),
            }
        }

        /// Parses an `UrlBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<Self> {
            let url_view = UrlBoxView::parse(payload)?;
            Ok(Self::from_view(&url_view))
        }

        /// Returns the size of the `UrlBox` data.
        #[inline]
        pub fn size(&self) -> usize {
            4 + self.location.as_ref().map_or(0, |loc| loc.len() + 1)
        }

        pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_bytes())?;

            if let Some(location) = &self.location {
                cur.write_slice(location.as_bytes())?;
                cur.write_u8(0)?;
            }

            if !cur.is_empty() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxSize {
                        reason: "Buffer larger than expected",
                        got: cur.remaining() as u64,
                    },
                    BoxType::URL_,
                ));
            }

            Ok(())
        }

        /// Writes this `UrlBox` into the given payload.
        pub fn write(&self, buffer: &mut [u8]) -> Result<()> {
            let mut cursor = WriteCursor::new(buffer);
            self.write_in(&mut cursor)
        }
    }

    impl UrlBoxView<'_> {
        /// Converts this `UrlBoxView` into an owned `UrlBox`.
        pub fn to_owned(&self) -> UrlBox {
            UrlBox::from_view(self)
        }
    }

    impl From<UrlBoxView<'_>> for UrlBox {
        fn from(view: UrlBoxView<'_>) -> Self {
            Self::from_view(&view)
        }
    }

    /// An owned Data Entry URN Box (`urn `).
    #[derive(Debug, Clone)]
    pub struct UrnBox {
        /// The version of the box.
        pub version: u8,
        /// The flags of the box.
        pub flags: UrnFlags,
        /// The name of the resource.
        pub name: String,
        /// The location of the resource.
        pub location: String,
    }

    impl UrnBox {
        /// Creates an `UrnBox` from an `UrnBoxView`.
        pub fn from_view(view: &UrnBoxView) -> Self {
            UrnBox {
                version: view.version,
                flags: view.flags,
                name: view.name.to_string(),
                location: view.location.to_string(),
            }
        }

        /// Parses an `UrnBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<Self> {
            let urn_view = UrnBoxView::parse(payload)?;
            Ok(Self::from_view(&urn_view))
        }

        /// Returns the size of the `UrnBox` data.
        #[inline]
        pub fn size(&self) -> usize {
            4 + self.name.len() + 1 + self.location.len() + 1
        }

        pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_bytes())?;

            cur.write_slice(self.name.as_bytes())?;
            cur.write_u8(0)?;

            cur.write_slice(self.location.as_bytes())?;
            cur.write_u8(0)?;

            if !cur.is_empty() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxSize {
                        reason: "Buffer larger than expected",
                        got: cur.remaining() as u64,
                    },
                    BoxType::URN_,
                ));
            }

            Ok(())
        }

        /// Writes this `UrnBox` into the given payload.
        pub fn write(&self, payload: &mut [u8]) -> Result<()> {
            let mut cursor = WriteCursor::new(payload);
            self.write_in(&mut cursor)
        }
    }

    impl UrnBoxView<'_> {
        /// Converts this `UrnBoxView` into an owned `UrnBox`.
        pub fn to_owned(&self) -> UrnBox {
            UrnBox::from_view(self)
        }
    }

    impl From<UrnBoxView<'_>> for UrnBox {
        fn from(view: UrnBoxView<'_>) -> Self {
            Self::from_view(&view)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_box_header(size: u32, fourcc: &[u8; 4]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&size.to_be_bytes());
        data.extend_from_slice(fourcc);
        data
    }

    fn make_full_box_header(version: u8, flags: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(version);
        data.extend_from_slice(&flags.to_be_bytes()[1..4]);
        data
    }

    fn make_url_entry_self_contained() -> Vec<u8> {
        let mut payload = make_full_box_header(0, 0x000001);
        let size = 8 + payload.len() as u32;
        let mut data = make_box_header(size, b"url ");
        data.append(&mut payload);
        data
    }

    fn make_url_entry_with_location(location: &str) -> Vec<u8> {
        let mut payload = make_full_box_header(0, 0x000000);
        payload.extend_from_slice(location.as_bytes());
        payload.push(0);
        let size = 8 + payload.len() as u32;
        let mut data = make_box_header(size, b"url ");
        data.append(&mut payload);
        data
    }

    fn make_urn_entry(name: &str, location: &str) -> Vec<u8> {
        let mut payload = make_full_box_header(0, 0x000000);
        payload.extend_from_slice(name.as_bytes());
        payload.push(0);
        payload.extend_from_slice(location.as_bytes());
        payload.push(0);
        let size = 8 + payload.len() as u32;
        let mut data = make_box_header(size, b"urn ");
        data.append(&mut payload);
        data
    }

    fn make_dref_payload(entries: Vec<Vec<u8>>) -> Vec<u8> {
        let mut payload = make_full_box_header(0, 0);
        payload.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        for entry in entries {
            payload.extend_from_slice(&entry);
        }
        payload
    }

    #[test]
    fn parse_dref_empty() {
        let payload = make_dref_payload(vec![]);
        let dref = DrefBoxView::parse(&payload).unwrap();

        assert_eq!(dref.entry_count, 0);
        assert_eq!(dref.entries().count(), 0);
    }

    #[test]
    fn parse_dref_multiple_entries() {
        let entries = vec![
            make_url_entry_self_contained(),
            make_url_entry_with_location("http://example.com/a.mp4"),
            make_urn_entry("urn:test", "http://test.com"),
        ];
        let payload = make_dref_payload(entries);
        let dref = DrefBoxView::parse(&payload).unwrap();

        assert_eq!(dref.entry_count, 3);

        let results: Vec<_> = dref.entries().collect();
        assert_eq!(results.len(), 3);

        // URL (self-contained)
        match results[0].as_ref().unwrap() {
            DrefEntryView::Url(url) => {
                assert!(url.flags.contains(UrlFlags::SELF_CONTAINED));
                assert!(url.location.is_none());
            }
            _ => panic!("Expected Url entry"),
        }

        // URL (with location)
        match results[1].as_ref().unwrap() {
            DrefEntryView::Url(url) => {
                assert!(!url.flags.contains(UrlFlags::SELF_CONTAINED));
                assert_eq!(url.location, Some("http://example.com/a.mp4"));
            }
            _ => panic!("Expected Url entry"),
        }

        // URN
        match results[2].as_ref().unwrap() {
            DrefEntryView::Urn(urn) => {
                assert_eq!(urn.name, "urn:test");
                assert_eq!(urn.location, "http://test.com");
            }
            _ => panic!("Expected Urn entry"),
        }
    }

    #[test]
    fn parse_dref_entry_count_mismatch() {
        let mut payload = make_full_box_header(0, 0);
        payload.extend_from_slice(&2u32.to_be_bytes());
        payload.extend_from_slice(&make_url_entry_self_contained());

        let result = DrefBoxView::parse(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn parse_dref_invalid_entry_type() {
        let mut invalid_entry = make_box_header(12, b"xxxx");
        invalid_entry.extend_from_slice(&make_full_box_header(0, 0));

        let payload = make_dref_payload(vec![invalid_entry]);
        let dref = DrefBoxView::parse(&payload).unwrap();

        let entry = dref.entries().next().unwrap();
        assert!(entry.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn dref_box_round_trip() {
        // Parse original data
        let entries = vec![
            make_url_entry_self_contained(),
            make_url_entry_with_location("http://example.com/video.mp4"),
            make_urn_entry("urn:example", "http://example.com"),
        ];
        let original_payload = make_dref_payload(entries);
        let original_view = DrefBoxView::parse(&original_payload).unwrap();
        let owned = DrefBox::from_view(&original_view).unwrap();

        // Write to buffer
        let mut buf = vec![0u8; owned.size()];
        owned.write(&mut buf).unwrap();

        // Parse again and compare
        let reparsed = DrefBoxView::parse(&buf).unwrap();
        assert_eq!(reparsed.entry_count, original_view.entry_count);

        let original_entries: Vec<_> = original_view.entries().collect();
        let reparsed_entries: Vec<_> = reparsed.entries().collect();

        for (orig, reparsed) in original_entries.iter().zip(reparsed_entries.iter()) {
            match (orig.as_ref().unwrap(), reparsed.as_ref().unwrap()) {
                (DrefEntryView::Url(o), DrefEntryView::Url(r)) => {
                    assert_eq!(o.version, r.version);
                    assert_eq!(o.flags.get(), r.flags.get());
                    assert_eq!(o.location, r.location);
                }
                (DrefEntryView::Urn(o), DrefEntryView::Urn(r)) => {
                    assert_eq!(o.version, r.version);
                    assert_eq!(o.flags.get(), r.flags.get());
                    assert_eq!(o.name, r.name);
                    assert_eq!(o.location, r.location);
                }
                _ => panic!("Entry type mismatch"),
            }
        }
    }
}
