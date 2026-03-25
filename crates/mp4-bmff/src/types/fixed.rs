//! Fixed-point number types for BMFF fields.
//!
//! BMFF uses fixed-point numbers to represent fractional values with exact
//! binary representation. This module provides a generic [`Fixed`] type and
//! common type aliases used throughout the specification.
//!
//! # Type Aliases
//!
//! | Type | Storage | Integer Bits | Fractional Bits | Range |
//! |------|---------|--------------|-----------------|-------|
//! | [`I8F8`] | `i16` | 8 (signed) | 8 | -128.0 to ~127.996 |
//! | [`U8F8`] | `u16` | 8 (unsigned) | 8 | 0.0 to ~255.996 |
//! | [`I16F16`] | `i32` | 16 (signed) | 16 | -32768.0 to ~32767.999 |
//! | [`U16F16`] | `u32` | 16 (unsigned) | 16 | 0.0 to ~65535.999 |
//! | [`I2F30`] | `i32` | 2 (signed) | 30 | -2.0 to ~1.999 |
//!
//! # Usage
//!
//! ```
//! use mp4_bmff::types::{I16F16, U16F16};
//!
//! // Create from raw storage value
//! let resolution = U16F16::from_raw(0x00480000); // 72.0 dpi
//! assert_eq!(resolution.integer(), 72);
//! assert_eq!(resolution.fraction_bits(), 0);
//!
//! // Access components
//! let value = I16F16::from_raw(0x0001_8000); // 1.5
//! assert_eq!(value.integer(), 1);
//! assert_eq!(value.to_f64(), 1.5);
//! ```

use core::fmt;

/// Trait implemented by primitive integer types that back a [`Fixed`].
///
/// This trait centralizes masking, sign-extension, and clamping logic,
/// keeping the [`Fixed`] implementation clean and usable in `no_std`
/// environments without external dependencies.
pub trait FixedStorage: Copy + Ord {
    /// Total bit width of the storage type.
    const BITS: u32;

    /// Whether the storage type is signed.
    const IS_SIGNED: bool;

    /// Return the underlying raw bits as an unsigned integer.
    fn to_bits(self) -> u128;

    /// Build the storage value from raw bits (low `BITS` bits are used).
    fn from_bits(bits: u128) -> Self;

    /// Clamp a raw integer into the range representable by this storage.
    fn clamp_raw(raw: i128) -> i128 {
        raw.clamp(Self::min_raw(), Self::max_raw())
    }

    fn min_raw() -> i128 {
        if Self::IS_SIGNED {
            if Self::BITS == 0 {
                0
            } else if Self::BITS >= 128 {
                i128::MIN
            } else {
                -((1i128) << (Self::BITS - 1))
            }
        } else {
            0
        }
    }

    fn max_raw() -> i128 {
        if Self::IS_SIGNED {
            if Self::BITS == 0 {
                0
            } else if Self::BITS >= 128 {
                i128::MAX
            } else {
                ((1i128) << (Self::BITS - 1)) - 1
            }
        } else if Self::BITS >= 127 {
            i128::MAX
        } else {
            ((1i128) << Self::BITS) - 1
        }
    }

    fn bit_mask() -> u128 {
        ones(Self::BITS)
    }

    /// Return the sign-extended raw integer represented by this storage.
    #[inline]
    fn to_raw_value(self) -> i128 {
        if Self::IS_SIGNED {
            sign_extend(self.to_bits(), Self::BITS)
        } else {
            self.to_bits().cast_signed()
        }
    }

    /// Clamp an `i128` into range and turn it into the storage type.
    #[inline]
    fn from_raw_value(raw: i128) -> Self {
        let clamped = Self::clamp_raw(raw);
        let bits = clamped.cast_unsigned() & Self::bit_mask();
        Self::from_bits(bits)
    }
}

#[inline]
const fn ones(bits: u32) -> u128 {
    if bits == 0 {
        0
    } else if bits >= 128 {
        u128::MAX
    } else {
        (1u128 << bits) - 1
    }
}

macro_rules! impl_fixed_storage_signed {
    ($($ty:ty),* $(,)?) => {
        $(
            impl FixedStorage for $ty {
                #[allow(clippy::cast_possible_truncation)]
                const BITS: u32 = (core::mem::size_of::<Self>() * 8) as u32;
                const IS_SIGNED: bool = true;

                #[inline]
                fn to_bits(self) -> u128 {
                    i128::from(self).cast_unsigned() & Self::bit_mask()
                }

                #[inline]
                #[allow(clippy::cast_possible_truncation)]
                fn from_bits(bits: u128) -> Self {
                    let mask = Self::bit_mask();
                    let bits = bits & mask;
                    let shift = 128 - Self::BITS;
                    let signed = (bits << shift).cast_signed() >> shift;
                    signed as Self
                }
            }
        )*
    };
}

