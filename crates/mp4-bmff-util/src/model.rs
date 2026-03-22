use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub struct Sample<D = Vec<u8>> {
    pub track_id: u32,
    pub decode_time: u64,
    pub composition_time: i64,
    pub duration: u32,
    pub is_sync: bool,
    pub data: D,
}
