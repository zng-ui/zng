use crate::Superellipse;

use serde::{Deserialize, Serialize};

/// Superellipses that define the shape (curvature) of the four corners.
///
/// See also [`CornerRadius2D`] for the radii definitions.
///
/// [`CornerRadius2D`]: crate::CornerRadius2D
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CornerShape {
    /// Top-left corner curvature.
    pub top_left: Superellipse,
    /// Top-right corner curvature.
    pub top_right: Superellipse,
    /// Bottom-right corner curvature.
    pub bottom_right: Superellipse,
    /// Bottom-left corner curvature.
    pub bottom_left: Superellipse,
}
impl CornerShape {
    /// New every corner unique.
    pub fn new<TL: Into<Superellipse>, TR: Into<Superellipse>, BR: Into<Superellipse>, BL: Into<Superellipse>>(
        top_left: TL,
        top_right: TR,
        bottom_right: BR,
        bottom_left: BL,
    ) -> Self {
        CornerShape {
            top_left: top_left.into(),
            top_right: top_right.into(),
            bottom_right: bottom_right.into(),
            bottom_left: bottom_left.into(),
        }
    }

    /// New all corners the same.
    pub fn new_all<E: Into<Superellipse>>(k: E) -> Self {
        let k = k.into();
        CornerShape {
            top_left: k,
            top_right: k,
            bottom_left: k,
            bottom_right: k,
        }
    }

    /// If all corners are the same value.
    pub fn all_corners_eq(&self) -> bool {
        self.top_left == self.top_right && self.top_left == self.bottom_right && self.top_left == self.bottom_left
    }
}
impl CornerShape {
    const fn new_all_const(k: Superellipse) -> Self {
        Self {
            top_left: k,
            top_right: k,
            bottom_right: k,
            bottom_left: k,
        }
    }

    /// Defines a straight, diagonal corner, which is neither convex nor concave.
    pub const BEVEL: Self = Self::new_all_const(Superellipse::BEVEL);
    /// Defines a 90-degree concave square corner.
    pub const NOTCH: Self = Self::new_all_const(Superellipse::NOTCH);
    /// Defines a convex ordinary ellipse, which is the standard rounded corner created by the radius.
    ///
    /// This is the default value.
    pub const ROUND: Self = Self::new_all_const(Superellipse::ROUND);
    /// Defines a concave ordinary ellipse.
    pub const SCOOP: Self = Self::new_all_const(Superellipse::SCOOP);
    /// Defines a 90-degree convex square corner, the same as when the corner has no radius.
    pub const SQUARE: Self = Self::new_all_const(Superellipse::SQUARE);
    /// Defines a "squircle", which is a convex curve in between `ROUND` and `SQUARE`.
    pub const SQUIRCLE: Self = Self::new_all_const(Superellipse::SQUIRCLE);
}

/// [`CornerShape::new`]
impl<TL: Into<Superellipse>, TR: Into<Superellipse>, BR: Into<Superellipse>, BL: Into<Superellipse>> From<(TL, TR, BR, BL)>
    for CornerShape
{
    fn from((top_left, top_right, bottom_right, bottom_left): (TL, TR, BR, BL)) -> Self {
        Self::new(top_left, top_right, bottom_right, bottom_left)
    }
}
/// [`CornerShape::new_all`]
impl From<Superellipse> for CornerShape {
    fn from(k: Superellipse) -> Self {
        Self::new_all(k)
    }
}
/// [`CornerShape::new_all`]
impl From<f32> for CornerShape {
    fn from(k: f32) -> Self {
        Self::new_all(k)
    }
}
