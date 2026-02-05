/// Size of an instance in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizeOfInstance(u32);

impl SizeOfInstance {
    /// Maximum size representable in SizeOfInstance (2^28 - 1)
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
        const MAX_BYTES: usize = 4;

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

        for (i, byte) in bytes.iter_mut().enumerate().take(num_bytes) {
            let shift = 7 * (num_bytes - 1 - i);
            let byte_val = ((size >> shift) & 0x7F) as u8;
            if i < num_bytes - 1 {
                *byte = byte_val | 0x80; // continuation bit
            } else {
                *byte = byte_val; // last byte, no continuation
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_constant() {
        assert_eq!(SizeOfInstance::MAX.get(), 0x0FFFFFFF);
    }

    #[test]
    fn test_get() {
        let size = SizeOfInstance::from_u32(42).unwrap();
        assert_eq!(size.get(), 42);
    }

    #[test]
    fn test_from_u32_valid() {
        // 最小値
        let size = SizeOfInstance::from_u32(0);
        assert!(size.is_some());
        assert_eq!(size.unwrap().get(), 0);

        // 通常の値
        let size = SizeOfInstance::from_u32(1000);
        assert!(size.is_some());
        assert_eq!(size.unwrap().get(), 1000);

        // 最大値
        let size = SizeOfInstance::from_u32(0x0FFFFFFF);
        assert!(size.is_some());
        assert_eq!(size.unwrap().get(), 0x0FFFFFFF);
    }

    #[test]
    fn test_from_u32_invalid() {
        // 最大値を超える値
        let size = SizeOfInstance::from_u32(0x10000000);
        assert!(size.is_none());

        let size = SizeOfInstance::from_u32(u32::MAX);
        assert!(size.is_none());
    }

    #[test]
    fn test_from_bytes_one_byte() {
        // 0x00 (0) - 継続ビットなし
        let bytes = [0x00];
        let result = SizeOfInstance::from_bytes(&bytes);
        assert!(result.is_some());
        let (size, num_bytes) = result.unwrap();
        assert_eq!(size.get(), 0);
        assert_eq!(num_bytes, 1);

        // 0x7F (127) - 継続ビットなし、1バイト最大値
        let bytes = [0x7F];
        let result = SizeOfInstance::from_bytes(&bytes);
        assert!(result.is_some());
        let (size, num_bytes) = result.unwrap();
        assert_eq!(size.get(), 127);
        assert_eq!(num_bytes, 1);
    }

    #[test]
    fn test_from_bytes_two_bytes() {
        // 0x81 0x00 (128) - 継続ビットあり + 終端
        let bytes = [0x81, 0x00];
        let result = SizeOfInstance::from_bytes(&bytes);
        assert!(result.is_some());
        let (size, num_bytes) = result.unwrap();
        assert_eq!(size.get(), 128);
        assert_eq!(num_bytes, 2);

        // 0xFF 0x7F (16383) - 2バイト最大値
        let bytes = [0xFF, 0x7F];
        let result = SizeOfInstance::from_bytes(&bytes);
        assert!(result.is_some());
        let (size, num_bytes) = result.unwrap();
        assert_eq!(size.get(), 16383);
        assert_eq!(num_bytes, 2);
    }

    #[test]
    fn test_from_bytes_three_bytes() {
        // 0x81 0x80 0x00 (16384)
        let bytes = [0x81, 0x80, 0x00];
        let result = SizeOfInstance::from_bytes(&bytes);
        assert!(result.is_some());
        let (size, num_bytes) = result.unwrap();
        assert_eq!(size.get(), 16384);
        assert_eq!(num_bytes, 3);
    }

    #[test]
    fn test_from_bytes_four_bytes() {
        // 0x81 0x80 0x80 0x00 (2097152)
        let bytes = [0x81, 0x80, 0x80, 0x00];
        let result = SizeOfInstance::from_bytes(&bytes);
        assert!(result.is_some());
        let (size, num_bytes) = result.unwrap();
        assert_eq!(size.get(), 2097152);
        assert_eq!(num_bytes, 4);

        // 最大値 0x0FFFFFFF
        let bytes = [0xFF, 0xFF, 0xFF, 0x7F];
        let result = SizeOfInstance::from_bytes(&bytes);
        assert!(result.is_some());
        let (size, num_bytes) = result.unwrap();
        assert_eq!(size.get(), 0x0FFFFFFF);
        assert_eq!(num_bytes, 4);
    }

    #[test]
    fn test_from_bytes_empty() {
        let bytes: [u8; 0] = [];
        let result = SizeOfInstance::from_bytes(&bytes);
        assert!(result.is_none());
    }

    #[test]
    fn test_from_bytes_no_termination() {
        // 4バイト全てに継続ビットが設定されている場合
        let bytes = [0x80, 0x80, 0x80, 0x80];
        let result = SizeOfInstance::from_bytes(&bytes);
        assert!(result.is_none());
    }

    #[test]
    fn test_to_bytes_one_byte() {
        let size = SizeOfInstance::from_u32(0).unwrap();
        let (bytes, num_bytes) = size.to_bytes();
        assert_eq!(num_bytes, 1);
        assert_eq!(bytes[0], 0x00);

        let size = SizeOfInstance::from_u32(127).unwrap();
        let (bytes, num_bytes) = size.to_bytes();
        assert_eq!(num_bytes, 1);
        assert_eq!(bytes[0], 0x7F);
    }

    #[test]
    fn test_to_bytes_two_bytes() {
        let size = SizeOfInstance::from_u32(128).unwrap();
        let (bytes, num_bytes) = size.to_bytes();
        assert_eq!(num_bytes, 2);
        assert_eq!(bytes[0], 0x81);
        assert_eq!(bytes[1], 0x00);

        let size = SizeOfInstance::from_u32(16383).unwrap();
        let (bytes, num_bytes) = size.to_bytes();
        assert_eq!(num_bytes, 2);
        assert_eq!(bytes[0], 0xFF);
        assert_eq!(bytes[1], 0x7F);
    }

    #[test]
    fn test_to_bytes_three_bytes() {
        let size = SizeOfInstance::from_u32(16384).unwrap();
        let (bytes, num_bytes) = size.to_bytes();
        assert_eq!(num_bytes, 3);
        assert_eq!(bytes[0], 0x81);
        assert_eq!(bytes[1], 0x80);
        assert_eq!(bytes[2], 0x00);
    }

    #[test]
    fn test_to_bytes_four_bytes() {
        let size = SizeOfInstance::from_u32(2097152).unwrap();
        let (bytes, num_bytes) = size.to_bytes();
        assert_eq!(num_bytes, 4);
        assert_eq!(bytes[0], 0x81);
        assert_eq!(bytes[1], 0x80);
        assert_eq!(bytes[2], 0x80);
        assert_eq!(bytes[3], 0x00);

        let size = SizeOfInstance::from_u32(0x0FFFFFFF).unwrap();
        let (bytes, num_bytes) = size.to_bytes();
        assert_eq!(num_bytes, 4);
        assert_eq!(bytes[0], 0xFF);
        assert_eq!(bytes[1], 0xFF);
        assert_eq!(bytes[2], 0xFF);
        assert_eq!(bytes[3], 0x7F);
    }

    #[test]
    fn test_size_in_bytes() {
        // 1バイト: 0 - 127
        assert_eq!(SizeOfInstance::from_u32(0).unwrap().size_in_bytes(), 1);
        assert_eq!(SizeOfInstance::from_u32(127).unwrap().size_in_bytes(), 1);

        // 2バイト: 128 - 16383
        assert_eq!(SizeOfInstance::from_u32(128).unwrap().size_in_bytes(), 2);
        assert_eq!(SizeOfInstance::from_u32(16383).unwrap().size_in_bytes(), 2);

        // 3バイト: 16384 - 2097151
        assert_eq!(SizeOfInstance::from_u32(16384).unwrap().size_in_bytes(), 3);
        assert_eq!(SizeOfInstance::from_u32(2097151).unwrap().size_in_bytes(), 3);

        // 4バイト: 2097152 - 0x0FFFFFFF
        assert_eq!(SizeOfInstance::from_u32(2097152).unwrap().size_in_bytes(), 4);
        assert_eq!(SizeOfInstance::from_u32(0x0FFFFFFF).unwrap().size_in_bytes(), 4);
    }

    #[test]
    fn test_roundtrip() {
        // 様々な値でシリアライズ→デシリアライズの往復テスト
        let test_values = [0, 1, 127, 128, 255, 16383, 16384, 2097151, 2097152, 0x0FFFFFFF];

        for &value in &test_values {
            let original = SizeOfInstance::from_u32(value).unwrap();
            let (bytes, num_bytes) = original.to_bytes();
            let (restored, restored_num_bytes) = SizeOfInstance::from_bytes(&bytes[..num_bytes]).unwrap();

            assert_eq!(original.get(), restored.get(), "Roundtrip failed for value {}", value);
            assert_eq!(num_bytes, restored_num_bytes, "Byte count mismatch for value {}", value);
        }
    }

    #[test]
    fn test_equality() {
        let a = SizeOfInstance::from_u32(100).unwrap();
        let b = SizeOfInstance::from_u32(100).unwrap();
        let c = SizeOfInstance::from_u32(200).unwrap();

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_clone_and_copy() {
        let original = SizeOfInstance::from_u32(42).unwrap();
        let cloned = original.clone();
        let copied = original;

        assert_eq!(original.get(), cloned.get());
        assert_eq!(original.get(), copied.get());
    }
}
