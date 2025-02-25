use std::fmt;

use zng_var::{animation::Transitionable, impl_from_and_into_var};

use super::{FactorUnits, Px, PxSize, euclid};

pub use euclid::BoolVector2D;

/// Pixel length constraints.
///
/// These constraints can express lower and upper bounds, unbounded upper and preference of *fill* length.
///
/// See also the [`PxConstraints2d`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Transitionable)]
pub struct PxConstraints {
    #[serde(with = "serde_constraints_max")]
    max: Px,
    min: Px,

    /// Fill preference, when this is `true` and the constraints have a maximum bound the fill length is the maximum bounds,
    /// otherwise the fill length is the minimum bounds.
    pub fill: bool,
}
impl PxConstraints {
    /// New unbounded constrain.
    pub fn new_unbounded() -> Self {
        PxConstraints {
            max: Px::MAX,
            min: Px(0),
            fill: false,
        }
    }

    /// New bounded between zero and `max` with no fill.
    pub fn new_bounded(max: Px) -> Self {
        PxConstraints {
            max,
            min: Px(0),
            fill: false,
        }
    }

    /// New bounded to only allow the `length` and fill.
    pub fn new_exact(length: Px) -> Self {
        PxConstraints {
            max: length,
            min: length,
            fill: true,
        }
    }

