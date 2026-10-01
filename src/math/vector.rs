//! Algebra lineal del renderer, implementada con componentes `f32`.
//! Las matrices guardan columnas y las rotaciones siguen la regla de la mano derecha.

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self::new(0.0, 0.0);

    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    #[inline]
    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    #[inline]
    pub fn floor(self) -> Self {
        Self::new(self.x.floor(), self.y.floor())
    }

    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        self * (1.0 - t) + other * t
    }
}

/// Tres componentes contiguos; sin padding en las muestras del volumen.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self::splat(0.0);
    pub const ONE: Self = Self::splat(1.0);
    pub const X: Self = Self::new(1.0, 0.0, 0.0);
    pub const Y: Self = Self::new(0.0, 1.0, 0.0);
    pub const Z: Self = Self::new(0.0, 0.0, 1.0);
    #[cfg(test)]
    pub const NEG_Y: Self = Self::new(0.0, -1.0, 0.0);

    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub const fn splat(value: f32) -> Self {
        Self::new(value, value, value)
    }

    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    #[inline]
    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Requiere un vector no nulo de longitud finita.
    #[inline]
    pub fn normalize(self) -> Self {
        self * self.length().recip()
    }

    /// Un vector nulo o no normalizable no introduce NaNs en un rayo.
    #[inline]
    pub fn normalize_or_zero(self) -> Self {
        let inverse = self.length().recip();
        if inverse.is_finite() && inverse > 0.0 {
            self * inverse
        } else {
            Self::ZERO
        }
    }

    #[inline]
    pub fn floor(self) -> Self {
        Self::new(self.x.floor(), self.y.floor(), self.z.floor())
    }

    #[inline]
    pub fn abs(self) -> Self {
        Self::new(self.x.abs(), self.y.abs(), self.z.abs())
    }

    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self::new(
            self.x.max(other.x),
            self.y.max(other.y),
            self.z.max(other.z),
        )
    }

    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self::new(
            self.x.min(other.x),
            self.y.min(other.y),
            self.z.min(other.z),
        )
    }

    #[inline]
    pub fn max_element(self) -> f32 {
        self.x.max(self.y).max(self.z)
    }

    #[inline]
    pub fn min_element(self) -> f32 {
        self.x.min(self.y).min(self.z)
    }

    /// Luminancia Rec. 709 de un color lineal.
    #[inline]
    pub fn luminance(self) -> f32 {
        self.dot(Self::new(0.2126, 0.7152, 0.0722))
    }

    /// Exponencial por componente, para atenuacion de Beer-Lambert.
    #[inline]
    pub fn exp(self) -> Self {
        Self::new(self.x.exp(), self.y.exp(), self.z.exp())
    }

    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        self * (1.0 - t) + other * t
    }
}

// Los operadores son por componente; Vec * Vec modula el color RGB.
// Compartir estas definiciones evita duplicar la misma aritmetica en 2D y 3D.
macro_rules! vector_operators {
    ($vector:ident, $($field:ident),+) => {
        impl Add for $vector {
            type Output = Self;
            #[inline]
            fn add(self, rhs: Self) -> Self {
                Self { $($field: self.$field + rhs.$field),+ }
            }
        }
        impl Sub for $vector {
            type Output = Self;
            #[inline]
            fn sub(self, rhs: Self) -> Self {
                Self { $($field: self.$field - rhs.$field),+ }
            }
        }
        impl Neg for $vector {
            type Output = Self;
            #[inline]
            fn neg(self) -> Self {
                Self { $($field: -self.$field),+ }
            }
        }
        impl Mul for $vector {
            type Output = Self;
            #[inline]
            fn mul(self, rhs: Self) -> Self {
                Self { $($field: self.$field * rhs.$field),+ }
            }
        }
        impl Mul<f32> for $vector {
            type Output = Self;
            #[inline]
            fn mul(self, rhs: f32) -> Self {
                Self { $($field: self.$field * rhs),+ }
            }
        }
        impl Mul<$vector> for f32 {
            type Output = $vector;
            #[inline]
            fn mul(self, rhs: $vector) -> $vector {
                rhs * self
            }
        }
        impl Sub<$vector> for f32 {
            type Output = $vector;
            #[inline]
            fn sub(self, rhs: $vector) -> $vector {
                $vector { $($field: self - rhs.$field),+ }
            }
        }
        impl Div<f32> for $vector {
            type Output = Self;
            #[inline]
            fn div(self, rhs: f32) -> Self {
                Self { $($field: self.$field / rhs),+ }
            }
        }
        impl AddAssign for $vector {
            #[inline]
            fn add_assign(&mut self, rhs: Self) { *self = *self + rhs; }
        }
        impl SubAssign for $vector {
            #[inline]
            fn sub_assign(&mut self, rhs: Self) { *self = *self - rhs; }
        }
        impl MulAssign<f32> for $vector {
            #[inline]
            fn mul_assign(&mut self, rhs: f32) { *self = *self * rhs; }
        }
        impl DivAssign<f32> for $vector {
            #[inline]
            fn div_assign(&mut self, rhs: f32) { *self = *self / rhs; }
        }
    };
}

