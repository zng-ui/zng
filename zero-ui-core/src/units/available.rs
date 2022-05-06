use std::{cmp, ops};

use super::{euclid, Factor, Px, PxSize};

/// Maximum [`Px`] available for an [`UiNode::measure`].
///
/// [`UiNode::measure`]: crate::UiNode::measure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AvailablePx {
    /// The measure may return any desired size, if it derives the size from
    /// the available size it should collapse to zero.
    Infinite,
    /// The measure must try to fit up-to this size.
    Finite(Px),
}
impl From<u32> for AvailablePx {
    fn from(px: u32) -> Self {
        AvailablePx::Finite(Px(px as i32))
    }
}
impl From<Px> for AvailablePx {
    fn from(px: Px) -> Self {
        AvailablePx::Finite(px)
    }
}
impl AvailablePx {
    /// Convert `Infinite` to zero, or returns the `Finite`.

    pub fn to_px(self) -> Px {
        self.to_px_or(Px(0))
    }

    /// Convert `Infinite` to `fallback` or return the `Finite`.

    pub fn to_px_or(self, fallback: Px) -> Px {
        match self {
            AvailablePx::Infinite => fallback,
            AvailablePx::Finite(p) => p,
        }
    }

    /// Returns the greater length.
    ///
    /// Infinite is greater then any finite value.

    pub fn max(self, other: AvailablePx) -> AvailablePx {
        if self > other {
            self
        } else {
            other
        }
    }

    /// Returns the lesser length.
    ///
    /// Infinite is greater then any finite value.

    pub fn min(self, other: AvailablePx) -> AvailablePx {
        if self < other {
            self
        } else {
            other
        }
    }

    /// Returns the greater finite length or `Infinite` if `self` is `Infinite`.

    pub fn max_px(self, other: Px) -> AvailablePx {
        self.max(AvailablePx::Finite(other))
    }

    /// Return the lesser finite length.

    pub fn min_px(self, other: Px) -> AvailablePx {
        self.min(AvailablePx::Finite(other))
    }

    /// Returns `true` if is `Infinite`.

    pub fn is_infinite(self) -> bool {
        matches!(self, AvailablePx::Infinite)
    }

    /// Returns `true` if is `Finite(_)`.

    pub fn is_finite(self) -> bool {
        matches!(self, AvailablePx::Finite(_))
    }
}
impl Default for AvailablePx {
    fn default() -> Self {
        AvailablePx::Infinite
    }
}
impl PartialOrd for AvailablePx {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for AvailablePx {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        match (self, other) {
            (AvailablePx::Infinite, AvailablePx::Infinite) => cmp::Ordering::Equal,
            (AvailablePx::Infinite, AvailablePx::Finite(_)) => cmp::Ordering::Greater,
            (AvailablePx::Finite(_), AvailablePx::Infinite) => cmp::Ordering::Less,
            (AvailablePx::Finite(s), AvailablePx::Finite(o)) => s.cmp(o),
        }
    }
}
impl PartialEq<Px> for AvailablePx {
    fn eq(&self, other: &Px) -> bool {
        match self {
            AvailablePx::Infinite => false,
            AvailablePx::Finite(s) => s == other,
        }
    }
}
impl PartialOrd<Px> for AvailablePx {
    fn partial_cmp(&self, other: &Px) -> Option<cmp::Ordering> {
        Some(match self {
            AvailablePx::Infinite => cmp::Ordering::Greater,
            AvailablePx::Finite(s) => s.cmp(other),
        })
    }
}
impl ops::Add<Px> for AvailablePx {
    type Output = AvailablePx;

    fn add(self, rhs: Px) -> Self::Output {
        match self {
            AvailablePx::Finite(px) => AvailablePx::Finite(px + rhs),
            s => s,
        }
    }
}
impl ops::AddAssign<Px> for AvailablePx {
    fn add_assign(&mut self, rhs: Px) {
        *self = *self + rhs;
    }
}
impl ops::Sub<Px> for AvailablePx {
    type Output = AvailablePx;

    fn sub(self, rhs: Px) -> Self::Output {
        match self {
            AvailablePx::Finite(px) => AvailablePx::Finite(px - rhs),
            s => s,
        }
    }
}
impl ops::SubAssign<Px> for AvailablePx {
    fn sub_assign(&mut self, rhs: Px) {
        *self = *self - rhs;
    }
}
impl ops::Mul<Factor> for AvailablePx {
    type Output = AvailablePx;

    fn mul(self, rhs: Factor) -> Self::Output {
        match self {
            AvailablePx::Finite(px) => AvailablePx::Finite(px * rhs),
            s => s,
        }
    }
}
impl ops::MulAssign<Factor> for AvailablePx {
    fn mul_assign(&mut self, rhs: Factor) {
        *self = *self * rhs;
    }
}
impl ops::Div<Factor> for AvailablePx {
    type Output = AvailablePx;

