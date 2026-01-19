use core::mem;

/// Size of the descriptor instance
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizeOfInstance(u32);

impl SizeOfInstance {
    /// Maximum size representable in SizeOfInstance
    pub const MAX: SizeOfInstance = SizeOfInstance(0x0FFFFFFF);

    /// Returns the size as a u32 value
    pub fn get(&self) -> u32 {
        self.0
    }

    /// Creates a SizeOfInstance from a u32 value
    /// Returns None if the value exceeds the maximum representable size
    pub fn from_u32(size: u32) -> Option<Self> {
        if size <= Self::MAX.0 {
            Some(SizeOfInstance(size))
        } else {
            None
        }
    }

    /// Parses SizeOfInstance from a byte slice
    pub fn from_bytes(bytes: &[u8]) -> Option<(Self, usize)> {
        const MAX_BYTES: usize = mem::size_of::<u32>();

        let mut size: u32 = 0;
        let mut num_bytes = 0;

        while num_bytes < MAX_BYTES {
            let next_type = (bytes.get(num_bytes)? & 0x80) != 0;
            let size_byte = (bytes.get(num_bytes)? & 0x7F) as u32;
            size = (size << 7) | size_byte;
            num_bytes += 1;

            if !next_type {
                return Some((SizeOfInstance(size), num_bytes));
            }
        }

        None
    }
}
