use std::fmt;

use crate::impl_from_and_into_var;

use super::{euclid, Px, PxSize};

pub use euclid::BoolVector2D;

/// Pixel length constrains.
///
/// These constrains can express lower and upper bounds, unbounded upper and preference of *fill* length.
///
/// See also the [`PxConstrains2d`].
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PxConstrains {
    max: Px,
    min: Px,

    /// Fill preference, when this is `true` and the constrains have a maximum bound the fill length is the maximum bounds,
    /// otherwise the fill length is the minimum bounds.
    pub fill: bool,
}
impl PxConstrains {
    /// New unbounded constrain.
    pub fn new_unbounded() -> Self {
        PxConstrains {
            max: Px::MAX,
            min: Px(0),
            fill: false,
        }
    }

    /// New bounded between zero and `max` with no fill.
    pub fn new_bounded(max: Px) -> Self {
        PxConstrains {
            max,
            min: Px(0),
            fill: false,
        }
    }

    /// New bounded to only allow the `length`.
    pub fn new_exact(length: Px) -> Self {
        PxConstrains {
            max: length,
            min: length,
            fill: false,
        }
    }

    /// New bounded to fill the `length`.
    pub fn new_fill(length: Px) -> Self {
        PxConstrains {
            max: length,
            min: Px(0),
            fill: true,
        }
    }

    /// New bounded to a inclusive range.
    ///
    /// # Panics
    ///
    /// Panics if `min` is not <= `max`.
    pub fn new_range(min: Px, max: Px) -> Self {
        assert!(min <= max);

        PxConstrains { max, min, fill: false }
    }

    /// Returns a copy of the current constrains that has `min` as the lower bound and max adjusted to be >= `min`.
    pub fn with_min(mut self, min: Px) -> Self {
        self.min = min;
        self.max = self.max.max(self.min);
        self
    }

    /// Returns a copy of the current constrains that has `max` as the upper bound and min adjusted to be <= `max`.
    pub fn with_max(mut self, max: Px) -> Self {
        self.max = max;
        self.min = self.min.min(self.max);
        self
    }

    /// Returns a copy of the current constrains that sets the `fill` preference.
    pub fn with_fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    /// Returns a copy of the current constrains that sets the fill preference to `self.fill && fill`.
    pub fn with_fill_and(mut self, fill: bool) -> Self {
        self.fill &= fill;
        self
    }

    /// Returns a copy of the current constrains without upper bound.
    pub fn with_unbounded(mut self) -> Self {
        self.max = Px::MAX;
        self
    }

    /// Returns a copy of the current constrains with `sub` subtracted from the maximum bounds.
    ///
    /// Does nothing if is unbounded, otherwise does a saturating subtraction.
    pub fn with_less(mut self, sub: Px) -> Self {
        if self.max < Px::MAX {
            self.max -= sub;
        }
        self
    }

    /// Returns a copy of the current constrains with `add` added to the maximum bounds.
    ///
    /// Does a saturation addition, this can potentially unbound the constrains if [`Px::MAX`] is reached.
    pub fn with_more(mut self, add: Px) -> Self {
        self.max += add;
        self
    }

    /// Gets if the constrains have no upper bound.
    pub fn is_unbounded(self) -> bool {
        self.max == Px::MAX
    }

    /// Gets if the constrains only allow one length.
    pub fn is_exact(self) -> bool {
        self.max == self.min
    }

    /// Gets if the context prefers the maximum length over the minimum.
    ///
    /// Note that if the constrains are unbounded there is not maximum length, in this case the fill length is the minimum.
    pub fn is_fill(self) -> bool {
        self.fill
    }

    /// Gets the fixed length if the constrains only allow one length.
    pub fn exact(self) -> Option<Px> {
        if self.is_exact() {
            Some(self.max)
        } else {
            None
        }
    }

    /// Gets the maximum allowed length, or `None` if is unbounded.
    ///
    /// The maximum is inclusive.
    pub fn max(self) -> Option<Px> {
        if self.max < Px::MAX {
            Some(self.max)
        } else {
            None
        }
    }

    /// Gets the minimum allowed length.
    //
    /// The minimum is inclusive.
    pub fn min(self) -> Px {
        self.min
    }

    /// Clamp the `px` by min and max.
    pub fn clamp(&self, px: Px) -> Px {
        self.min.max(px).min(self.max)
    }