    /// New bounded to fill the `length`.
    pub fn new_fill(length: Px) -> Self {
        PxConstraints {
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

        PxConstraints { max, min, fill: false }
    }

    /// Returns a copy of the current constraints that has `min` as the lower bound and max adjusted to be >= `min`.
    pub fn with_new_min(mut self, min: Px) -> Self {
        self.min = min;
        self.max = self.max.max(self.min);
        self
    }

    /// Returns a copy [`with_new_min`] if `min` is greater then the current minimum.
    ///
    /// [`with_new_min`]: Self::with_new_min
    pub fn with_min(self, min: Px) -> Self {
        if min > self.min { self.with_new_min(min) } else { self }
    }

    /// Returns a copy of the current constraints that has `max` as the upper bound and min adjusted to be <= `max`.
    pub fn with_new_max(mut self, max: Px) -> Self {
        self.max = max;
        self.min = self.min.min(self.max);
        self
    }

    /// Returns a copy [`with_new_max`] if `max` is less then the current maximum or the current maximum is unbounded.
    ///
    /// [`with_new_max`]: Self::with_new_max
    pub fn with_max(self, max: Px) -> Self {
        if max < self.max { self.with_new_max(max) } else { self }
    }

    /// Returns a copy of the current constraints that has max and min set to `len` and fill enabled.
    pub fn with_new_exact(mut self, len: Px) -> Self {
        self.max = len;
        self.min = len;
        self.fill = true;
        self
    }

    /// Returns a copy [`with_new_exact`] if the new length clamped by the current constraints.
    ///
    /// [`with_new_exact`]: Self::with_new_exact
    pub fn with_exact(self, len: Px) -> Self {
        self.with_new_exact(self.clamp(len))
    }

    /// Returns a copy of the current constraints that sets the `fill` preference.
    pub fn with_fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    /// Returns a copy of the current constraints that sets the fill preference to `self.fill && fill`.
    pub fn with_fill_and(mut self, fill: bool) -> Self {
        self.fill &= fill;
        self
    }

    /// Returns a copy of the current constraints without upper bound.
    pub fn with_unbounded(mut self) -> Self {
        self.max = Px::MAX;
        self
    }

    /// Returns a copy of the current constraints with `sub` subtracted from the min and max bounds.
    ///
    /// The subtraction is saturating, does not subtract max if unbounded.
    pub fn with_less(mut self, sub: Px) -> Self {
        if self.max < Px::MAX {
            self.max -= sub;
            self.max = self.max.max(Px(0));
        }
        self.min -= sub;
        self.min = self.min.max(Px(0));
        self
    }

    /// Returns a copy of the current constraints with `add` added to the maximum bounds.
    ///
    /// Does a saturation addition, this can potentially unbound the constraints if [`Px::MAX`] is reached.
    pub fn with_more(mut self, add: Px) -> Self {
        self.max += add;
        self
    }

    /// Gets if the constraints have an upper bound.
    pub fn is_bounded(self) -> bool {
        self.max != Px::MAX
    }

    /// Gets if the constraints have no upper bound.
    pub fn is_unbounded(self) -> bool {
        self.max == Px::MAX
    }

    /// Gets if the constraints only allow one length.
    pub fn is_exact(self) -> bool {
        self.max == self.min
    }

    /// Gets if the context prefers the maximum length over the minimum.
    ///
    /// Note that if the constraints are unbounded there is not maximum length, in this case the fill length is the minimum.
    pub fn is_fill_pref(self) -> bool {
        self.fill
    }

    /// Gets if the context prefers the maximum length and there is a maximum length.
    pub fn is_fill_max(self) -> bool {
        self.fill && !self.is_unbounded()
    }

    /// Gets the fixed length if the constraints only allow one length.
    pub fn exact(self) -> Option<Px> {
        if self.is_exact() { Some(self.max) } else { None }
    }

    /// Gets the maximum allowed length, or `None` if is unbounded.
    ///
    /// The maximum is inclusive.
    pub fn max(self) -> Option<Px> {
        if self.max < Px::MAX { Some(self.max) } else { None }
    }

    /// Gets the minimum allowed length.
    //
    /// The minimum is inclusive.
    pub fn min(self) -> Px {
        self.min
    }

    /// Gets the maximum length if it is bounded, or the minimum if not.
    pub fn max_bounded(self) -> Px {
        if self.max < Px::MAX { self.max } else { self.min }
    }

    /// Clamp the `px` by min and max.
    pub fn clamp(self, px: Px) -> Px {
        self.min.max(px).min(self.max)
    }

    /// Gets the fill length, if fill is `true` this is the maximum length, otherwise it is the minimum length.
    pub fn fill(self) -> Px {
        if self.fill && !self.is_unbounded() { self.max } else { self.min }
    }

    /// Gets the maximum if fill is preferred and max is bounded, or `length` clamped by the constraints.
    pub fn fill_or(self, length: Px) -> Px {
        if self.fill && !self.is_unbounded() {
            self.max
        } else {
            self.clamp(length)
        }
    }

    /// Gets the max size if is fill and has max bounds, or gets the exact size if min equals max.
    pub fn fill_or_exact(self) -> Option<Px> {
        if self.is_fill_max() || self.is_exact() {
            Some(self.max)
        } else {
            None
        }
    }

    /// Gets the maximum length if bounded or `length` clamped by the constraints.
    pub fn max_or(self, length: Px) -> Px {
        if self.is_unbounded() { self.clamp(length) } else { self.max }
    }
}
impl_from_and_into_var! {
    /// New exact.
    fn from(length: Px) -> PxConstraints {
        PxConstraints::new_exact(length)
    }
}
impl fmt::Debug for PxConstraints {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("PxConstraints")
                .field("max", &self.max())
                .field("min", &self.min)
                .field("fill", &self.fill)
                .finish()
        } else if self.is_exact() {
            write!(f, "exact({})", self.min)
        } else if self.is_unbounded() {
            write!(f, "min({})", self.min)
        } else if self.fill {
            write!(f, "fill({}, {})", self.min, self.max)
        } else {
            write!(f, "range({}, {})", self.min, self.max)
        }
    }
}
impl Default for PxConstraints {
    fn default() -> Self {
        Self::new_unbounded()
    }
}
mod serde_constraints_max {
    use super::Px;
    use serde::*;
    pub fn serialize<S: Serializer>(max: &Px, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            let px = if *max == Px::MAX { None } else { Some(*max) };
            px.serialize(serializer)
        } else {
            max.serialize(serializer)
        }
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Px, D::Error> {
        if deserializer.is_human_readable() {
            Ok(Option::<Px>::deserialize(deserializer)?.unwrap_or(Px::MAX))
        } else {
            Px::deserialize(deserializer)
        }
    }
}

