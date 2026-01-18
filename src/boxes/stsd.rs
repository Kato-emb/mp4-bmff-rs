//!

pub enum SampleEntryView<'a> {
    // Placeholder for actual sample entry variants
    _Phantom(&'a ()),
}

pub struct StsdBoxView<'a> {
    entries: &'a [u8],
}
