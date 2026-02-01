//! Affine transform matrix used by BMFF visual track boxes.
//!
//! The matrix is stored row-major using 16.16 fixed point for the 2D affine
//! components and 2.30 fixed point for the projective terms.

use super::fixed::{
    I2F30,
    I16F16, //
};

/// 3×3 transformation matrix as described in ISO/IEC 14496-12 § 6.5.2.
#[rustfmt::skip]
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct Matrix {
    pub a: I16F16, pub b: I16F16, pub u: I2F30,
    pub c: I16F16, pub d: I16F16, pub v: I2F30,
    pub x: I16F16, pub y: I16F16, pub w: I2F30,
}

impl Matrix {
    /// Identity transform.
    #[inline]
    pub const fn identity() -> Self {
        Self {
            a: I16F16::from_raw(0x0001_0000),
            b: I16F16::from_raw(0),
            u: I2F30::from_raw(0),
            c: I16F16::from_raw(0),
            d: I16F16::from_raw(0x0001_0000),
            v: I2F30::from_raw(0),
            x: I16F16::from_raw(0),
            y: I16F16::from_raw(0),
            w: I2F30::from_raw(0x4000_0000),
        }
    }

    /// Build from the raw fixed-point components.
    #[inline]
    pub fn from_raw(raw: [i32; 9]) -> Self {
        Self {
            a: I16F16::from_raw(raw[0]),
            b: I16F16::from_raw(raw[1]),
            u: I2F30::from_raw(raw[2]),
            c: I16F16::from_raw(raw[3]),
            d: I16F16::from_raw(raw[4]),
            v: I2F30::from_raw(raw[5]),
            x: I16F16::from_raw(raw[6]),
            y: I16F16::from_raw(raw[7]),
            w: I2F30::from_raw(raw[8]),
        }
    }

    /// Raw fixed-point storage values in row-major order.
    #[rustfmt::skip]
    #[inline]
    pub fn to_raw(&self) -> [i32; 9] {
        [
            self.a.to_raw(), self.b.to_raw(), self.u.to_raw(),
            self.c.to_raw(), self.d.to_raw(), self.v.to_raw(),
            self.x.to_raw(), self.y.to_raw(), self.w.to_raw(),
        ]
    }
}

impl Default for Matrix {
    fn default() -> Self {
        Self::identity()
    }
}

impl From<[i32; 9]> for Matrix {
    fn from(raw: [i32; 9]) -> Self {
        Self::from_raw(raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_defaults() {
        let m = Matrix::default();
        assert_eq!(m.a.to_raw(), I16F16::from_f32(1.0).to_raw());
        assert_eq!(m.d.to_raw(), I16F16::from_f32(1.0).to_raw());
        assert_eq!(m.w.to_raw(), I2F30::from_f32(1.0).to_raw());
    }

    #[test]
    fn raw_conversion_round_trip() {
        let raw = [
            0x0001_0000,
            0x0000_8000,
            0x1234_5678,
            -0x0000_4000,
            0x0001_0000,
            0x9abc_def0u32 as i32,
            0x0000_2000,
            -0x0000_1000,
            0x4000_0000,
        ];
        let matrix = Matrix::from_raw(raw);
        assert_eq!(matrix.to_raw(), raw);
    }
}