macro_rules! impl_fixed_storage_unsigned {
    ($($ty:ty),* $(,)?) => {
        $(
            impl FixedStorage for $ty {
                #[allow(clippy::cast_possible_truncation)]
                const BITS: u32 = (core::mem::size_of::<Self>() * 8) as u32;
                const IS_SIGNED: bool = false;

                #[inline]
                fn to_bits(self) -> u128 {
                    u128::from(self)
                }

                #[inline]
                #[allow(clippy::cast_possible_truncation)]
                fn from_bits(bits: u128) -> Self {
                    (bits & Self::bit_mask()) as Self
                }
            }
        )*
    };
}

impl_fixed_storage_signed!(i8, i16, i32, i64);
impl_fixed_storage_unsigned!(u8, u16, u32, u64);

/// Generic fixed-point number with configurable storage and precision.
///
/// A fixed-point number stores a fractional value as an integer scaled by
/// a power of two. The `FRACTIONAL` parameter specifies how many bits are
/// used for the fractional part.
///
/// # Type Parameters
///
/// - `Storage`: The underlying integer type (`i16`, `u16`, `i32`, `u32`, etc.).
/// - `FRACTIONAL`: Number of bits used for the fractional part.
///
/// # Memory Layout
///
/// The storage is divided into integer and fractional parts:
/// ```text
/// |<-- INTEGER_BITS -->|<-- FRACTIONAL bits -->|
/// [  integer part      |   fractional part     ]
/// ```
///
/// # Conversions
///
/// - `from_raw()` / `to_raw()`: Direct access to storage value.
/// - `from_integer()`: Create from whole number.
/// - `to_f64()` / `to_f32()`: Convert to floating-point.
/// - `from_f64()` / `from_f32()`: Convert from floating-point.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Fixed<Storage, const FRACTIONAL: u32>
where
    Storage: FixedStorage,
{
    raw: Storage,
}

impl<Storage, const FRACTIONAL: u32> Fixed<Storage, FRACTIONAL>
where
    Storage: FixedStorage,
{
    /// Number of fractional bits carried by this fixed-point representation.
    pub const FRACTIONAL_BITS: u32 = FRACTIONAL;

    /// Number of integer bits (including the sign bit for signed storages).
    pub const INTEGER_BITS: u32 = Storage::BITS.saturating_sub(FRACTIONAL);

    /// Construct from a raw storage value.
    #[inline]
    pub const fn from_raw(raw: Storage) -> Self {
        Fixed { raw }
    }

    /// Return the underlying raw storage value.
    #[inline]
    pub const fn to_raw(self) -> Storage {
        self.raw
    }

    /// Raw bits reinterpreted as an unsigned integer.
    #[inline]
    pub fn raw_bits(self) -> u128 {
        self.raw.to_bits()
    }

    /// Fixed-point value interpreted as a signed integer scaled by `2^-FRACTIONAL`.
    #[inline]
    pub fn to_i128_raw(self) -> i128 {
        self.raw.to_raw_value()
    }

    /// Integer component (truncating toward zero).
    #[inline]
    pub fn integer(self) -> i128 {
        if FRACTIONAL == 0 {
            return self.to_i128_raw();
        }
        if FRACTIONAL >= i128::BITS {
            return 0;
        }

        let value = self.to_i128_raw();
        let shifted = value >> FRACTIONAL;
        if value >= 0 || self.fraction_bits() == 0 {
            shifted
        } else {
            shifted + 1
        }
    }

    /// Fraction component as an unsigned integer in the range `[0, 2^FRACTIONAL)`.
    #[inline]
    pub fn fraction_bits(self) -> u128 {
        if FRACTIONAL == 0 {
            0
        } else {
            self.raw_bits() & ones(FRACTIONAL)
        }
    }

    /// Convert to `f64`.
    #[inline]
    #[allow(clippy::cast_precision_loss)]
    pub fn to_f64(self) -> f64 {
        self.to_i128_raw() as f64 / pow2_f64(FRACTIONAL)
    }

    /// Convert to `f32`.
    #[inline]
    #[allow(clippy::cast_possible_truncation)]
    pub fn to_f32(self) -> f32 {
        self.to_f64() as f32
    }

    /// Construct from a raw scaled integer (already multiplied by `2^FRACTIONAL`).
    fn from_raw_value(raw: i128) -> Self {
        Fixed::from_raw(Storage::from_raw_value(raw))
    }

    /// Construct from an integer.
    #[inline]
    pub fn from_integer(integer: i128) -> Self {
        let scale = scaling_factor(FRACTIONAL);
        Fixed::from_raw_value(integer.saturating_mul(scale))
    }

    /// Construct from `f64`, saturating to the representable range.
    ///
    /// Special values are handled via the saturating semantics of `as i128`:
    /// `NaN` → 0, `±Infinity` → `i128::MAX`/`MIN` → clamped to storage range.
    #[inline]
    #[allow(clippy::cast_possible_truncation)]
    pub fn from_f64(value: f64) -> Self {
        let scaled = value * pow2_f64(FRACTIONAL);
        let rounded = if scaled >= 0.0 {
            scaled + 0.5
        } else {
            scaled - 0.5
        };
        Fixed::from_raw_value(rounded as i128)
    }

    /// Construct from `f32`, saturating on overflow/underflow.
    #[inline]
    pub fn from_f32(value: f32) -> Self {
        Fixed::from_f64(f64::from(value))
    }

    /// Smallest representable value.
    #[inline]
    pub fn min_value() -> Self {
        Fixed::from_raw_value(Storage::min_raw())
    }

    /// Largest representable value.
    #[inline]
    pub fn max_value() -> Self {
        Fixed::from_raw_value(Storage::max_raw())
    }
}