/// Pixel *size* constraints.
///
/// These constraints can express lower and upper bounds, unbounded upper and preference of *fill* length for
/// both the ***x*** and ***y*** axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Transitionable)]
pub struct PxConstraints2d {
    /// Constraints of lengths in the *x* or *width* dimension.
    pub x: PxConstraints,
    /// Constraints of lengths in the *y* or *height* dimension.
    pub y: PxConstraints,
}
impl PxConstraints2d {
    /// New unbounded constrain.
    pub fn new_unbounded() -> Self {
        Self {
            x: PxConstraints::new_unbounded(),
            y: PxConstraints::new_unbounded(),
        }
    }

    /// New bounded between zero and `max_y`, `max_y` with no fill.
    pub fn new_bounded(max_x: Px, max_y: Px) -> Self {
        Self {
            x: PxConstraints::new_bounded(max_x),
            y: PxConstraints::new_bounded(max_y),
        }
    }

    /// New bounded between zero and `max` with no fill.
    pub fn new_bounded_size(max: PxSize) -> Self {
        Self::new_bounded(max.width, max.height)
    }

    /// New bounded to only allow the *size* and fill.
    ///
    /// The type [`PxSize`] can also be converted into fixed constraints.
    pub fn new_exact(x: Px, y: Px) -> Self {
        Self {
            x: PxConstraints::new_exact(x),
            y: PxConstraints::new_exact(y),
        }
    }

    /// New bounded to only allow the `size` and fill.
    pub fn new_exact_size(size: PxSize) -> Self {
        Self::new_exact(size.width, size.height)
    }

    /// New bounded to fill the maximum `x` and `y`.
    pub fn new_fill(x: Px, y: Px) -> Self {
        Self {
            x: PxConstraints::new_fill(x),
            y: PxConstraints::new_fill(y),
        }
    }

    /// New bounded to fill the maximum `size`.
    pub fn new_fill_size(size: PxSize) -> Self {
        Self::new_fill(size.width, size.height)
    }

    /// New bounded to a inclusive range.
    ///
    /// A tuple of two [`PxSize`] values can also be converted to these constraints.
    ///
    /// # Panics
    ///
    /// Panics if min is greater then max.
    pub fn new_range(min_x: Px, max_x: Px, min_y: Px, max_y: Px) -> Self {
        Self {
            x: PxConstraints::new_range(min_x, max_x),
            y: PxConstraints::new_range(min_y, max_y),
        }
    }

    /// Returns a copy of the current constraints that has `min_x` and `min_y` as the lower
    /// bound and max adjusted to be >= min in both axis.
    pub fn with_new_min(mut self, min_x: Px, min_y: Px) -> Self {
        self.x = self.x.with_new_min(min_x);
        self.y = self.y.with_new_min(min_y);
        self
    }

    /// Returns a copy of the current constraints that has `min_x` and `min_y` as the lower
    /// bound and max adjusted to be >= min in both axis, if the new min is greater then the current min.
    pub fn with_min(mut self, min_x: Px, min_y: Px) -> Self {
        self.x = self.x.with_min(min_x);
        self.y = self.y.with_min(min_y);
        self
    }

    /// Returns a copy of the current constraints that has `min` as the lower
    /// bound and max adjusted to be >= min in both axis.
    pub fn with_new_min_size(self, min: PxSize) -> Self {
        self.with_new_min(min.width, min.height)
    }

    /// Returns a copy of the current constraints that has `min` as the lower
    /// bound and max adjusted to be >= min in both axis, if the new min is greater then the current min.
    pub fn with_min_size(self, min: PxSize) -> Self {
        self.with_min(min.width, min.height)
    }

    /// Returns a copy of the current constraints that has `min_x` as the lower
    /// bound and max adjusted to be >= min in the **x** axis.
    pub fn with_new_min_x(mut self, min_x: Px) -> Self {
        self.x = self.x.with_new_min(min_x);
        self
    }

    /// Returns a copy of the current constraints that has `min_y` as the lower
    /// bound and max adjusted to be >= min in the **y** axis.
    pub fn with_new_min_y(mut self, min_y: Px) -> Self {
        self.y = self.y.with_new_min(min_y);
        self
    }

    /// Returns a copy of the current constraints that has `min_x` as the lower
    /// bound and max adjusted to be >= min in the **x** axis if the new min is greater then the current min.
    pub fn with_min_x(mut self, min_x: Px) -> Self {
        self.x = self.x.with_min(min_x);
        self
    }