vector_operators!(Vec2, x, y);
vector_operators!(Vec3, x, y, z);

#[derive(Clone, Copy, Debug)]
pub struct Mat3 {
    columns: [Vec3; 3],
}

impl Mat3 {
    /// Base local -> mundo: cada columna es un eje local escrito en mundo.
    pub const fn from_columns(x: Vec3, y: Vec3, z: Vec3) -> Self {
        Self { columns: [x, y, z] }
    }

    /// Para una rotacion pura la transpuesta es la inversa.
    pub fn transpose(self) -> Self {
        let [a, b, c] = self.columns;
        Self {
            columns: [
                Vec3::new(a.x, b.x, c.x),
                Vec3::new(a.y, b.y, c.y),
                Vec3::new(a.z, b.z, c.z),
            ],
        }
    }

    pub fn from_rotation_y(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self {
            columns: [Vec3::new(c, 0.0, -s), Vec3::Y, Vec3::new(s, 0.0, c)],
        }
    }

    pub fn from_rotation_z(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self {
            columns: [Vec3::new(c, s, 0.0), Vec3::new(-s, c, 0.0), Vec3::Z],
        }
    }
}

impl Mul<Vec3> for Mat3 {
    type Output = Vec3;
    #[inline]
    fn mul(self, rhs: Vec3) -> Vec3 {
        self.columns[0] * rhs.x + self.columns[1] * rhs.y + self.columns[2] * rhs.z
    }
}

impl Mul for Mat3 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self {
            columns: rhs.columns.map(|column| self * column),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_product_preserves_orientation_and_orthogonality() {
        assert_eq!(Vec3::X.cross(Vec3::Y), Vec3::Z);
        let a = Vec3::new(2.0, -3.0, 5.0);
        let b = Vec3::new(-7.0, 1.0, 4.0);
        let cross = a.cross(b);
        assert_eq!(cross, -b.cross(a));
        assert_eq!(cross.dot(a), 0.0);
        assert_eq!(cross.dot(b), 0.0);
        assert!((a.normalize().length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn normalization_handles_degenerate_directions() {
        for value in [
            Vec3::ZERO,
            Vec3::splat(f32::INFINITY),
            Vec3::splat(f32::NAN),
        ] {
            assert_eq!(value.normalize_or_zero(), Vec3::ZERO);
        }
        let a = Vec3::new(3.0, 4.0, 0.0);
        assert!((a.normalize_or_zero() - Vec3::new(0.6, 0.8, 0.0)).length() < 1e-6);
    }

    #[test]
    fn sky_rotations_preserve_handedness_lengths_and_composition() {
        let quarter_turn = std::f32::consts::FRAC_PI_2;
        assert!((Mat3::from_rotation_y(quarter_turn) * Vec3::Z - Vec3::X).length() < 1e-6);
        assert!((Mat3::from_rotation_z(quarter_turn) * Vec3::X - Vec3::Y).length() < 1e-6);
        let a = Mat3::from_rotation_y(0.7);
        let b = Mat3::from_rotation_z(-0.23);
        let v = Vec3::new(2.0, -3.0, 5.0);
        assert!(((a * b) * v - a * (b * v)).length() < 1e-6);
        assert!((((a * b) * v).length() - v.length()).abs() < 1e-6);
        assert!((Mat3::from_rotation_y(-0.7) * (a * v) - v).length() < 1e-6);
        let rotation = Mat3::from_rotation_z(0.4) * Mat3::from_rotation_y(-1.1);
        assert!((rotation.transpose() * (rotation * v) - v).length() < 1e-5);
    }
}
