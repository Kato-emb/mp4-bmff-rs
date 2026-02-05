//! Affine transformation matrix for video tracks.
//!
//! BMFF uses a 3×3 transformation matrix in track and movie headers to
//! specify how video should be displayed. The matrix supports rotation,
//! scaling, translation, and perspective transforms.
//!
//! # Matrix Layout
//!
//! The matrix is stored in row-major order:
//!
//! ```text
//! | a  b  u |
//! | c  d  v |
//! | x  y  w |
//! ```
//!
//! Where:
//! - `a`, `b`, `c`, `d`: 2D affine components (I16F16 fixed-point).
//! - `x`, `y`: Translation components (I16F16 fixed-point).
//! - `u`, `v`, `w`: Projective components (I2F30 fixed-point).
//!
//! # Coordinate Transformation
//!
//! A point (px, py) is transformed to (px', py') by:
//!
//! ```text
//! px' = (a*px + c*py + x) / (u*px + v*py + w)
//! py' = (b*px + d*py + y) / (u*px + v*py + w)
//! ```
//!
//! For affine transforms (no perspective), u=0, v=0, w=1.

use super::fixed::{
    I2F30,
    I16F16, //
};

/// 3×3 transformation matrix as defined in ISO/IEC 14496-12 § 6.5.2.
///
/// Used in `mvhd` (movie header) and `tkhd` (track header) boxes to specify
/// the display transformation for video content.
///
/// # Structure
///
/// - `a`, `b`: First row affine components (I16F16).
/// - `u`: First row projective component (I2F30).
/// - `c`, `d`: Second row affine components (I16F16).
/// - `v`: Second row projective component (I2F30).
/// - `x`, `y`: Translation components (I16F16).
/// - `w`: Homogeneous coordinate scale (I2F30, typically 1.0).
///
/// # Common Transforms
///
/// - **Identity**: `Matrix::identity()` - no transformation.
/// - **90° rotation**: Set a=0, b=1, c=-1, d=0.
/// - **Horizontal flip**: Set a=-1, d=1.
#[rustfmt::skip]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct Matrix {
    /// Horizontal scaling / rotation component.
    pub a: I16F16,
    /// Vertical shearing component.
    pub b: I16F16,
    /// First projective component (typically 0).
    pub u: I2F30,
    /// Horizontal shearing component.
    pub c: I16F16,
    /// Vertical scaling / rotation component.
    pub d: I16F16,
    /// Second projective component (typically 0).
    pub v: I2F30,
    /// Horizontal translation.
    pub x: I16F16,
    /// Vertical translation.
    pub y: I16F16,
    /// Homogeneous scale (typically 1.0).
    pub w: I2F30,
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
