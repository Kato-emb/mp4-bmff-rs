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
        let version = cur
            .read_u8()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let flags = DrefFlags::from_bytes(
            cur.read_array()
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
        );

        let entry_count = cur
            .read_u32_be()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;

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
        let version = cur
            .read_u8()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let flags = UrlFlags::from_bytes(
            cur.read_array()
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
        );

        let location = if flags.contains(UrlFlags::SELF_CONTAINED) {
            None
        } else {
            let loc_bytes = cur
                .take_until(0)
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
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
        let version = cur
            .read_u8()
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let flags = UrnFlags::from_bytes(
            cur.read_array()
                .map_err(|e| Error::at(e.into(), cur.position() as u64))?,
        );

        let name_bytes = cur
            .take_until(0)
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
        let name = str::from_utf8(name_bytes).map_err(|_| {
            Error::at(
                ErrorKind::InvalidBoxField {
                    field: "name",
                    reason: "invalid UTF-8 data",
                },
                cur.position() as u64,
            )
        })?;

        let location_bytes = cur
            .take_until(0)
            .map_err(|e| Error::at(e.into(), cur.position() as u64))?;
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

    /// Helper to create a box header (size + type).
    fn make_box_header(size: u32, fourcc: &[u8; 4]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&size.to_be_bytes());
        data.extend_from_slice(fourcc);
        data
    }

    /// Helper to create a FullBoxHeader payload (version + flags).
    fn make_full_box_header(version: u8, flags: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(version);
        data.extend_from_slice(&flags.to_be_bytes()[1..4]); // Only 3 bytes for flags
        data
    }

    /// Creates a self-contained URL box payload (no location).
    fn make_url_box_self_contained() -> Vec<u8> {
        let mut payload = Vec::new();
        // FullBoxHeader: version=0, flags=0x000001 (self-contained)
        payload.extend_from_slice(&make_full_box_header(0, 0x000001));
        payload
    }

    /// Creates a URL box payload with a location.
    fn make_url_box_with_location(location: &str) -> Vec<u8> {
        let mut payload = Vec::new();
        // FullBoxHeader: version=0, flags=0x000000
        payload.extend_from_slice(&make_full_box_header(0, 0x000000));
        // location (null-terminated)
        payload.extend_from_slice(location.as_bytes());
        payload.push(0);
        payload
    }

    /// Creates a URN box payload.
    fn make_urn_box_payload(name: &str, location: &str) -> Vec<u8> {
        let mut payload = Vec::new();
        // FullBoxHeader: version=0, flags=0x000000
        payload.extend_from_slice(&make_full_box_header(0, 0x000000));
        // name (null-terminated)
        payload.extend_from_slice(name.as_bytes());
        payload.push(0);
        // location (null-terminated)
        payload.extend_from_slice(location.as_bytes());
        payload.push(0);
        payload
    }

    /// Creates a complete URL entry box (header + payload).
    fn make_url_entry_self_contained() -> Vec<u8> {
        let payload = make_url_box_self_contained();
        let size = 8 + payload.len() as u32; // BoxHeader (8) + payload
        let mut data = make_box_header(size, b"url ");
        data.extend_from_slice(&payload);
        data
    }

    /// Creates a complete URL entry box with location.
    fn make_url_entry_with_location(location: &str) -> Vec<u8> {
        let payload = make_url_box_with_location(location);
        let size = 8 + payload.len() as u32;
        let mut data = make_box_header(size, b"url ");
        data.extend_from_slice(&payload);
        data
    }

    /// Creates a complete URN entry box.
    fn make_urn_entry(name: &str, location: &str) -> Vec<u8> {
        let payload = make_urn_box_payload(name, location);
        let size = 8 + payload.len() as u32;
        let mut data = make_box_header(size, b"urn ");
        data.extend_from_slice(&payload);
        data
    }

    /// Creates a dref box payload.
    fn make_dref_payload(entries: Vec<Vec<u8>>) -> Vec<u8> {
        let mut payload = Vec::new();
        // FullBoxHeader: version=0, flags=0
        payload.extend_from_slice(&make_full_box_header(0, 0));
        // entry_count
        payload.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        // entries
        for entry in entries {
            payload.extend_from_slice(&entry);
        }
        payload
    }

    // UrlBoxView tests

    #[test]
    fn parse_url_box_self_contained() {
        let payload = make_url_box_self_contained();
        let url_box = UrlBoxView::parse(&payload).unwrap();

        assert_eq!(url_box.version, 0);
        assert!(url_box.flags.contains(UrlFlags::SELF_CONTAINED));
        assert!(url_box.location.is_none());
    }

    #[test]
    fn parse_url_box_with_location() {
        let location = "http://example.com/video.mp4";
        let payload = make_url_box_with_location(location);
        let url_box = UrlBoxView::parse(&payload).unwrap();

        assert_eq!(url_box.version, 0);
        assert!(!url_box.flags.contains(UrlFlags::SELF_CONTAINED));
        assert_eq!(url_box.location, Some(location));
    }

    // UrnBoxView tests

    #[test]
    fn parse_urn_box() {
        let name = "urn:example:video";
        let location = "http://example.com/video.mp4";
        let payload = make_urn_box_payload(name, location);
        let urn_box = UrnBoxView::parse(&payload).unwrap();

        assert_eq!(urn_box.version, 0);
        assert_eq!(urn_box.name, name);
        assert_eq!(urn_box.location, location);
    }

    // DrefBoxView tests

    #[test]
    fn parse_dref_empty() {
        let payload = make_dref_payload(vec![]);
        let dref = DrefBoxView::parse(&payload).unwrap();

        assert_eq!(dref.version, 0);
        assert_eq!(dref.entry_count, 0);
        assert_eq!(dref.entries().count(), 0);
    }

    #[test]
    fn parse_dref_single_url_self_contained() {
        let entries = vec![make_url_entry_self_contained()];
        let payload = make_dref_payload(entries);
        let dref = DrefBoxView::parse(&payload).unwrap();

        assert_eq!(dref.version, 0);
        assert_eq!(dref.entry_count, 1);

        let mut iter = dref.entries();
        let entry = iter.next().unwrap().unwrap();
        match entry {
            DrefEntryView::Url(url) => {
                assert!(url.flags.contains(UrlFlags::SELF_CONTAINED));
                assert!(url.location.is_none());
            }
            DrefEntryView::Urn(_) => panic!("Expected Url entry"),
        }
        assert!(iter.next().is_none());
    }

    #[test]
    fn parse_dref_single_url_with_location() {
        let location = "http://example.com/data.mp4";
        let entries = vec![make_url_entry_with_location(location)];
        let payload = make_dref_payload(entries);
        let dref = DrefBoxView::parse(&payload).unwrap();

        assert_eq!(dref.entry_count, 1);

        let entry = dref.entries().next().unwrap().unwrap();
        match entry {
            DrefEntryView::Url(url) => {
                assert!(!url.flags.contains(UrlFlags::SELF_CONTAINED));
                assert_eq!(url.location, Some(location));
            }
            DrefEntryView::Urn(_) => panic!("Expected Url entry"),
        }
    }

    #[test]
    fn parse_dref_single_urn() {
        let name = "urn:example:resource";
        let location = "http://example.com/resource";
        let entries = vec![make_urn_entry(name, location)];
        let payload = make_dref_payload(entries);
        let dref = DrefBoxView::parse(&payload).unwrap();

        assert_eq!(dref.entry_count, 1);

        let entry = dref.entries().next().unwrap().unwrap();
        match entry {
            DrefEntryView::Urn(urn) => {
                assert_eq!(urn.name, name);
                assert_eq!(urn.location, location);
            }
            DrefEntryView::Url(_) => panic!("Expected Urn entry"),
        }
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

        // First: self-contained URL
        match results[0].as_ref().unwrap() {
            DrefEntryView::Url(url) => {
                assert!(url.flags.contains(UrlFlags::SELF_CONTAINED));
            }
            _ => panic!("Expected Url entry"),
        }

        // Second: URL with location
        match results[1].as_ref().unwrap() {
            DrefEntryView::Url(url) => {
                assert_eq!(url.location, Some("http://example.com/a.mp4"));
            }
            _ => panic!("Expected Url entry"),
        }

        // Third: URN
        match results[2].as_ref().unwrap() {
            DrefEntryView::Urn(urn) => {
                assert_eq!(urn.name, "urn:test");
                assert_eq!(urn.location, "http://test.com");
            }
            _ => panic!("Expected Urn entry"),
        }
    }

    // Error case tests

    #[test]
    fn parse_dref_entry_count_mismatch() {
        let mut payload = make_full_box_header(0, 0);
        // Declare 2 entries but provide only 1
        payload.extend_from_slice(&2u32.to_be_bytes());
        payload.extend_from_slice(&make_url_entry_self_contained());

        let result = DrefBoxView::parse(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn parse_dref_invalid_entry_type() {
        // Create an entry with an invalid box type
        let mut invalid_entry = make_box_header(12, b"xxxx");
        invalid_entry.extend_from_slice(&make_full_box_header(0, 0));

        let payload = make_dref_payload(vec![invalid_entry]);
        let dref = DrefBoxView::parse(&payload).unwrap();

        // Parsing the dref itself succeeds, but iterating over entries should fail
        let mut iter = dref.entries();
        let entry = iter.next().unwrap();
        assert!(entry.is_err());
    }

    // UrlFlags tests

    #[test]
    fn url_flags_self_contained() {
        assert_eq!(UrlFlags::SELF_CONTAINED.get(), 0x000001);
    }

    // to_owned tests (requires alloc feature)

    #[cfg(feature = "alloc")]
    mod alloc_tests {
        use super::*;

        #[test]
        fn url_box_ref_to_owned() {
            let payload = make_url_box_with_location("http://example.com/test.mp4");
            let url_ref = UrlBoxView::parse(&payload).unwrap();
            let url_owned = url_ref.to_owned();

            assert_eq!(url_owned.version, url_ref.version);
            assert_eq!(url_owned.flags.get(), url_ref.flags.get());
            assert_eq!(url_owned.location.as_deref(), url_ref.location);
        }

        #[test]
        fn urn_box_ref_to_owned() {
            let payload = make_urn_box_payload("urn:example", "http://example.com");
            let urn_ref = UrnBoxView::parse(&payload).unwrap();
            let urn_owned = urn_ref.to_owned();

            assert_eq!(urn_owned.version, urn_ref.version);
            assert_eq!(urn_owned.flags.get(), urn_ref.flags.get());
            assert_eq!(urn_owned.name.as_str(), urn_ref.name);
            assert_eq!(urn_owned.location.as_str(), urn_ref.location);
        }

        #[test]
        fn url_box_from_ref() {
            let payload = make_url_box_self_contained();
            let url_ref = UrlBoxView::parse(&payload).unwrap();
            let url_owned: UrlBox = url_ref.into();

            assert!(url_owned.flags.contains(UrlFlags::SELF_CONTAINED));
            assert!(url_owned.location.is_none());
        }

        #[test]
        fn urn_box_from_ref() {
            let payload = make_urn_box_payload("urn:test", "http://test.com");
            let urn_ref = UrnBoxView::parse(&payload).unwrap();
            let urn_owned: UrnBox = urn_ref.into();

            assert_eq!(urn_owned.name, "urn:test");
            assert_eq!(urn_owned.location, "http://test.com");
        }
    }
}