    /// Gets the fill length, if fill is `true` this is the maximum length, otherwise it is the minimum length.
    pub fn fill(self) -> Px {
        if self.fill && !self.is_unbounded() {
            self.max
        } else {
            self.min
        }
    }

    /// Gets the maximum if fill is preferred and max is bounded, or `desired_length` clamped by the constrains.
    pub fn fill_or(&self, desired_length: Px) -> Px {
        if self.fill && !self.is_unbounded() {
            self.max
        } else {
            self.clamp(desired_length)
        }
    }
}
impl_from_and_into_var! {
    /// New fixed.
    fn from(length: Px) -> PxConstrains {
        PxConstrains::new_exact(length)
    }
}
impl fmt::Debug for PxConstrains {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PxConstrains")
            .field("max", &self.max())
            .field("min", &self.min)
            .field("fill", &self.fill)
            .finish()
    }
}
impl Default for PxConstrains {
    fn default() -> Self {
        Self::new_unbounded()
    }
}

/// Pixel *size* constrains.
///
/// These constrains can express lower and upper bounds, unbounded upper and preference of *fill* length for
/// both the ***x*** and ***y*** axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PxConstrains2d {
    /// Constrains of lengths in the *x* or *width* dimension.
    pub x: PxConstrains,
    /// Constrains of lengths in the *y* or *height* dimension.
    pub y: PxConstrains,
}
impl PxConstrains2d {
    /// New unbounded constrain.
    pub fn new_unbounded() -> Self {
        Self {
            x: PxConstrains::new_unbounded(),
            y: PxConstrains::new_unbounded(),
        }
    }

    /// New bounded between zero and `max_y`, `max_y` with no fill.
    pub fn new_bounded(max_x: Px, max_y: Px) -> Self {
        Self {
            x: PxConstrains::new_bounded(max_x),
            y: PxConstrains::new_bounded(max_y),
        }
    }

    /// New bounded between zero and `max` with no fill.
    pub fn new_bounded_size(max: PxSize) -> Self {
        Self::new_bounded(max.width, max.height)
    }

    /// New bounded to only allow the *size*.
    ///
    /// The type [`PxSize`] can also be converted into fixed constrains.
    pub fn new_exact(x: Px, y: Px) -> Self {
        Self {
            x: PxConstrains::new_exact(x),
            y: PxConstrains::new_exact(y),
        }
    }

    /// New bounded to only allow the `size`.
    pub fn new_exact_size(size: PxSize) -> Self {
        Self::new_exact(size.width, size.height)
    }

    /// New bounded to fill the maximum `x` and `y`.
    pub fn new_fill(x: Px, y: Px) -> Self {
        Self {
            x: PxConstrains::new_fill(x),
            y: PxConstrains::new_fill(y),
        }
    }

    /// New bounded to fill the maximum `size`.
    pub fn new_fill_size(size: PxSize) -> Self {
        Self::new_fill(size.width, size.height)
    }

    /// New bounded to a inclusive range.
    ///
    /// A tuple of two [`PxSize`] values can also be converted to these constrains.
    pub fn new_range(min_x: Px, max_x: Px, min_y: Px, max_y: Px) -> Self {
        Self {
            x: PxConstrains::new_range(min_x, max_x),
            y: PxConstrains::new_range(min_y, max_y),
        }
    }

    /// Returns a copy of the current constrains that has `min_x` and `min_y` as the lower bound and max adjusted to be >= min in both axis.
    pub fn with_min(mut self, min_x: Px, min_y: Px) -> Self {
        self.x = self.x.with_min(min_x);
        self.y = self.y.with_min(min_y);
        self
    }

    /// Returns a copy of the current constrains that has `min` as the lower bound and max adjusted to be >= min in both axis.
    pub fn with_min_size(self, min: PxSize) -> Self {
        self.with_min(min.width, min.height)
    }

    /// Returns a copy of the current constrains that has `min_x` as the lower bound and max adjusted to be >= min in the **x** axis.
    pub fn with_min_x(mut self, min_x: Px) -> Self {
        self.x = self.x.with_min(min_x);
        self
    }