    /// Returns a copy of the current constraints that has `min_y` as the lower
    /// bound and max adjusted to be >= min in the **y** axis if the new min is greater then the current min.
    pub fn with_min_y(mut self, min_y: Px) -> Self {
        self.y = self.y.with_min(min_y);
        self
    }

    /// Returns a copy of the current constraints that has `max_x` and `max_y` as the upper
    /// bound and min adjusted to be <= max in both axis.
    pub fn with_new_max(mut self, max_x: Px, max_y: Px) -> Self {
        self.x = self.x.with_new_max(max_x);
        self.y = self.y.with_new_max(max_y);
        self
    }

    /// Returns a copy of the current constraints that has `max_x` and `max_y` as the upper
    /// bound and min adjusted to be <= max in both axis if the new max if less then the current max.
    pub fn with_max(mut self, max_x: Px, max_y: Px) -> Self {
        self.x = self.x.with_max(max_x);
        self.y = self.y.with_max(max_y);
        self
    }

    /// Returns a copy of the current constraints that has `max` as the upper
    /// bound and min adjusted to be <= max in both axis.
    pub fn with_new_max_size(self, max: PxSize) -> Self {
        self.with_new_max(max.width, max.height)
    }

    /// Returns a copy of the current constraints that has `max` as the upper
    /// bound and min adjusted to be <= max in both axis if the new max if less then the current max.
    pub fn with_max_size(self, max: PxSize) -> Self {
        self.with_max(max.width, max.height)
    }

    /// Returns a copy of the current constraints that has `min_x` as the lower
    /// bound and max adjusted to be << max in the **x** axis.
    pub fn with_new_max_x(mut self, max_x: Px) -> Self {
        self.x = self.x.with_new_max(max_x);
        self
    }

    /// Returns a copy of the current constraints that has `max_y` as the lower
    /// bound and min adjusted to be <= max in the **y** axis.
    pub fn with_new_max_y(mut self, max_y: Px) -> Self {
        self.y = self.y.with_new_max(max_y);
        self
    }

    /// Returns a copy of the current constraints that has `min_x` as the lower
    /// bound and max adjusted to be << max in the **x** axis if the new max if less then the current max.
    pub fn with_max_x(mut self, max_x: Px) -> Self {
        self.x = self.x.with_max(max_x);
        self
    }

    /// Returns a copy of the current constraints that has `max_y` as the lower
    /// bound and min adjusted to be <= max in the **y** axis if the new max if less then the current max.
    pub fn with_max_y(mut self, max_y: Px) -> Self {
        self.y = self.y.with_max(max_y);
        self
    }

    /// Returns a copy with min and max bounds set to `x` and `y`.
    pub fn with_new_exact(mut self, x: Px, y: Px) -> Self {
        self.x = self.x.with_new_exact(x);
        self.y = self.y.with_new_exact(y);
        self
    }

    /// Returns a copy with min and max bounds set to `x` and `y` clamped by the current constraints.
    pub fn with_exact(mut self, x: Px, y: Px) -> Self {
        self.x = self.x.with_exact(x);
        self.y = self.y.with_exact(y);
        self
    }

    /// Returns a copy with min and max bounds set to `size`.
    pub fn with_new_exact_size(self, size: PxSize) -> Self {
        self.with_new_exact(size.width, size.height)
    }

    /// Returns a copy with min and max bounds set to `size` clamped by the current constraints.
    pub fn with_exact_size(self, size: PxSize) -> Self {
        self.with_exact(size.width, size.height)
    }

    /// Returns a copy of the current constraints with the **x** maximum and minimum set to `x`.
    pub fn with_new_exact_x(mut self, x: Px) -> Self {
        self.x = self.x.with_new_exact(x);
        self
    }

    /// Returns a copy of the current constraints with the **y** maximum and minimum set to `y`.
    pub fn with_new_exact_y(mut self, y: Px) -> Self {
        self.y = self.y.with_new_exact(y);
        self
    }

    /// Returns a copy of the current constraints with the **x** maximum and minimum set to `x`
    /// clamped by the current constraints.
    pub fn with_exact_x(mut self, x: Px) -> Self {
        self.x = self.x.with_exact(x);
        self
    }

