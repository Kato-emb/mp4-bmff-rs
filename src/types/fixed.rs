//! Fixed-point helpers used by various BMFF fields.
//!
//! The implementations here keep the math and encoding details in a single
//! place so higher-level boxes can simply use the type aliases (e.g. `I16F16`).

use core::fmt;

/// Trait implemented by primitive integer types that can back a [`Fixed`].
///
/// Keeping the masking, sign-extension, and clamping logic centralized in this
/// trait lets the [`Fixed`] implementation stay DRY and usable in `no_std`
/// environments without depending on external crates.
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
            self.to_bits() as i128
        }
    }

    /// Clamp an `i128` into range and turn it into the storage type.
    #[inline]
    fn from_raw_value(raw: i128) -> Self {
        let clamped = Self::clamp_raw(raw);
        let bits = (clamped as u128) & Self::bit_mask();
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
                const BITS: u32 = (core::mem::size_of::<Self>() * 8) as u32;
                const IS_SIGNED: bool = true;

                #[inline]
                fn to_bits(self) -> u128 {
                    (self as i128 as u128) & Self::bit_mask()
                }

                #[inline]
                fn from_bits(bits: u128) -> Self {
                    let mask = Self::bit_mask();
                    let bits = bits & mask;
                    let shift = 128 - Self::BITS;
                    let signed = ((bits << shift) as i128) >> shift;
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
                const BITS: u32 = (core::mem::size_of::<Self>() * 8) as u32;
                const IS_SIGNED: bool = false;

                #[inline]
                fn to_bits(self) -> u128 {
                    self as u128
                }

                #[inline]
                fn from_bits(bits: u128) -> Self {
                    (bits & Self::bit_mask()) as Self
                }
            }
        )*
    };
}

impl_fixed_storage_signed!(i8, i16, i32, i64);
impl_fixed_storage_unsigned!(u8, u16, u32, u64);

/// Generic fixed-point value that stores `FRACTIONAL` bits of fraction.
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
    pub fn to_f64(self) -> f64 {
        let scale = scaling_factor(FRACTIONAL) as f64;
        self.to_i128_raw() as f64 / scale
    }

    /// Convert to `f32`.
    #[inline]
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
    #[cfg(any(feature = "std", test))]
    pub fn from_f64(value: f64) -> Self {
        if value.is_nan() {
            return Fixed::from_raw_value(0);
        }

        if value.is_infinite() {
            return if value.is_sign_positive() {
                Fixed::max_value()
            } else {
                Fixed::min_value()
            };
        }

        let scale = scaling_factor(FRACTIONAL) as f64;
        let scaled = (value * scale).round();
        let limited = scaled.clamp(i128::MIN as f64, i128::MAX as f64);
        Fixed::from_raw_value(limited as i128)
    }

    /// Construct from `f32`, saturating on overflow/underflow.
    #[cfg(any(feature = "std", test))]
    #[inline]
    pub fn from_f32(value: f32) -> Self {
        Fixed::from_f64(value as f64)
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

    if scale_u > i128::MAX as u128 {
        i128::MAX
    } else {
        scale_u as i128
    }
}

#[inline]
fn sign_extend(bits: u128, width: u32) -> i128 {
    if width >= 128 {
        bits as i128
    } else {
        let shift = 128 - width;
        ((bits << shift) as i128) >> shift
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

impl<Storage, const FRACTIONAL: u32> From<Storage> for Fixed<Storage, FRACTIONAL>
where
    Storage: FixedStorage,
{
    fn from(raw: Storage) -> Self {
        Fixed::from_raw(raw)
    }
}

#[cfg(any(feature = "std", test))]
impl<Storage, const FRACTIONAL: u32> From<f32> for Fixed<Storage, FRACTIONAL>
where
    Storage: FixedStorage,
{
    fn from(value: f32) -> Self {
        Fixed::from_f32(value)
    }
}

#[cfg(feature = "std")]
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

/// Signed 8.8 fixed point (uses `i16` storage).
pub type I8F8 = Fixed<i16, 8>;

/// Unsigned 8.8 fixed point (uses `u16` storage).
pub type U8F8 = Fixed<u16, 8>;

/// Unsigned 16.16 fixed point (uses `u32` storage).
pub type U16F16 = Fixed<u32, 16>;

/// Signed 16.16 fixed point (uses `i32` storage).
pub type I16F16 = Fixed<i32, 16>;

/// Signed 2.30 fixed point (uses `i32` storage).
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
