use std::{fmt, ops};

use zng_var::{animation::Transitionable, impl_from_and_into_var};

use crate::unit::{LengthCompositeParser, ParseCompositeError};

use super::{DipPoint, Factor, Factor2d, FactorPercent, Layout1d, LayoutMask, Length, PxPoint, Size, Vector, impl_length_comp_conversions};

/// 2D point in [`Length`] units.
#[derive(Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Transitionable)]
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
            write!(f, "({:.p$?}, {:.p$?})", self.x, self.y, p = f.precision().unwrap_or(0))
        }
    }
}
impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:.p$}, {:.p$}", self.x, self.y, p = f.precision().unwrap_or(0))
    }
}
impl std::str::FromStr for Point {
    type Err = ParseCompositeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parser = LengthCompositeParser::new(s)?;
        let a = parser.next()?;
        if parser.has_ended() {
            return Ok(Self::splat(a));
        }
        let b = parser.expect_last()?;
        Ok(Self::new(a, b))
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
    pub fn zero() -> Self {
        Self::new(Length::zero(), Length::zero())
    }

    /// Point at the top-middle of the available space.
    ///
    /// ***x:*** [`Length::half`], ***y:*** [`Length::zero`]
    pub fn top() -> Self {
        Self::new(Length::half(), Length::zero())
    }

    /// Point at the bottom-middle of the available space.
    ///
    /// ***x:*** [`Length::half`], ***y:*** [`Length::fill`]
    pub fn bottom() -> Self {
        Self::new(Length::half(), Length::fill())
    }

    /// Point at the middle-left of the available space.
    ///
    /// ***x:*** [`Length::zero`], ***y:*** [`Length::half`]
    pub fn left() -> Self {
        Self::new(Length::zero(), Length::half())
    }

    /// Point at the middle-right of the available space.
    ///
    /// ***x:*** [`Length::fill`], ***y:*** [`Length::half`]
    pub fn right() -> Self {
        Self::new(Length::fill(), Length::half())
    }

    /// Point at the top-left of the available space.
    ///
    /// ***x:*** [`Length::zero`], ***y:*** [`Length::zero`]
    pub fn top_left() -> Self {
        Self::zero()
    }

    /// Point at the top-right of the available space.
    ///
    /// ***x:*** [`Length::fill`], ***y:*** [`Length::zero`]
    pub fn top_right() -> Self {
        Self::new(Length::fill(), Length::zero())
    }

    /// Point at the bottom-left of the available space.
    ///
    /// ***x:*** [`Length::zero`], ***y:*** [`Length::fill`]
    pub fn bottom_left() -> Self {
        Self::new(Length::zero(), Length::fill())
    }

    /// Point at the bottom-right of the available space.
    ///
    /// ***x:*** [`Length::fill`], ***y:*** [`Length::fill`]
    pub fn bottom_right() -> Self {
        Self::new(Length::fill(), Length::fill())
    }

    /// Point at the center.
    ///
    /// ***x:*** [`Length::half`], ***y:*** [`Length::half`]
    pub fn center() -> Self {
        Self::new(Length::half(), Length::half())
    }

    /// Swap `x` and `y`.
    pub fn yx(self) -> Self {
        Point { y: self.x, x: self.y }
    }

    /// Returns `(x, y)`.
    pub fn as_tuple(self) -> (Length, Length) {
        (self.x, self.y)
    }

    /// Returns `[x, y]`.
    pub fn as_array(self) -> [Length; 2] {
        [self.x, self.y]
    }

    /// Returns `true` if all values are [`Length::Default`].
    pub fn is_default(&self) -> bool {
        self.x.is_default() && self.y.is_default()
    }

