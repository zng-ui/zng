use std::{cmp, fmt, ops};

use serde::{Deserialize, Serialize};

use crate::SideOffsets2D;

/// Ellipses that define the radius of the four corners of a 2D box.
#[derive(Serialize, Deserialize)]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: Deserialize<'de>"))]
pub struct CornerRadius2D<T, U> {
    /// Top-left corner radius.
    pub top_left: euclid::Size2D<T, U>,
    /// Top-right corner radius.
    pub top_right: euclid::Size2D<T, U>,
    /// Bottom-right corner radius.
    pub bottom_right: euclid::Size2D<T, U>,
    /// Bottom-left corner radius.
    pub bottom_left: euclid::Size2D<T, U>,
}
impl<T: Default, U> Default for CornerRadius2D<T, U> {
    fn default() -> Self {
        Self {
            top_left: Default::default(),
            top_right: Default::default(),
            bottom_right: Default::default(),
            bottom_left: Default::default(),
        }
    }
}
impl<T: Clone, U> Clone for CornerRadius2D<T, U> {
    fn clone(&self) -> Self {
        Self {
            top_left: self.top_left.clone(),
            top_right: self.top_right.clone(),
            bottom_right: self.bottom_right.clone(),
            bottom_left: self.bottom_left.clone(),
        }
    }
}
impl<T: Copy, U> Copy for CornerRadius2D<T, U> {}
impl<T: Copy + num_traits::Zero, U> CornerRadius2D<T, U> {
    /// New with distinct values.
    pub fn new(
        top_left: euclid::Size2D<T, U>,
        top_right: euclid::Size2D<T, U>,
        bottom_right: euclid::Size2D<T, U>,
        bottom_left: euclid::Size2D<T, U>,
    ) -> Self {
        Self {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }

    /// New all corners same radius.
    pub fn new_all(radius: euclid::Size2D<T, U>) -> Self {
        Self::new(radius, radius, radius, radius)
    }

    /// All zeros.
    pub fn zero() -> Self {
        Self::new_all(euclid::Size2D::zero())
    }

    /// Calculate the corner radius of an outer border around `self` to perfectly fit.
    pub fn inflate(self, offsets: SideOffsets2D<T, U>) -> Self
    where
        T: ops::AddAssign,
    {
        let mut r = self;

        r.top_left.width += offsets.left;
        r.top_left.height += offsets.top;

        r.top_right.width += offsets.right;
        r.top_right.height += offsets.top;

        r.bottom_right.width += offsets.right;
        r.bottom_right.height += offsets.bottom;

        r.bottom_left.width += offsets.left;
        r.bottom_left.height += offsets.bottom;

        r
    }

    /// Calculate the corner radius of an inner border inside `self` to perfectly fit.
    pub fn deflate(self, offsets: SideOffsets2D<T, U>) -> Self
    where
        T: ops::SubAssign + cmp::PartialOrd,
    {
        let mut r = self;

        if r.top_left.width >= offsets.left {
            r.top_left.width -= offsets.left;
        } else {
            r.top_left.width = T::zero();
        }
        if r.top_left.height >= offsets.top {
            r.top_left.height -= offsets.top;
        } else {
            r.top_left.height = T::zero();
        }

        if r.top_right.width >= offsets.right {
            r.top_right.width -= offsets.right;
        } else {
            r.top_right.width = T::zero();
        }
        if r.top_right.height >= offsets.top {
            r.top_right.height -= offsets.top;
        } else {
            r.top_right.height = T::zero();
        }

        if r.bottom_right.width >= offsets.right {
            r.bottom_right.width -= offsets.right;
        } else {
            r.bottom_right.width = T::zero();
        }
        if r.bottom_right.height >= offsets.bottom {
            r.bottom_right.height -= offsets.bottom;
        } else {
            r.bottom_right.height = T::zero();
        }

        if r.bottom_left.width >= offsets.left {
            r.bottom_left.width -= offsets.left;
        } else {
            r.bottom_left.width = T::zero();
        }
        if r.bottom_left.height >= offsets.bottom {
            r.bottom_left.height -= offsets.bottom;
        } else {
            r.bottom_left.height = T::zero();
        }

        r
    }
}
impl<T: fmt::Debug, U> fmt::Debug for CornerRadius2D<T, U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CornerRadius2D")
            .field("top_left", &self.top_left)
            .field("top_right", &self.top_right)
            .field("bottom_right", &self.bottom_right)
            .field("bottom_left", &self.bottom_left)
            .finish()
    }
}
impl<T: PartialEq, U> PartialEq for CornerRadius2D<T, U> {
    fn eq(&self, other: &Self) -> bool {
        self.top_left == other.top_left
            && self.top_right == other.top_right
            && self.bottom_right == other.bottom_right
            && self.bottom_left == other.bottom_left
    }
}
impl<T: Eq, U> Eq for CornerRadius2D<T, U> {}
