//! Fraction type for rational number fields.
//!
//! Some BMFF boxes use rational numbers represented as a numerator/denominator
//! pair. This module provides a generic [`Fraction<T>`] type and a common
//! type alias [`Fraction32`] for unsigned 32-bit fractions.
//!
//! # Usage
//!
//! ```
//! use mp4_bmff::types::FractionU32;
//!
//! let half = FractionU32::new(1, 2);
//! assert_eq!(*half.numer(), 1);
//! assert_eq!(*half.denom(), 2);
//! ```

use core::fmt;

/// A fraction represented as a numerator/denominator pair.
///
/// The type parameter `T` determines the storage type for both the
/// numerator and denominator.
///
/// # Example
///
/// ```
/// use mp4_bmff::types::Fraction;
///
/// let value = Fraction::<u32>::new(720, 1);
/// assert_eq!(*value.numer(), 720);
/// assert_eq!(*value.denom(), 1);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fraction<T> {
    numerator: T,
    denominator: T,
}

/// Unsigned 32-bit fraction.
pub type FractionU32 = Fraction<u32>;

impl<T> Fraction<T> {
    /// Creates a new fraction from numerator and denominator.
    ///
    /// # Note
    ///
    /// This constructor does not validate that the denominator is non-zero.
    /// A zero denominator may be present in malformed BMFF data. Methods
    /// that compute a value from the fraction (e.g. `to_f64`) will return
    /// `None` when the denominator is zero.
    pub const fn new(numerator: T, denominator: T) -> Self {
        Self {
            numerator,
            denominator,
        }
    }

    /// Returns the raw numerator and denominator as a tuple.
    pub fn into_raw(self) -> (T, T) {
        (self.numerator, self.denominator)
    }

    /// Returns the numerator.
    #[inline]
    pub const fn numer(&self) -> &T {
        &self.numerator
    }

    /// Returns the denominator.
    #[inline]
    pub const fn denom(&self) -> &T {
        &self.denominator
    }
}

impl FractionU32 {
    /// Converts the fraction to an `f32` value.
    ///
    /// Returns `None` if the denominator is zero.
    pub fn to_f32(self) -> Option<f32> {
        if self.denominator == 0 {
            return None;
        }

        Some(self.numerator as f32 / self.denominator as f32)
    }

    /// Converts the fraction to an `f64` value.
    ///
    /// Returns `None` if the denominator is zero.
    pub fn to_f64(self) -> Option<f64> {
        if self.denominator == 0 {
            return None;
        }

        Some(self.numerator as f64 / self.denominator as f64)
    }
}

impl<T: fmt::Display> fmt::Debug for Fraction<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}

impl<T: fmt::Display> fmt::Display for Fraction<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use super::*;

    #[test]
    fn accessors_and_equality() {
        let f = Fraction::new(3u32, 4u32);
        assert_eq!(*f.numer(), 3);
        assert_eq!(*f.denom(), 4);
        assert_eq!(f.into_raw(), (3, 4));

        // No reduction: 1/2 != 2/4
        assert_eq!(Fraction::new(1u32, 2u32), Fraction::new(1u32, 2u32));
        assert_ne!(Fraction::new(1u32, 2u32), Fraction::new(2u32, 4u32));
    }

    #[test]
    fn to_f64_and_to_f32() {
        assert_eq!(FractionU32::new(1, 2).to_f64(), Some(0.5));
        assert_eq!(FractionU32::new(720, 1).to_f64(), Some(720.0));
        assert_eq!(FractionU32::new(0, 1).to_f64(), Some(0.0));
        assert_eq!(FractionU32::new(1, 4).to_f32(), Some(0.25));

        // Zero denominator returns None
        assert_eq!(FractionU32::new(1, 0).to_f64(), None);
        assert_eq!(FractionU32::new(0, 0).to_f64(), None);
        assert_eq!(FractionU32::new(1, 0).to_f32(), None);
    }

    #[test]
    fn display_and_debug_formatting() {
        let f = Fraction::new(3u32, 4u32);
        assert_eq!(format!("{f}"), "3/4");
        assert_eq!(format!("{f:?}"), "3/4");
    }
}
