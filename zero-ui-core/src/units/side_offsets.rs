use std::fmt;

use crate::{context::LayoutMetrics, impl_from_and_into_var};

use super::{impl_length_comp_conversions, AvailableSize, Factor, FactorPercent, LayoutMask, Length, PxSideOffsets};

/// 2D size offsets in [`Length`] units.
///
/// This unit defines spacing around all four sides of a box, a widget margin can be defined using a value of this type.
#[derive(Clone, Default, PartialEq)]
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
    pub fn new_dimension<TB: Into<Length>, LR: Into<Length>>(top_bottom: TB, left_right: LR) -> Self {
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

    /// Compute the offsets in a layout context.

    pub fn to_layout(&self, ctx: &LayoutMetrics, available_size: AvailableSize, default_value: PxSideOffsets) -> PxSideOffsets {
        let width = available_size.width;
        let height = available_size.height;
        PxSideOffsets::new(
            self.top.to_layout(ctx, height, default_value.top),
            self.right.to_layout(ctx, width, default_value.right),
            self.bottom.to_layout(ctx, height, default_value.bottom),
            self.left.to_layout(ctx, width, default_value.left),
        )
    }

    /// Compute a [`LayoutMask`] that flags all contextual values that affect the result of [`to_layout`].
    ///
    /// [`to_layout`]: Self::to_layout

    pub fn affect_mask(&self) -> LayoutMask {
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
}

impl_length_comp_conversions! {
    /// (top-bottom, left-right)
    fn from(top_bottom: TB, left_right: LR) -> SideOffsets {
        SideOffsets::new_dimension(top_bottom,left_right)
    }

    /// (top, right, bottom, left)
    fn from(top: T, right: R, bottom: B, left: L) -> SideOffsets {
        SideOffsets::new(top, right, bottom, left)
    }
}
