use std::{fmt, ops};

use crate::{EQ_GRANULARITY, about_eq, about_eq_hash, about_eq_ord};

/// Defines the curvature of a corner ellipses.
///
/// The value is ***K*** where n=2^K of a superellipse, the same as the CSS `superellipse` function.
#[derive(Copy, Clone, serde::Serialize, serde::Deserialize, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Superellipse(pub f32);
impl Superellipse {
    /// Defines a straight, diagonal corner, which is neither convex nor concave.
    pub const BEVEL: Self = Self(0.0);
    /// Defines a 90-degree concave square corner.
    pub const NOTCH: Self = Self(f32::NEG_INFINITY);
    /// Defines a convex ordinary ellipse, which is the standard rounded corner created by the radius.
    ///
    /// This is the default value.
    pub const ROUND: Self = Self(1.0);
    /// Defines a concave ordinary ellipse.
    pub const SCOOP: Self = Self(-1.0);
    /// Defines a 90-degree convex square corner, the same as when the corner has no radius.
    pub const SQUARE: Self = Self(f32::INFINITY);
    /// Defines a "squircle", which is a convex curve in between `ROUND` and `SQUARE`.
    pub const SQUIRCLE: Self = Self(2.0);
}
impl Default for Superellipse {
    fn default() -> Self {
        Self::ROUND
    }
}
impl fmt::Debug for Superellipse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (named, name) in [
            (Self::BEVEL, "BEVEL"),
            (Self::NOTCH, "NOTCH"),
            (Self::ROUND, "ROUND"),
            (Self::SCOOP, "SCOOP"),
            (Self::SQUARE, "SQUARE"),
            (Self::SQUIRCLE, "SQUIRCLE"),
        ] {
            if named == *self {
                if f.alternate() {
                    write!(f, "Superellipse::")?
                }
                return write!(f, "{name}");
            }
        }
        f.debug_tuple("Superellipse").field(&self.0).finish()
    }
}
impl From<f32> for Superellipse {
    fn from(value: f32) -> Self {
        Superellipse(value)
    }
}
impl ops::Add for Superellipse {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl ops::AddAssign for Superellipse {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}
impl ops::Sub for Superellipse {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}
impl ops::SubAssign for Superellipse {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}
impl std::hash::Hash for Superellipse {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        about_eq_hash(self.0, EQ_GRANULARITY, state)
    }
}
impl PartialEq for Superellipse {
    fn eq(&self, other: &Self) -> bool {
        about_eq(self.0, other.0, EQ_GRANULARITY)
    }
}
impl Eq for Superellipse {}
impl std::cmp::PartialOrd for Superellipse {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl std::cmp::Ord for Superellipse {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        about_eq_ord(self.0, other.0, EQ_GRANULARITY)
    }
}
impl ops::Mul for Superellipse {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Superellipse(self.0 * rhs.0)
    }
}
impl ops::MulAssign for Superellipse {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}
impl ops::Div for Superellipse {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Superellipse(self.0 / rhs.0)
    }
}
impl ops::DivAssign for Superellipse {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl ops::Mul<f32> for Superellipse {
    type Output = Superellipse;

    fn mul(self, rhs: f32) -> Self::Output {
        Superellipse(self.0 * rhs)
    }
}
impl ops::MulAssign<f32> for Superellipse {
    fn mul_assign(&mut self, rhs: f32) {
        self.0 *= rhs;
    }
}
impl ops::Div<f32> for Superellipse {
    type Output = Superellipse;

    fn div(self, rhs: f32) -> Self::Output {
        Superellipse(self.0 / rhs)
    }
}
impl ops::DivAssign<f32> for Superellipse {
    fn div_assign(&mut self, rhs: f32) {
        self.0 /= rhs;
    }
}

/// Parses `"##"` as `f32`, parse names.
impl std::str::FromStr for Superellipse {
    type Err = std::num::ParseFloatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<f32>().map(Self)
    }
}
