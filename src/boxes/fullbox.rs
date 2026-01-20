//! FullBox flags type for BMFF.
//!
//! In ISO base media file format, a `FullBox` extends the basic `Box` structure
//! with a version field (8 bits) and flags field (24 bits):
//!
//! ```text
//! aligned(8) class FullBox(unsigned int(32) boxtype, unsigned int(8) v, unsigned int(24) f)
//!     extends Box(boxtype) {
//!     unsigned int(8)   version = v;
//!     unsigned int(24)  flags = f;
//! }
//! ```
//!
//! This module provides [`FullBoxFlags`], a type-safe representation of the 24-bit
//! flags field with support for bitwise operations.
//!
//! # Type Parameter
//!
//! `FullBoxFlags<T>` uses a phantom type parameter `T` to distinguish flags for
//! different box types at compile time, preventing accidental mixing of flags
//! from different box types.
//!
//! # Example
//!
//! ```
//! use mp4_bmff::boxes::FullBoxFlags;
//!
//! // Define a marker type for a specific box
//! struct MyBox;
//!
//! let flags: FullBoxFlags<MyBox> = FullBoxFlags::new(0x000001);
//! assert!(flags.is_set(0));
//! assert!(!flags.is_set(1));
//!
//! // Combine flags with bitwise OR
//! let combined = flags | FullBoxFlags::new(0x000002);
//! assert_eq!(combined.get(), 0x000003);
//! ```

use core::fmt;
use core::marker;
use core::ops;

/// Represents the flags of a full box in BMFF.
#[derive(PartialEq, Eq, Hash)]
pub struct FullBoxFlags<T> {
    pub(crate) mask: u32,
    _marker: marker::PhantomData<T>,
}

impl<T> FullBoxFlags<T> {
    /// Creates an empty `FullBoxFlags` with all flags cleared.
    #[inline]
    pub const fn empty() -> Self {
        Self {
            mask: 0,
            _marker: marker::PhantomData,
        }
    }

    /// Creates a new `FullBoxFlags` with the given raw flags value.
    #[inline]
    pub const fn new(raw_flags: u32) -> Self {
        Self {
            mask: raw_flags & 0x00FF_FFFF,
            _marker: marker::PhantomData,
        }
    }

    /// Creates a new `FullBoxFlags` from a 3-byte array (big-endian).
    #[inline]
    pub const fn from_bytes(bytes: [u8; 3]) -> Self {
        let mask = ((bytes[0] as u32) << 16) | ((bytes[1] as u32) << 8) | (bytes[2] as u32);
        Self {
            mask,
            _marker: marker::PhantomData,
        }
    }

    /// Creates a `FullBoxFlags` from the given raw bits, truncating to 24 bits.
    #[inline]
    pub const fn from_bits_truncate(bits: u32) -> Self {
        FullBoxFlags::new(bits)
    }

    /// Returns the raw flags value.
    #[inline]
    pub const fn get(&self) -> u32 {
        self.mask
    }

    /// Checks if the flag at the given index is set.
    #[inline]
    pub fn is_set(&self, i: usize) -> bool {
        if i >= 24 {
            return false;
        }

        (self.mask & (1u32 << i)) != 0
    }

    /// Checks if no flags are set.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.mask == 0
    }

    /// Checks if all flags in `other` are set in `self`.
    #[inline]
    pub const fn contains(&self, other: FullBoxFlags<T>) -> bool {
        (self.mask & other.mask) == other.mask
    }
}

impl<T> fmt::Debug for FullBoxFlags<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FullBoxFlags({:#08X})", self.mask)
    }
}

impl<T> fmt::Display for FullBoxFlags<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#08X}", self.mask)
    }
}

impl<T> Clone for FullBoxFlags<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for FullBoxFlags<T> {}

impl<T> ops::BitOr for FullBoxFlags<T> {
    type Output = FullBoxFlags<T>;

    fn bitor(self, rhs: FullBoxFlags<T>) -> Self::Output {
        FullBoxFlags::new(self.mask | rhs.mask)
    }
}

impl<T> ops::BitOrAssign for FullBoxFlags<T> {
    fn bitor_assign(&mut self, rhs: FullBoxFlags<T>) {
        self.mask |= rhs.mask;
    }
}

impl<T> ops::BitAnd for FullBoxFlags<T> {
    type Output = FullBoxFlags<T>;

    fn bitand(self, rhs: FullBoxFlags<T>) -> Self::Output {
        FullBoxFlags::new(self.mask & rhs.mask)
    }
}

impl<T> ops::BitAndAssign for FullBoxFlags<T> {
    fn bitand_assign(&mut self, rhs: FullBoxFlags<T>) {
        self.mask &= rhs.mask;
    }
}

impl<T> ops::BitXor for FullBoxFlags<T> {
    type Output = FullBoxFlags<T>;

    fn bitxor(self, rhs: FullBoxFlags<T>) -> Self::Output {
        FullBoxFlags::new(self.mask ^ rhs.mask)
    }
}

impl<T> ops::BitXorAssign for FullBoxFlags<T> {
    fn bitxor_assign(&mut self, rhs: FullBoxFlags<T>) {
        self.mask ^= rhs.mask;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy)]
    struct TestBox;

    #[test]
    fn new_masks_to_24_bits() {
        assert_eq!(FullBoxFlags::<TestBox>::new(0x00FF_FFFF).get(), 0x00FF_FFFF);
        assert_eq!(FullBoxFlags::<TestBox>::new(0xFFFF_FFFF).get(), 0x00FF_FFFF);
        assert_eq!(FullBoxFlags::<TestBox>::new(0x0100_0000).get(), 0);
    }

    #[test]
    fn is_set() {
        let flags: FullBoxFlags<TestBox> = FullBoxFlags::new(0b101);
        assert!(flags.is_set(0));
        assert!(!flags.is_set(1));
        assert!(flags.is_set(2));
        // Out of range returns false
        assert!(!flags.is_set(24));
        assert!(!flags.is_set(usize::MAX));
    }

    #[test]
    fn contains() {
        let all: FullBoxFlags<TestBox> = FullBoxFlags::new(0b1111);
        let subset: FullBoxFlags<TestBox> = FullBoxFlags::new(0b0101);

        assert!(all.contains(subset));
        assert!(all.contains(FullBoxFlags::empty()));
        assert!(!subset.contains(all));
    }
}
