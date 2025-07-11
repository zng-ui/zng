use std::{fmt, ops};

use zng_unit::DipSideOffsets;
use zng_var::{animation::Transitionable, impl_from_and_into_var};

use super::{Factor, FactorPercent, FactorSideOffsets, Layout1d, LayoutMask, Length, PxSideOffsets, impl_length_comp_conversions};

/// 2D size offsets in [`Length`] units.
///
/// This unit defines spacing around all four sides of a box, a widget margin can be defined using a value of this type.
#[derive(Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Transitionable)]
pub struct SideOffsets {
    /// Spacing above, in length units.
    pub top: Length,
    /// Spacing to the right, in length units.
    pub right: Length,
    /// Spacing bellow, in length units.
    pub bottom: Length,
    /// Spacing to the left ,in length units.
    pub left: Length,
}
impl fmt::Debug for SideOffsets {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("SideOffsets")
                .field("top", &self.top)
                .field("right", &self.right)
                .field("bottom", &self.bottom)
                .field("left", &self.left)
                .finish()
        } else if self.all_eq() {
            write!(f, "{:?}", self.top)
        } else if self.dimensions_eq() {
            write!(f, "({:?}, {:?})", self.top, self.left)
        } else {
            write!(f, "({:?}, {:?}, {:?}, {:?})", self.top, self.right, self.bottom, self.left)
        }
    }
}
impl SideOffsets {
    /// New top, right, bottom left offsets. From any [`Length`] type.
    pub fn new<T: Into<Length>, R: Into<Length>, B: Into<Length>, L: Into<Length>>(top: T, right: R, bottom: B, left: L) -> Self {
        SideOffsets {
            top: top.into(),
            right: right.into(),
            bottom: bottom.into(),
            left: left.into(),
        }
    }

    /// Top-bottom and left-right equal. From any [`Length`] type.
    pub fn new_vh<TB: Into<Length>, LR: Into<Length>>(top_bottom: TB, left_right: LR) -> Self {
        let top_bottom = top_bottom.into();
        let left_right = left_right.into();
        SideOffsets {
            top: top_bottom.clone(),
            bottom: top_bottom,
            left: left_right.clone(),
            right: left_right,
        }
    }

    /// All sides equal. From any [`Length`] type.
    pub fn new_all<T: Into<Length>>(all_sides: T) -> Self {
        let all_sides = all_sides.into();
        SideOffsets {
            top: all_sides.clone(),
            right: all_sides.clone(),
            bottom: all_sides.clone(),
            left: all_sides,
        }
    }

    /// All sides [zero](Length::zero).
    pub fn zero() -> Self {
        Self::new_all(Length::zero())
    }

    /// If all sides are equal.
    pub fn all_eq(&self) -> bool {
        self.top == self.bottom && self.top == self.left && self.top == self.right
    }

    /// If top and bottom are equal; and left and right are equal.
    pub fn dimensions_eq(&self) -> bool {
        self.top == self.bottom && self.left == self.right
    }
}
impl super::Layout2d for SideOffsets {
    type Px = PxSideOffsets;

    fn layout_dft(&self, default: Self::Px) -> Self::Px {
        PxSideOffsets::new(
            self.top.layout_dft_y(default.top),
            self.right.layout_dft_x(default.right),
            self.bottom.layout_dft_y(default.bottom),
            self.left.layout_dft_x(default.left),
        )
    }

    fn affect_mask(&self) -> LayoutMask {
        self.top.affect_mask() | self.right.affect_mask() | self.bottom.affect_mask() | self.left.affect_mask()
    }
}
impl_from_and_into_var! {
    /// All sides equal.
    fn from(all: Length) -> SideOffsets {
        SideOffsets::new_all(all)
    }

    /// All sides equal relative length.
    fn from(percent: FactorPercent) -> SideOffsets {
        SideOffsets::new_all(percent)
    }
    /// All sides equal relative length.
    fn from(norm: Factor) -> SideOffsets {
        SideOffsets::new_all(norm)
    }

    /// All sides equal exact length.
    fn from(f: f32) -> SideOffsets {
        SideOffsets::new_all(f)
    }
    /// All sides equal exact length.
    fn from(i: i32) -> SideOffsets {
        SideOffsets::new_all(i)
    }

    /// From exact lengths.
    fn from(offsets: PxSideOffsets) -> SideOffsets {
        SideOffsets::new(offsets.top, offsets.right, offsets.bottom, offsets.left)
    }

    // From exact lengths.
    fn from(offsets: DipSideOffsets) -> SideOffsets {
        SideOffsets::new(offsets.top, offsets.right, offsets.bottom, offsets.left)
    }
}

impl_length_comp_conversions! {
    /// (top-bottom, left-right)
    fn from(top_bottom: TB, left_right: LR) -> SideOffsets {
        SideOffsets::new_vh(top_bottom, left_right)
    }

    /// (top, right, bottom, left)
    fn from(top: T, right: R, bottom: B, left: L) -> SideOffsets {
        SideOffsets::new(top, right, bottom, left)
    }
}

impl ops::Add for SideOffsets {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self {
        self += rhs;
        self
    }
}
impl ops::AddAssign for SideOffsets {
    fn add_assign(&mut self, rhs: Self) {
        self.top += rhs.top;
        self.right += rhs.right;
        self.bottom += rhs.bottom;
        self.left += rhs.left;
    }
}
impl ops::Sub for SideOffsets {
    type Output = Self;

    fn sub(mut self, rhs: Self) -> Self {
        self -= rhs;
        self
    }
}
impl ops::SubAssign for SideOffsets {
    fn sub_assign(&mut self, rhs: Self) {
        self.top -= rhs.top;
        self.right -= rhs.right;
        self.bottom -= rhs.bottom;
        self.left -= rhs.left;
    }
}
impl<S: Into<FactorSideOffsets>> ops::Mul<S> for SideOffsets {
    type Output = Self;

    fn mul(mut self, rhs: S) -> Self {
        self *= rhs;
        self
    }
}
impl<S: Into<FactorSideOffsets>> ops::Mul<S> for &SideOffsets {
    type Output = SideOffsets;

    fn mul(self, rhs: S) -> Self::Output {
        self.clone() * rhs
    }
}
impl<S: Into<FactorSideOffsets>> ops::MulAssign<S> for SideOffsets {
    fn mul_assign(&mut self, rhs: S) {
        let rhs = rhs.into();
        self.top *= rhs.top;
        self.right *= rhs.right;
        self.bottom *= rhs.bottom;
        self.left *= rhs.left;
    }
}
impl<S: Into<FactorSideOffsets>> ops::Div<S> for SideOffsets {
    type Output = Self;

    fn div(mut self, rhs: S) -> Self {
        self /= rhs;
        self
    }
}
impl<S: Into<FactorSideOffsets>> ops::Div<S> for &SideOffsets {
    type Output = SideOffsets;

    fn div(self, rhs: S) -> Self::Output {
        self.clone() / rhs
    }
}
impl<S: Into<FactorSideOffsets>> ops::DivAssign<S> for SideOffsets {
    fn div_assign(&mut self, rhs: S) {
        let rhs = rhs.into();
        self.top /= rhs.top;
        self.right /= rhs.right;
        self.bottom /= rhs.bottom;
        self.left /= rhs.left;
    }
}