#[inline]
fn scaling_factor(bits: u32) -> i128 {
    let scale_u = if bits == 0 {
        1
    } else if bits >= 128 {
        u128::MAX
    } else {
        1u128 << bits
    };

    if scale_u > i128::MAX.cast_unsigned() {
        i128::MAX
    } else {
        scale_u.cast_signed()
    }
}

/// Return `2^n` as an exact `f64` (IEEE 754 exponent construction).
#[inline]
fn pow2_f64(n: u32) -> f64 {
    if n >= 1024 {
        f64::INFINITY
    } else {
        f64::from_bits((u64::from(n) + 1023) << 52)
    }
}

#[inline]
fn sign_extend(bits: u128, width: u32) -> i128 {
    if width >= 128 {
        bits.cast_signed()
    } else {
        let shift = 128 - width;
        (bits << shift).cast_signed() >> shift
    }
}

impl<Storage, const FRACTIONAL: u32> fmt::Debug for Fixed<Storage, FRACTIONAL>
where
    Storage: FixedStorage + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Fixed")
            .field("raw", &self.raw)
            .field("value", &self.to_f64())
            .field("fractional_bits", &FRACTIONAL)
            .finish()
    }
}

impl<Storage, const FRACTIONAL: u32> fmt::Display for Fixed<Storage, FRACTIONAL>
where
    Storage: FixedStorage,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_f64())
    }
}

impl<Storage, const FRACTIONAL: u32> Default for Fixed<Storage, FRACTIONAL>
where
    Storage: FixedStorage + Default,
{
    fn default() -> Self {
        Fixed::from_raw(Storage::default())
    }
}

impl<Storage, const FRACTIONAL: u32> From<Storage> for Fixed<Storage, FRACTIONAL>
where
    Storage: FixedStorage,
{
    fn from(raw: Storage) -> Self {
        Fixed::from_raw(raw)
    }
}

impl<Storage, const FRACTIONAL: u32> From<f32> for Fixed<Storage, FRACTIONAL>
where
    Storage: FixedStorage,
{
    fn from(value: f32) -> Self {
        Fixed::from_f32(value)
    }
}

impl<Storage, const FRACTIONAL: u32> From<f64> for Fixed<Storage, FRACTIONAL>
where
    Storage: FixedStorage,
{
    fn from(value: f64) -> Self {
        Fixed::from_f64(value)
    }
}

impl<Storage, const FRACTIONAL: u32> From<Fixed<Storage, FRACTIONAL>> for f32
where
    Storage: FixedStorage,
{
    fn from(value: Fixed<Storage, FRACTIONAL>) -> Self {
        value.to_f32()
    }
}