    /// Returns a copy of the current constrains that has `min_y` as the lower bound and max adjusted to be >= min in the **y** axis.
    pub fn with_min_y(mut self, min_y: Px) -> Self {
        self.y = self.y.with_min(min_y);
        self
    }

    /// Returns a copy of the current constrains that has `max_x` and `max_y` as the upper bound and min adjusted to be <= max in both axis.
    pub fn with_max(mut self, max_x: Px, max_y: Px) -> Self {
        self.x = self.x.with_max(max_x);
        self.y = self.y.with_max(max_y);
        self
    }

    /// Returns a copy of the current constrains that has `max` as the upper bound and min adjusted to be <= max in both axis.
    pub fn with_max_size(self, max: PxSize) -> Self {
        self.with_max(max.width, max.height)
    }

    /// Returns a copy of the current constrains that has `min_x` as the lower bound and max adjusted to be << max in the **x** axis.
    pub fn with_max_x(mut self, max_x: Px) -> Self {
        self.x = self.x.with_max(max_x);
        self
    }

    /// Returns a copy of the current constrains that has `max_y` as the lower bound and min adjusted to be <= max in the **y** axis.
    pub fn with_max_y(mut self, max_y: Px) -> Self {
        self.y = self.y.with_max(max_y);
        self
    }

    /// Returns a copy of the current constrains that sets the `fill_x` and `fill_y` preference.
    pub fn with_fill(mut self, fill_x: bool, fill_y: bool) -> Self {
        self.x = self.x.with_fill(fill_x);
        self.y = self.y.with_fill(fill_y);
        self
    }

    /// Returns a copy of the current constrains that sets the fill preference to *current && fill*.
    pub fn with_fill_and(mut self, fill_x: bool, fill_y: bool) -> Self {
        self.x = self.x.with_fill_and(fill_x);
        self.y = self.y.with_fill_and(fill_y);
        self
    }

    /// Returns a copy of the current constrains that sets the `fill` preference
    pub fn with_fill_vector(self, fill: BoolVector2D) -> Self {
        self.with_fill(fill.x, fill.y)
    }

    /// Returns a copy of the current constrains that sets the `fill_x` preference.
    pub fn with_fill_x(mut self, fill_x: bool) -> Self {
        self.x = self.x.with_fill(fill_x);
        self
    }

    /// Returns a copy of the current constrains that sets the `fill_y` preference.
    pub fn with_fill_y(mut self, fill_y: bool) -> Self {
        self.y = self.y.with_fill(fill_y);
        self
    }

    /// Returns a copy of the current constrains without upper bound in both axis.
    pub fn with_unbounded(mut self) -> Self {
        self.x = self.x.with_unbounded();
        self.y = self.y.with_unbounded();
        self
    }

    /// Returns a copy of the current constrains without a upper bound in the **x** axis.
    pub fn with_unbounded_x(mut self) -> Self {
        self.x = self.x.with_unbounded();
        self
    }

    /// Returns a copy of the current constrains without a upper bound in the **x** axis.
    pub fn with_unbounded_y(mut self) -> Self {
        self.x = self.x.with_unbounded();
        self
    }

    /// Returns a copy of the current constrains with `sub_x` and `sob_y` subtracted from the maximum bounds.
    ///
    /// Does nothing if is unbounded, otherwise does a saturating subtraction.
    pub fn with_less(mut self, sub_x: Px, sub_y: Px) -> Self {
        self.x = self.x.with_less(sub_x);
        self.y = self.y.with_less(sub_y);
        self
    }

    /// Returns a copy of the current constrains with `sub` subtracted from the maximum bounds.
    ///
    /// Does nothing if is unbounded, otherwise does a saturating subtraction.
    pub fn with_less_size(self, sub: PxSize) -> Self {
        self.with_less(sub.width, sub.height)
    }

    /// Returns a copy of the current constrains with `sub_x` subtracted from the maximum bounds of the **x** axis.
    ///
    /// Does nothing if is unbounded, otherwise does a saturating subtraction.
    pub fn with_less_x(mut self, sub_x: Px) -> Self {
        self.x = self.x.with_less(sub_x);
        self
    }

    /// Returns a copy of the current constrains with `sub_y` subtracted from the maximum bounds of the **y** axis.
    ///
    /// Does nothing if is unbounded, otherwise does a saturating subtraction.
    pub fn with_less_y(mut self, sub_y: Px) -> Self {
        self.y = self.y.with_less(sub_y);
        self
    }

