use std::{fmt, mem, ops};

use crate::{context::LayoutMetrics, impl_from_and_into_var};

use super::{impl_length_comp_conversions, AvailableSize, DipPoint, LayoutMask, Length, PxPoint, Scale2d, Size, Vector};

/// 2D point in [`Length`] units.
#[derive(Clone, Default, PartialEq)]
pub struct Point {
    /// *x* offset in length units.
    pub x: Length,
    /// *y* offset in length units.
    pub y: Length,
}
impl fmt::Debug for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("Point").field("x", &self.x).field("y", &self.y).finish()
        } else {
            write!(f, "({:?}, {:?})", self.x, self.y)
        }
    }
}
impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(p) = f.precision() {
            write!(f, "({:.p$}, {:.p$})", self.x, self.y, p = p)
        } else {
            write!(f, "({}, {})", self.x, self.y)
        }
    }
}
impl Point {
    /// New x, y from any [`Length`] unit.
    pub fn new<X: Into<Length>, Y: Into<Length>>(x: X, y: Y) -> Self {
        Point { x: x.into(), y: y.into() }
    }

    /// New x, y from single value of any [`Length`] unit.
    pub fn splat(xy: impl Into<Length>) -> Self {
        let xy = xy.into();
        Point { x: xy.clone(), y: xy }
    }

    /// ***x:*** [`Length::zero`], ***y:*** [`Length::zero`].
    #[inline]
    pub fn zero() -> Self {
        Self::new(Length::zero(), Length::zero())
    }

    /// Point at the top-middle of the available space.
    ///
    /// ***x:*** [`Length::half`], ***y:*** [`Length::zero`]
    #[inline]
    pub fn top() -> Self {
        Self::new(Length::half(), Length::zero())
    }

    /// Point at the bottom-middle of the available space.
    ///
    /// ***x:*** [`Length::half`], ***y:*** [`Length::fill`]
    #[inline]
    pub fn bottom() -> Self {
        Self::new(Length::half(), Length::fill())
    }

    /// Point at the middle-left of the available space.
    ///
    /// ***x:*** [`Length::zero`], ***y:*** [`Length::half`]
    #[inline]
    pub fn left() -> Self {
        Self::new(Length::zero(), Length::half())
    }

    /// Point at the middle-right of the available space.
    ///
    /// ***x:*** [`Length::fill`], ***y:*** [`Length::half`]
    #[inline]
    pub fn right() -> Self {
        Self::new(Length::fill(), Length::half())
    }

    /// Point at the top-left of the available space.
    ///
    /// ***x:*** [`Length::zero`], ***y:*** [`Length::zero`]
    #[inline]
    pub fn top_left() -> Self {
        Self::zero()
    }

    /// Point at the top-right of the available space.
    ///
    /// ***x:*** [`Length::fill`], ***y:*** [`Length::zero`]
    #[inline]
    pub fn top_right() -> Self {
        Self::new(Length::fill(), Length::zero())
    }

    /// Point at the bottom-left of the available space.
    ///
    /// ***x:*** [`Length::zero`], ***y:*** [`Length::fill`]
    #[inline]
    pub fn bottom_left() -> Self {
        Self::new(Length::zero(), Length::fill())
    }

    /// Point at the bottom-right of the available space.
    ///
    /// ***x:*** [`Length::fill`], ***y:*** [`Length::fill`]
    #[inline]
    pub fn bottom_right() -> Self {
        Self::new(Length::fill(), Length::fill())
    }

    /// Point at the center.
    ///
    /// ***x:*** [`Length::half`], ***y:*** [`Length::half`]
    #[inline]
    pub fn center() -> Self {
        Self::new(Length::half(), Length::half())
    }

    /// Swap `x` and `y`.
    #[inline]
    pub fn yx(self) -> Self {
        Point { y: self.x, x: self.y }
    }

    /// Returns `(x, y)`.
    #[inline]
    pub fn as_tuple(self) -> (Length, Length) {
        (self.x, self.y)
    }

    /// Returns `[x, y]`.
    #[inline]
    pub fn as_array(self) -> [Length; 2] {
        [self.x, self.y]
    }

    /// Compute the point in a layout context.
    #[inline]
    pub fn to_layout(&self, ctx: &LayoutMetrics, available_size: AvailableSize, default_value: PxPoint) -> PxPoint {
        PxPoint::new(
            self.x.to_layout(ctx, available_size.width, default_value.x),
            self.y.to_layout(ctx, available_size.height, default_value.y),
        )
    }