    /// Returns a copy of the current constraints with the **y** maximum and minimum set to `y`
    /// clamped by the current constraints.
    pub fn with_exact_y(mut self, y: Px) -> Self {
        self.y = self.y.with_exact(y);
        self
    }

    /// Returns a copy of the current constraints that sets the `fill_x` and `fill_y` preference.
    pub fn with_fill(mut self, fill_x: bool, fill_y: bool) -> Self {
        self.x = self.x.with_fill(fill_x);
        self.y = self.y.with_fill(fill_y);
        self
    }

    /// Returns a copy of the current constraints that sets the fill preference to *current && fill*.
    pub fn with_fill_and(mut self, fill_x: bool, fill_y: bool) -> Self {
        self.x = self.x.with_fill_and(fill_x);
        self.y = self.y.with_fill_and(fill_y);
        self
    }

    /// Returns a copy of the current constraints that sets the `fill` preference
    pub fn with_fill_vector(self, fill: BoolVector2D) -> Self {
        self.with_fill(fill.x, fill.y)
    }

    /// Returns a copy of the current constraints that sets the `fill_x` preference.
    pub fn with_fill_x(mut self, fill_x: bool) -> Self {
        self.x = self.x.with_fill(fill_x);
        self
    }

    /// Returns a copy of the current constraints that sets the `fill_y` preference.
    pub fn with_fill_y(mut self, fill_y: bool) -> Self {
        self.y = self.y.with_fill(fill_y);
        self
    }

    /// Returns a copy of the current constraints without upper bound in both axis.
    pub fn with_unbounded(mut self) -> Self {
        self.x = self.x.with_unbounded();
        self.y = self.y.with_unbounded();
        self
    }

    /// Returns a copy of the current constraints without a upper bound in the **x** axis.
    pub fn with_unbounded_x(mut self) -> Self {
        self.x = self.x.with_unbounded();
        self
    }

    /// Returns a copy of the current constraints without a upper bound in the **y** axis.
    pub fn with_unbounded_y(mut self) -> Self {
        self.y = self.y.with_unbounded();
        self
    }

    /// Returns a copy of the current constraints with `sub_x` and `sub_y` subtracted from the min and max bounds.
    ///
    /// The subtraction is saturating, does not subtract max if unbounded.
    pub fn with_less(mut self, sub_x: Px, sub_y: Px) -> Self {
        self.x = self.x.with_less(sub_x);
        self.y = self.y.with_less(sub_y);
        self
    }

    /// Returns a copy of the current constraints with `sub` subtracted from the min and max bounds.
    ///
    /// The subtraction is saturating, does not subtract max if unbounded.
    pub fn with_less_size(self, sub: PxSize) -> Self {
        self.with_less(sub.width, sub.height)
    }

    /// Returns a copy of the current constraints with `sub_x` subtracted from the min and max bounds of the **x** axis.
    ///
    /// The subtraction is saturating, does not subtract max if unbounded.
    pub fn with_less_x(mut self, sub_x: Px) -> Self {
        self.x = self.x.with_less(sub_x);
        self
    }

    /// Returns a copy of the current constraints with `sub_y` subtracted from the min and max bounds of the **y** axis.
    ///
    /// The subtraction is saturating, does not subtract max if unbounded.
    pub fn with_less_y(mut self, sub_y: Px) -> Self {
        self.y = self.y.with_less(sub_y);
        self
    }

    /// Returns a copy of the current constraints with `add_x` and `add_y` added to the maximum bounds.
    ///
    /// Does a saturation addition, this can potentially unbound the constraints if [`Px::MAX`] is reached.
    pub fn with_more(mut self, add_x: Px, add_y: Px) -> Self {
        self.x = self.x.with_more(add_x);
        self.y = self.y.with_more(add_y);
        self
    }

    /// Returns a copy of the current constraints with `add` added to the maximum bounds.
    ///
    /// Does a saturation addition, this can potentially unbound the constraints if [`Px::MAX`] is reached.
    pub fn with_more_size(self, add: PxSize) -> Self {
        self.with_more(add.width, add.height)
    }

