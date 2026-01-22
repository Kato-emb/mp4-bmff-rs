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

    /// Serializes SizeOfInstance to a byte array
    /// Returns the byte array and the number of bytes used.
    /// The bytes are stored at the beginning of the array (indices 0..num_bytes).
    pub fn to_bytes(&self) -> ([u8; 4], usize) {
        let size = self.0;
        let num_bytes = self.size_in_bytes();
        let mut bytes = [0u8; 4];

        for i in 0..num_bytes {
            let shift = 7 * (num_bytes - 1 - i);
            let byte = ((size >> shift) & 0x7F) as u8;
            if i < num_bytes - 1 {
                bytes[i] = byte | 0x80; // continuation bit
            } else {
                bytes[i] = byte; // last byte, no continuation
            }
        }

        (bytes, num_bytes)
    }

    /// Returns the number of bytes required to serialize this SizeOfInstance
    pub fn size_in_bytes(&self) -> usize {
        if self.0 < 0x80 {
            1
        } else if self.0 < 0x4000 {
            2
        } else if self.0 < 0x200000 {
            3
        } else {
            4
        }
    }
}