    /// Returns a copy of the current constrains with `add_x` and `add_y` added to the maximum bounds.
    ///
    /// Does a saturation addition, this can potentially unbound the constrains if [`Px::MAX`] is reached.
    pub fn with_more(mut self, add_x: Px, add_y: Px) -> Self {
        self.x = self.x.with_more(add_x);
        self.y = self.y.with_more(add_y);
        self
    }

    /// Returns a copy of the current constrains with `add` added to the maximum bounds.
    ///
    /// Does a saturation addition, this can potentially unbound the constrains if [`Px::MAX`] is reached.
    pub fn with_more_size(self, add: PxSize) -> Self {
        self.with_more(add.width, add.height)
    }

    /// Returns a copy of the current constrains with [`x`] modified by the closure.
    ///
    /// [`x`]: Self::x
    pub fn with_x(mut self, x: impl FnOnce(PxConstrains) -> PxConstrains) -> Self {
        self.x = x(self.x);
        self
    }

    /// Returns a copy of the current constrains with [`y`] modified by the closure.
    ///
    /// [`y`]: Self::y
    pub fn with_y(mut self, y: impl FnOnce(PxConstrains) -> PxConstrains) -> Self {
        self.y = y(self.y);
        self
    }

    /// Gets if the constrains have no upper bound.
    pub fn is_unbounded(self) -> BoolVector2D {
        BoolVector2D {
            x: self.x.is_unbounded(),
            y: self.y.is_unbounded(),
        }
    }

    /// Gets if the constrains only allow one length.
    pub fn is_exact(self) -> BoolVector2D {
        BoolVector2D {
            x: self.x.is_exact(),
            y: self.y.is_exact(),
        }
    }

    /// Gets if the context prefers the maximum length over the minimum.
    ///
    /// Note that if the constrains are unbounded there is not maximum length, in this case the fill length is the minimum.
    pub fn is_fill(self) -> BoolVector2D {
        BoolVector2D {
            x: self.x.is_fill(),
            y: self.y.is_fill(),
        }
    }

    /// Gets the fixed size if the constrains only allow one length in both axis.
    pub fn fixed_size(self) -> Option<PxSize> {
        Some(PxSize::new(self.x.exact()?, self.y.exact()?))
    }

    /// Gets the maximum allowed size, or `None` if is unbounded in any of the axis.
    ///
    /// The maximum is inclusive.
    pub fn max_size(self) -> Option<PxSize> {
        Some(PxSize::new(self.x.max()?, self.y.max()?))
    }

    /// Gets the minimum allowed size.
    //
    /// The minimum is inclusive.
    pub fn min_size(self) -> PxSize {
        PxSize::new(self.x.min(), self.y.min())
    }

    /// Clamp the `size` by min and max.
    pub fn clamp_size(self, size: PxSize) -> PxSize {
        PxSize::new(self.x.clamp(size.width), self.y.clamp(size.height))
    }

    /// Gets the fill size, if fill is `true` this is the maximum length, otherwise it is the minimum length.
    pub fn fill_size(self) -> PxSize {
        PxSize::new(self.x.fill(), self.y.fill())
    }

    /// Gets the maximum if fill is preferred and max is bounded, or `desired_length` clamped by the constrains.
    pub fn fill_size_or(&self, desired_size: PxSize) -> PxSize {
        PxSize::new(self.x.fill_or(desired_size.width), self.y.fill_or(desired_size.height))
    }
}
impl_from_and_into_var! {
    /// New fixed.
    fn from(size: PxSize) -> PxConstrains2d {
        PxConstrains2d::new_exact(size.width, size.height)
    }

    /// New range, the minimum and maximum is computed.
    fn from((a, b): (PxSize, PxSize)) -> PxConstrains2d {
        PxConstrains2d {
            x: if a.width > b.width {
                PxConstrains::new_range(b.width, a.width)
            } else {
                PxConstrains::new_range(a.width, b.width)
            },
            y: if a.height > b.height {
                PxConstrains::new_range(b.height, a.height)
            } else {
                PxConstrains::new_range(a.height, b.height)
            }
        }
    }
}
impl Default for PxConstrains2d {
    fn default() -> Self {
        Self::new_unbounded()
    }
}