    /// Returns a copy of the current constraints with [`x`] modified by the closure.
    ///
    /// [`x`]: Self::x
    pub fn with_x(mut self, x: impl FnOnce(PxConstraints) -> PxConstraints) -> Self {
        self.x = x(self.x);
        self
    }

    /// Returns a copy of the current constraints with [`y`] modified by the closure.
    ///
    /// [`y`]: Self::y
    pub fn with_y(mut self, y: impl FnOnce(PxConstraints) -> PxConstraints) -> Self {
        self.y = y(self.y);
        self
    }

    /// Gets if the constraints have an upper bound.
    pub fn is_bounded(self) -> BoolVector2D {
        BoolVector2D {
            x: self.x.is_bounded(),
            y: self.y.is_bounded(),
        }
    }

    /// Gets if the constraints have no upper bound.
    pub fn is_unbounded(self) -> BoolVector2D {
        BoolVector2D {
            x: self.x.is_unbounded(),
            y: self.y.is_unbounded(),
        }
    }

    /// Gets if the constraints only allow one length.
    pub fn is_exact(self) -> BoolVector2D {
        BoolVector2D {
            x: self.x.is_exact(),
            y: self.y.is_exact(),
        }
    }

    /// Gets if the context prefers the maximum length over the minimum.
    ///
    /// Note that if the constraints are unbounded there is not maximum length, in this case the fill length is the minimum.
    pub fn is_fill_pref(self) -> BoolVector2D {
        BoolVector2D {
            x: self.x.is_fill_pref(),
            y: self.y.is_fill_pref(),
        }
    }

    /// Gets if the context prefers the maximum length over the minimum and there is a maximum length.
    pub fn is_fill_max(self) -> BoolVector2D {
        BoolVector2D {
            x: self.x.is_fill_max(),
            y: self.y.is_fill_max(),
        }
    }

    /// Gets the fixed size if the constraints only allow one length in both axis.
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

    /// Gets the maximum if fill is preferred and max is bounded, or `size` clamped by the constraints.
    pub fn fill_size_or(self, size: PxSize) -> PxSize {
        PxSize::new(self.x.fill_or(size.width), self.y.fill_or(size.height))
    }

    /// Gets the max size if is fill and has max bounds, or gets the exact size if min equals max.
    pub fn fill_or_exact(self) -> Option<PxSize> {
        Some(PxSize::new(self.x.fill_or_exact()?, self.y.fill_or_exact()?))
    }

    /// Gets the maximum size if bounded, or the `size` clamped by constraints.
    pub fn max_size_or(self, size: PxSize) -> PxSize {
        PxSize::new(self.x.max_or(size.width), self.y.max_or(size.height))
    }

    /// Gets the maximum size if bounded, or the minimum if not.
    pub fn max_bounded_size(self) -> PxSize {
        PxSize::new(self.x.max_bounded(), self.y.max_bounded())
    }

