use std::{fmt, ops};

use crate::{context::LayoutMetrics, impl_from_and_into_var};

use super::{impl_length_comp_conversions, AvailableSize, DipRect, Factor2d, LayoutMask, Length, Point, PxRect, Size, Vector};

/// 2D rect in [`Length`] units.
#[derive(Clone, Default, PartialEq)]
pub struct Rect {
    /// Top-left origin of the rectangle in length units.
    pub origin: Point,
    /// Size of the rectangle in length units.
    pub size: Size,
}
impl fmt::Debug for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("Rect")
                .field("origin", &self.origin)
                .field("size", &self.size)
                .finish()
        } else {
            write!(f, "{:?}.at{:?}", self.origin, self.size)
        }
    }
}
impl fmt::Display for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(p) = f.precision() {
            write!(f, "{:.p$} {:.p$}", self.origin, self.size, p = p)
        } else {
            write!(f, "{} {}", self.origin, self.size)
        }
    }
}
impl Rect {
    /// New rectangle defined by an origin point (top-left) and a size, both in any type that converts to
    /// [`Point`] and [`Size`].
    ///
    /// Also see [`RectFromTuplesBuilder`] for another way of initializing a rectangle value.
    pub fn new<O: Into<Point>, S: Into<Size>>(origin: O, size: S) -> Self {
        Rect {
            origin: origin.into(),
            size: size.into(),
        }
    }

    /// New rectangle at [`Point::zero`]. The size is in any [`Length`] unit.
    pub fn from_size<S: Into<Size>>(size: S) -> Self {
        Self::new(Point::zero(), size)
    }

    /// New rectangle at [`Point::zero`] and [`Size::zero`].
    #[inline]
    pub fn zero() -> Self {
        Self::new(Point::zero(), Size::zero())
    }

    /// Rect that fills the available space.
    #[inline]
    pub fn fill() -> Self {
        Self::from_size(Size::fill())
    }

    /// Min x and y, this is the [`origin`].
    ///
    /// [`origin`]: Self::origin
    #[inline]
    pub fn min(&self) -> Point {
        self.origin.clone()
    }

    /// Max x and y, this is the sum of [`origin`] and [`size`].
    ///
    /// [`origin`]: Self::origin
    /// [`size`]: Self::size
    #[inline]
    pub fn max(&self) -> Point {
        self.origin.clone() + self.size.clone()
    }

    /// Min x, this is the `origin.x`.
    #[inline]
    pub fn min_x(&self) -> Length {
        self.origin.x.clone()
    }
    /// Min y, this is the `origin.y`.
    #[inline]
    pub fn min_y(&self) -> Length {
        self.origin.y.clone()
    }

    /// Max x, this is the `origin.x + width`.
    #[inline]
    pub fn max_x(&self) -> Length {
        self.origin.x.clone() + self.size.width.clone()
    }
    /// Max y, this is the `origin.y + height`.
    #[inline]
    pub fn max_y(&self) -> Length {
        self.origin.y.clone() + self.size.height.clone()
    }

    /// Returns a rectangle of same size that adds the vector to the origin.
    #[inline]
    pub fn translate(&self, by: impl Into<Vector>) -> Self {
        let mut r = self.clone();
        r.origin += by.into();
        r
    }

    /// Compute the rectangle in a layout context.
    #[inline]
    pub fn to_layout(&self, ctx: &LayoutMetrics, available_size: AvailableSize, default_value: PxRect) -> PxRect {
        PxRect::new(
            self.origin.to_layout(ctx, available_size, default_value.origin),
            self.size.to_layout(ctx, available_size, default_value.size),
        )
    }

    /// Compute a [`LayoutMask`] that flags all contextual values that affect the result of [`to_layout`].
    ///
    /// [`to_layout`]: Self::to_layout
    #[inline]
    pub fn affect_mask(&self) -> LayoutMask {
        self.origin.affect_mask() | self.size.affect_mask()
    }

    /// Returns `true` if all values are [`Length::Default`].
    pub fn is_default(&self) -> bool {
        self.origin.is_default() && self.size.is_default()
    }

    /// Replaces [`Length::Default`] values with `overwrite` values.
    pub fn replace_default(&mut self, overwrite: &Rect) {
        self.origin.replace_default(&overwrite.origin);
        self.size.replace_default(&overwrite.size);
    }
}
impl_length_comp_conversions! {
    fn from(x: X, y: Y, width: W, height: H) -> Rect {
        Rect::new((x, y), (width, height))
    }
}
impl_from_and_into_var! {
    /// New in exact length.
    fn from(rect: PxRect) -> Rect {
        Rect::new(rect.origin, rect.size)
    }

    /// New in exact length.
    fn from(rect: DipRect) -> Rect {
        Rect::new(rect.origin, rect.size)
    }

    fn from(size: Size) -> Rect {
        Rect::from_size(size)
    }

    /// New from origin and size.
    fn from<O: Into<Point> + Clone, S: Into<Size> + Clone>((origin, size): (O, S)) -> Rect {
        Rect::new(origin, size)
    }
}
impl<S: Into<Factor2d>> ops::Mul<S> for Rect {
    type Output = Self;

    fn mul(mut self, rhs: S) -> Self {
        self *= rhs;
        self
    }
}
impl<'a, S: Into<Factor2d>> ops::Mul<S> for &'a Rect {
    type Output = Rect;

    fn mul(self, rhs: S) -> Self::Output {
        self.clone() * rhs
    }
}
impl<S: Into<Factor2d>> ops::MulAssign<S> for Rect {
    fn mul_assign(&mut self, rhs: S) {
        let rhs = rhs.into();
        self.origin *= rhs;
        self.size *= rhs;
    }
}
impl<S: Into<Factor2d>> ops::Div<S> for Rect {
    type Output = Self;

    fn div(mut self, rhs: S) -> Self {
        self /= rhs;
        self
    }
}
impl<'a, S: Into<Factor2d>> ops::Div<S> for &'a Rect {
    type Output = Rect;

    fn div(self, rhs: S) -> Self::Output {
        self.clone() / rhs
    }
}
impl<S: Into<Factor2d>> ops::DivAssign<S> for Rect {
    fn div_assign(&mut self, rhs: S) {
        let rhs = rhs.into();
        self.origin /= rhs;
        self.size /= rhs;
    }
}

/// Build a [`Rect`] using the syntax `(width, height).at(x, y)`.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::units::*;
/// let rect = (800, 600).at(10, 20);
/// assert_eq!(Rect::new(Point::new(10, 20), Size::new(800, 600)), rect);
/// ```
pub trait RectFromTuplesBuilder {
    /// New [`Rect`] from `self` as the size placed at the `x, y` origin.
    fn at<X: Into<Length>, Y: Into<Length>>(self, x: X, y: Y) -> Rect;
}
impl<W: Into<Length>, H: Into<Length>> RectFromTuplesBuilder for (W, H) {
    fn at<X: Into<Length>, Y: Into<Length>>(self, x: X, y: Y) -> Rect {
        Rect::new((x, y), self)
    }
}