    fn div(self, rhs: Factor) -> Self::Output {
        match self {
            AvailablePx::Finite(px) => AvailablePx::Finite(px / rhs),
            s => s,
        }
    }
}
impl ops::DivAssign<Factor> for AvailablePx {
    fn div_assign(&mut self, rhs: Factor) {
        *self = *self / rhs;
    }
}
impl ops::Add<AvailablePx> for AvailablePx {
    type Output = AvailablePx;

    fn add(self, rhs: AvailablePx) -> Self::Output {
        match (self, rhs) {
            (AvailablePx::Infinite, _) | (_, AvailablePx::Infinite) => AvailablePx::Infinite,
            (AvailablePx::Finite(a), AvailablePx::Finite(b)) => AvailablePx::Finite(a + b),
        }
    }
}
impl ops::Sub<AvailablePx> for AvailablePx {
    type Output = AvailablePx;

    fn sub(self, rhs: AvailablePx) -> Self::Output {
        match (self, rhs) {
            (AvailablePx::Infinite, _) | (_, AvailablePx::Infinite) => AvailablePx::Infinite,
            (AvailablePx::Finite(a), AvailablePx::Finite(b)) => AvailablePx::Finite(a - b),
        }
    }
}

/// Maximum [`AvailablePx`] size for an [`UiNode::measure`].
///
/// Methods for this type are implemented by [`AvailableSizeExt`],
/// it must be imported together with this type definition.
///
/// [`UiNode::measure`]: crate::UiNode::measure
pub type AvailableSize = euclid::Size2D<AvailablePx, ()>;
/// Extension methods for [`AvailableSize`].
pub trait AvailableSizeExt {
    /// Width and height [`AvailablePx::Infinite`].
    fn inf() -> Self;
    /// New finite size.
    fn finite(size: PxSize) -> Self;

    /// Convert `Infinite` to zero, or returns the `Finite`.
    fn to_px(self) -> PxSize;
    /// Return the values of `fallback` for `Infinite`, otherwise returns the `Finite`.
    fn to_px_or(self, fallback: PxSize) -> PxSize;

    /// Increment the `Finite` value.
    ///
    /// Returns `Infinite` if `self` is infinite.
    fn add_px(self, size: PxSize) -> Self;
    /// Decrement the `Finite` value.
    ///
    /// Returns `Infinite` if `self` is infinite.
    fn sub_px(self, size: PxSize) -> Self;

    /// Returns a size that has the greater dimensions.
    fn max(self, other: Self) -> Self;
    /// Returns a size that has the lesser dimensions.
    fn min(self, other: Self) -> Self;

    /// Returns a size that has the greater dimensions.
    fn max_px(self, other: PxSize) -> Self;
    /// Returns a size that has the lesser finite dimensions.
    fn min_px(self, other: PxSize) -> Self;

    /// Returns the `desired_size` if infinite or the minimum size.
    fn clip(self, desired_size: PxSize) -> PxSize;

    /// Available size from finite size.
    fn from_size(size: PxSize) -> AvailableSize;
}
impl AvailableSizeExt for AvailableSize {
    fn inf() -> Self {
        AvailableSize::new(AvailablePx::Infinite, AvailablePx::Infinite)
    }

    fn finite(size: PxSize) -> Self {
        AvailableSize::new(AvailablePx::Finite(size.width), AvailablePx::Finite(size.height))
    }

    fn to_px(self) -> PxSize {
        PxSize::new(self.width.to_px(), self.height.to_px())
    }

    fn to_px_or(self, fallback: PxSize) -> PxSize {
        PxSize::new(self.width.to_px_or(fallback.width), self.height.to_px_or(fallback.height))
    }

    fn add_px(self, size: PxSize) -> Self {
        AvailableSize::new(self.width + size.width, self.height + size.height)
    }

    fn sub_px(self, size: PxSize) -> Self {
        AvailableSize::new(self.width - size.width, self.height - size.height)
    }

    fn max(self, other: Self) -> Self {
        AvailableSize::new(self.width.max(other.width), self.height.max(other.height))
    }

    fn min(self, other: Self) -> Self {
        AvailableSize::new(self.width.min(other.width), self.height.min(other.height))
    }

    fn max_px(self, other: PxSize) -> Self {
        AvailableSize::new(self.width.max_px(other.width), self.height.max_px(other.height))
    }

    fn min_px(self, other: PxSize) -> Self {
        AvailableSize::new(self.width.min_px(other.width), self.height.min_px(other.height))
    }

    fn clip(self, other: PxSize) -> PxSize {
        other.min(self.to_px_or(PxSize::new(Px::MAX, Px::MAX)))
    }

    fn from_size(size: PxSize) -> AvailableSize {
        AvailableSize::new(size.width.into(), size.height.into())
    }
}