    /// Gets the maximum fill size that preserves the `size` ratio.
    pub fn fill_ratio(self, size: PxSize) -> PxSize {
        if self.x.is_unbounded() {
            if self.y.is_unbounded() {
                // cover min
                let container = size.max(self.min_size()).to_f32();
                let content = size.to_f32();
                let scale = (container.width / content.width).max(container.height / content.height).fct();
                size * scale
            } else {
                // expand height
                let height = self.y.fill_or(size.height.max(self.y.min));
                let scale = (height.0 as f32 / size.height.0 as f32).fct();
                PxSize::new(size.width * scale, height)
            }
        } else if self.y.is_unbounded() {
            // expand width
            let width = self.x.fill_or(size.width.max(self.x.min));
            let scale = (width.0 as f32 / size.width.0 as f32).fct();
            PxSize::new(width, size.height * scale)
        } else if self.x.is_fill_pref() || self.y.is_fill_pref() {
            // contain max & clamp min
            let container = self.fill_size_or(size).to_f32();
            let content = size.to_f32();
            let scale = (container.width / content.width).min(container.height / content.height).fct();

            (size * scale).max(self.min_size())
        } else {
            // cover min & clamp max
            let container = self.min_size().to_f32();
            let content = size.to_f32();
            let scale = (container.width / content.width).max(container.height / content.height).fct();

            (size * scale).min(PxSize::new(self.x.max, self.y.max))
        }
    }
}
impl_from_and_into_var! {
    /// New exact.
    fn from(size: PxSize) -> PxConstraints2d {
        PxConstraints2d::new_exact(size.width, size.height)
    }

    /// New range, the minimum and maximum is computed.
    fn from((a, b): (PxSize, PxSize)) -> PxConstraints2d {
        PxConstraints2d {
            x: if a.width > b.width {
                PxConstraints::new_range(b.width, a.width)
            } else {
                PxConstraints::new_range(a.width, b.width)
            },
            y: if a.height > b.height {
                PxConstraints::new_range(b.height, a.height)
            } else {
                PxConstraints::new_range(a.height, b.height)
            },
        }
    }
}
impl Default for PxConstraints2d {
    fn default() -> Self {
        Self::new_unbounded()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_ratio_unbounded_no_min() {
        let constraints = PxConstraints2d::new_unbounded();

        let size = PxSize::new(Px(400), Px(200));
        let filled = constraints.fill_ratio(size);

        assert_eq!(size, filled)
    }

    #[test]
    fn fill_ratio_unbounded_with_min_x() {
        let constraints = PxConstraints2d::new_unbounded().with_min_x(Px(800));

        let size = PxSize::new(Px(400), Px(200));
        let filled = constraints.fill_ratio(size);

        assert_eq!(filled, PxSize::new(Px(800), Px(400)))
    }

    #[test]
    fn fill_ratio_unbounded_with_min_y() {
        let constraints = PxConstraints2d::new_unbounded().with_min_y(Px(400));

        let size = PxSize::new(Px(400), Px(200));
        let filled = constraints.fill_ratio(size);

        assert_eq!(filled, PxSize::new(Px(800), Px(400)))
    }

    #[test]
    fn fill_ratio_bounded_x() {
        let constraints = PxConstraints2d::new_fill(Px(800), Px::MAX);

        let size = PxSize::new(Px(400), Px(200));
        let filled = constraints.fill_ratio(size);

        assert_eq!(filled, PxSize::new(Px(800), Px(400)))
    }

    #[test]
    fn fill_ratio_bounded_y() {
        let constraints = PxConstraints2d::new_fill(Px::MAX, Px(400));

        let size = PxSize::new(Px(400), Px(200));
        let filled = constraints.fill_ratio(size);

        assert_eq!(filled, PxSize::new(Px(800), Px(400)))
    }

    #[test]
    fn fill_ratio_bounded1() {
        let constraints = PxConstraints2d::new_fill(Px(800), Px(400));

        let size = PxSize::new(Px(400), Px(200));
        let filled = constraints.fill_ratio(size);

        assert_eq!(filled, PxSize::new(Px(800), Px(400)))
    }

    #[test]
    fn fill_ratio_bounded2() {
        let constraints = PxConstraints2d::new_fill(Px(400), Px(400));

        let size = PxSize::new(Px(400), Px(200));
        let filled = constraints.fill_ratio(size);

        assert_eq!(filled, PxSize::new(Px(400), Px(200)))
    }

    #[test]
    fn fill_ratio_exact() {
        let constraints = PxConstraints2d::new_exact(Px(123), Px(321));

        let size = PxSize::new(Px(400), Px(200));
        let filled = constraints.fill_ratio(size);

        assert_eq!(filled, PxSize::new(Px(123), Px(321)))
    }

    #[test]
    fn fill_ratio_no_fill_bounded_with_min_x() {
        let constraints = PxConstraints2d::new_bounded(Px(1000), Px(1000)).with_min_x(Px(800));

        let size = PxSize::new(Px(400), Px(200));
        let filled = constraints.fill_ratio(size);

        assert_eq!(filled, PxSize::new(Px(800), Px(400)))
    }

    #[test]
    fn fill_ratio_no_fill_bounded_with_min_y() {
        let constraints = PxConstraints2d::new_bounded(Px(1000), Px(1000)).with_min_y(Px(400));

        let size = PxSize::new(Px(400), Px(200));
        let filled = constraints.fill_ratio(size);

        assert_eq!(filled, PxSize::new(Px(800), Px(400)))
    }
}