    /// Compute a [`LayoutMask`] that flags all contextual values that affect the result of [`to_layout`].
    ///
    /// [`to_layout`]: Self::to_layout
    #[inline]
    pub fn affect_mask(&self) -> LayoutMask {
        self.x.affect_mask() | self.y.affect_mask()
    }

    /// Returns `true` if all values are [`Length::Default`].
    pub fn is_default(&self) -> bool {
        self.x.is_default() && self.y.is_default()
    }

    /// Replaces [`Length::Default`] values with `overwrite` values.
    pub fn replace_default(&mut self, overwrite: &Point) {
        self.x.replace_default(&overwrite.x);
        self.y.replace_default(&overwrite.y);
    }

    /// Cast to [`Vector`].
    pub fn as_vector(self) -> Vector {
        Vector { x: self.x, y: self.y }
    }
}
impl_length_comp_conversions! {
    fn from(x: X, y: Y) -> Point {
        Point::new(x, y)
    }
}
impl_from_and_into_var! {
    fn from(p: PxPoint) -> Point {
        Point::new(p.x, p.y)
    }
    fn from(p: DipPoint) -> Point {
        Point::new(p.x, p.y)
    }
}
impl ops::Add<Vector> for Point {
    type Output = Self;

    fn add(self, rhs: Vector) -> Self {
        Point {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}
impl ops::Add<Size> for Point {
    type Output = Self;

    fn add(self, rhs: Size) -> Self {
        Point {
            x: self.x + rhs.width,
            y: self.y + rhs.height,
        }
    }
}
impl ops::AddAssign<Vector> for Point {
    fn add_assign(&mut self, rhs: Vector) {
        let x = mem::take(&mut self.x);
        let y = mem::take(&mut self.y);

        self.x = x + rhs.x;
        self.y = y + rhs.y;
    }
}
impl ops::AddAssign<Size> for Point {
    fn add_assign(&mut self, rhs: Size) {
        let x = mem::take(&mut self.x);
        let y = mem::take(&mut self.y);

        self.x = x + rhs.width;
        self.y = y + rhs.height;
    }
}
impl ops::Sub<Vector> for Point {
    type Output = Self;

    fn sub(self, rhs: Vector) -> Self {
        Point {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}
impl ops::Sub<Size> for Point {
    type Output = Self;

    fn sub(self, rhs: Size) -> Self {
        Point {
            x: self.x - rhs.width,
            y: self.y - rhs.height,
        }
    }
}
impl ops::SubAssign<Vector> for Point {
    fn sub_assign(&mut self, rhs: Vector) {
        let x = mem::take(&mut self.x);
        let y = mem::take(&mut self.y);

        self.x = x - rhs.x;
        self.y = y - rhs.y;
    }
}
impl ops::SubAssign<Size> for Point {
    fn sub_assign(&mut self, rhs: Size) {
        let x = mem::take(&mut self.x);
        let y = mem::take(&mut self.y);

        self.x = x - rhs.width;
        self.y = y - rhs.height;
    }
}
impl<S: Into<Scale2d>> ops::Mul<S> for Point {
    type Output = Self;

    fn mul(self, rhs: S) -> Self {
        let fct = rhs.into();

        Point {
            x: self.x * fct.x,
            y: self.y * fct.y,
        }
    }
}
impl<S: Into<Scale2d>> ops::MulAssign<S> for Point {
    fn mul_assign(&mut self, rhs: S) {
        let x = mem::take(&mut self.x);
        let y = mem::take(&mut self.y);
        let fct = rhs.into();

        self.x = x * fct.x;
        self.y = y * fct.y;
    }
}
impl<S: Into<Scale2d>> ops::Div<S> for Point {
    type Output = Self;

    fn div(self, rhs: S) -> Self {
        let fct = rhs.into();

        Point {
            x: self.x / fct.x,
            y: self.y / fct.y,
        }
    }
}
impl<S: Into<Scale2d>> ops::DivAssign<S> for Point {
    fn div_assign(&mut self, rhs: S) {
        let x = mem::take(&mut self.x);
        let y = mem::take(&mut self.y);
        let fct = rhs.into();

        self.x = x / fct.x;
        self.y = y / fct.y;
    }
}
impl ops::Neg for Point {
    type Output = Self;

    fn neg(self) -> Self {
        Point { x: -self.x, y: -self.y }
    }
}