    /// Returns `true` if any value is [`Length::Default`].
    pub fn has_default(&self) -> bool {
        self.x.has_default() || self.y.has_default()
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
impl super::Layout2d for Point {
    type Px = PxPoint;

    fn layout_dft(&self, default: Self::Px) -> Self::Px {
        PxPoint::new(self.x.layout_dft_x(default.x), self.y.layout_dft_y(default.y))
    }

    fn affect_mask(&self) -> LayoutMask {
        self.x.affect_mask() | self.y.affect_mask()
    }
}
impl_length_comp_conversions! {
    fn from(x: X, y: Y) -> Point {
        Point::new(x, y)
    }
}
impl_from_and_into_var! {
    /// Splat.
    fn from(all: Length) -> Point {
        Point::splat(all)
    }
    /// Splat relative length.
    fn from(percent: FactorPercent) -> Point {
        Point::splat(percent)
    }
    /// Splat relative length.
    fn from(norm: Factor) -> Point {
        Point::splat(norm)
    }

    /// Splat exact length.
    fn from(f: f32) -> Point {
        Point::splat(f)
    }
    /// Splat exact length.
    fn from(i: i32) -> Point {
        Point::splat(i)
    }
    fn from(p: PxPoint) -> Point {
        Point::new(p.x, p.y)
    }
    fn from(p: DipPoint) -> Point {
        Point::new(p.x, p.y)
    }
    fn from(v: Vector) -> Point {
        v.as_point()
    }
}
impl<V: Into<Vector>> ops::Add<V> for Point {
    type Output = Self;

    fn add(mut self, rhs: V) -> Self {
        self += rhs;
        self
    }
}
impl<'a> ops::Add<&'a Vector> for &Point {
    type Output = Point;

    fn add(self, rhs: &'a Vector) -> Self::Output {
        self.clone() + rhs.clone()
    }
}
impl<'a> ops::Add<&'a Size> for &Point {
    type Output = Point;

    fn add(self, rhs: &'a Size) -> Self::Output {
        self.clone() + rhs.clone()
    }
}
impl<V: Into<Vector>> ops::AddAssign<V> for Point {
    fn add_assign(&mut self, rhs: V) {
        let rhs = rhs.into();
        self.x += rhs.x;
        self.y += rhs.y;
    }
}
impl<'a> ops::AddAssign<&'a Vector> for Point {
    fn add_assign(&mut self, rhs: &'a Vector) {
        *self += rhs.clone();
    }
}
impl<'a> ops::AddAssign<&'a Size> for Point {
    fn add_assign(&mut self, rhs: &'a Size) {
        *self += rhs.clone();
    }
}
impl<V: Into<Vector>> ops::Sub<V> for Point {
    type Output = Self;

    fn sub(mut self, rhs: V) -> Self {
        self -= rhs;
        self
    }
}
impl<'a> ops::Sub<&'a Vector> for &Point {
    type Output = Point;

    fn sub(self, rhs: &'a Vector) -> Self::Output {
        self.clone() - rhs.clone()
    }
}
impl<'a> ops::Sub<&'a Size> for &Point {
    type Output = Point;

    fn sub(self, rhs: &'a Size) -> Self::Output {
        self.clone() - rhs.clone()
    }
}
impl<V: Into<Vector>> ops::SubAssign<V> for Point {
    fn sub_assign(&mut self, rhs: V) {
        let rhs = rhs.into();
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}
impl<'a> ops::SubAssign<&'a Vector> for Point {
    fn sub_assign(&mut self, rhs: &'a Vector) {
        *self -= rhs.clone();
    }
}
impl<'a> ops::SubAssign<&'a Size> for Point {
    fn sub_assign(&mut self, rhs: &'a Size) {
        *self -= rhs.clone();
    }
}
impl<S: Into<Factor2d>> ops::Mul<S> for Point {
    type Output = Self;

    fn mul(mut self, rhs: S) -> Self {
        self *= rhs;
        self
    }
}
impl<S: Into<Factor2d>> ops::Mul<S> for &Point {
    type Output = Point;

    fn mul(self, rhs: S) -> Self::Output {
        self.clone() * rhs
    }
}
impl<S: Into<Factor2d>> ops::MulAssign<S> for Point {
    fn mul_assign(&mut self, rhs: S) {
        let rhs = rhs.into();
        self.x *= rhs.x;
        self.y *= rhs.y;
    }
}
impl<S: Into<Factor2d>> ops::Div<S> for Point {
    type Output = Self;

    fn div(mut self, rhs: S) -> Self {
        self /= rhs;
        self
    }
}
impl<S: Into<Factor2d>> ops::Div<S> for &Point {
    type Output = Point;

    fn div(self, rhs: S) -> Self::Output {
        self.clone() / rhs
    }
}
impl<S: Into<Factor2d>> ops::DivAssign<S> for Point {
    fn div_assign(&mut self, rhs: S) {
        let rhs = rhs.into();
        self.x /= rhs.x;
        self.y /= rhs.y;
    }
}
impl ops::Neg for Point {
    type Output = Self;

    fn neg(self) -> Self {
        Point { x: -self.x, y: -self.y }
    }
}

impl ops::Neg for &Point {
    type Output = Point;

    fn neg(self) -> Self::Output {
        -self.clone()
    }
}