impl<Storage, const FRACTIONAL: u32> From<Fixed<Storage, FRACTIONAL>> for f64
where
    Storage: FixedStorage,
{
    fn from(value: Fixed<Storage, FRACTIONAL>) -> Self {
        value.to_f64()
    }
}

/// Signed 8.8 fixed-point number (16-bit storage).
///
/// Used for fields requiring moderate precision with signed values,
/// such as audio balance (-1.0 to +1.0 range).
pub type I8F8 = Fixed<i16, 8>;

/// Unsigned 8.8 fixed-point number (16-bit storage).
///
/// Used for fields requiring moderate precision with positive values only.
pub type U8F8 = Fixed<u16, 8>;

/// Unsigned 16.16 fixed-point number (32-bit storage).
///
/// Commonly used for resolutions (72.0 dpi = 0x00480000) and
/// sample rates in media headers.
pub type U16F16 = Fixed<u32, 16>;

/// Signed 16.16 fixed-point number (32-bit storage).
///
/// Used for transformation matrix elements (a, b, c, d, x, y)
/// and other values requiring signed fractional precision.
pub type I16F16 = Fixed<i32, 16>;

/// Signed 2.30 fixed-point number (32-bit storage).
///
/// Used for transformation matrix projective terms (u, v, w)
/// which are typically 0.0, 0.0, and 1.0 for affine transforms.
pub type I2F30 = Fixed<i32, 30>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_f32() {
        let values = [0.0, 1.0, -1.0, 0.5, 123.125];
        for &value in &values {
            let fixed = I16F16::from(value);
            let back: f32 = fixed.into();
            assert!((back - value).abs() < 0.0001);
        }
    }

    #[test]
    fn integer_and_fraction() {
        let fixed = I8F8::from_f32(42.5);
        assert_eq!(fixed.integer(), 42);
        assert_eq!(fixed.fraction_bits(), 0x80);
    }

    #[test]
    fn integer_truncates_toward_zero_for_negatives() {
        let fixed = I8F8::from_f32(-3.75);
        assert_eq!(fixed.integer(), -3);
    }

    #[test]
    fn integer_is_zero_when_fraction_bits_dominate() {
        type I16F130 = Fixed<i16, 130>;
        let value = I16F130::from_raw(i16::MIN);
        assert_eq!(value.integer(), 0);
    }

    #[test]
    fn saturates_on_overflow() {
        let big = I2F30::from_f64(1000.0);
        assert_eq!(big, I2F30::max_value());

        let small = U16F16::from_f64(-5.0);
        assert_eq!(small.raw_bits(), 0);
    }

    #[test]
    fn from_integer_scales_and_saturates() {
        type I4F12 = Fixed<i16, 12>;
        let exact = I4F12::from_integer(5);
        assert_eq!(exact.integer(), 5);
        assert_eq!(exact.fraction_bits(), 0);

        let maxed = I4F12::from_integer(i128::MAX);
        assert_eq!(maxed, I4F12::max_value());

        type U4F12 = Fixed<u16, 12>;
        let clipped = U4F12::from_integer(-3);
        assert_eq!(clipped.raw_bits(), 0);
    }

    #[test]
    fn from_f64_handles_special_values() {
        let nan = I16F16::from_f64(f64::NAN);
        assert_eq!(nan.raw_bits(), 0);

        let pos_inf = I16F16::from_f64(f64::INFINITY);
        assert_eq!(pos_inf, I16F16::max_value());

        let neg_inf = I16F16::from_f64(f64::NEG_INFINITY);
        assert_eq!(neg_inf, I16F16::min_value());

        let rounded = I8F8::from_f64(1.5);
        assert_eq!(rounded.raw_bits(), 0x0180);
    }

    #[test]
    fn min_and_max_match_storage_bounds() {
        let min = I8F8::min_value();
        let max = I8F8::max_value();
        assert_eq!(min.to_raw(), i16::MIN);
        assert_eq!(max.to_raw(), i16::MAX);
    }

    #[test]
    fn fraction_bits_zero_when_no_fractional_part() {
        type I32F0 = Fixed<i32, 0>;
        let value = I32F0::from_integer(123);
        assert_eq!(value.fraction_bits(), 0);
        assert_eq!(value.integer(), 123);
    }

    #[test]
    fn floats_round_trip_with_reasonable_precision() {
        let value = I8F8::from_f64(-2.25);
        assert!((value.to_f64() - -2.25).abs() < 1e-6);
        assert!((value.to_f32() - -2.25).abs() < 1e-6);
    }
}
