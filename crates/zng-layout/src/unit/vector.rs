use std::{fmt, ops};

use zng_var::{animation::Transitionable, impl_from_and_into_var};

use super::{
    Dip, DipVector, Factor, Factor2d, FactorPercent, Layout1d, LayoutMask, Length, LengthUnits, Point, Px, PxVector, Size, Transform,
    impl_length_comp_conversions,
};

/// 2D vector in [`Length`] units.
#[derive(Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Transitionable)]
pub struct Vector {
    /// *x* displacement in length units.
    pub x: Length,
    /// *y* displacement in length units.
    pub y: Length,
}
impl fmt::Debug for Vector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("Vector").field("x", &self.x).field("y", &self.y).finish()
        } else {
            write!(f, "({:?}, {:?})", self.x, self.y)
        }
    }
}
impl fmt::Display for Vector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(p) = f.precision() {
            write!(f, "({:.p$}, {:.p$})", self.x, self.y, p = p)
        } else {
            write!(f, "({}, {})", self.x, self.y)
        }
    }
}
impl Vector {
    /// New x, y from any [`Length`] unit.
    pub fn new<X: Into<Length>, Y: Into<Length>>(x: X, y: Y) -> Self {
        Vector { x: x.into(), y: y.into() }
    }

    /// New x, y from single value of any [`Length`] unit.
    pub fn splat(xy: impl Into<Length>) -> Self {
        let xy = xy.into();
        Vector { x: xy.clone(), y: xy }
    }

    /// ***x:*** [`Length::zero`], ***y:*** [`Length::zero`].
    pub fn zero() -> Self {
        Self::new(Length::zero(), Length::zero())
    }

    /// `(1, 1)`.
    pub fn one() -> Self {
        Self::new(1, 1)
    }

    /// `(1.px(), 1.px())`.
    pub fn one_px() -> Self {
        Self::new(1.px(), 1.px())
    }

    /// Swap `x` and `y`.
    pub fn yx(self) -> Self {
        Vector { y: self.x, x: self.y }
    }

    /// Returns `(x, y)`.
    pub fn as_tuple(self) -> (Length, Length) {
        (self.x, self.y)
    }

    /// Returns `[x, y]`.
    pub fn as_array(self) -> [Length; 2] {
        [self.x, self.y]
    }

    /// Returns a vector that computes the absolute layout vector of `self`.
    pub fn abs(&self) -> Vector {
        Vector {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }

    /// Returns `true` if all values are [`Length::Default`].
    pub fn is_default(&self) -> bool {
        self.x.is_default() && self.y.is_default()
    }

    /// Returns `true` if any value is [`Length::Default`].
    pub fn has_default(&self) -> bool {
        self.x.has_default() && self.y.has_default()
    }

    /// Replaces [`Length::Default`] values with `overwrite` values.
    pub fn replace_default(&mut self, overwrite: &Vector) {
        self.x.replace_default(&overwrite.x);
        self.y.replace_default(&overwrite.y);
    }

    /// Cast to [`Point`].
    pub fn as_point(self) -> Point {
        Point { x: self.x, y: self.y }
    }

    /// Cast to [`Size`].
    pub fn as_size(self) -> Size {
        Size {
            width: self.x,
            height: self.y,
        }
    }

    /// Create a translate transform from `self`.
    pub fn into_transform(self) -> Transform {
        Transform::new_translate(self.x, self.y)
    }
}
impl super::Layout2d for Vector {
    type Px = PxVector;

    fn layout_dft(&self, default: Self::Px) -> Self::Px {
        PxVector::new(self.x.layout_dft_x(default.x), self.y.layout_dft_y(default.y))
    }

    fn affect_mask(&self) -> LayoutMask {
        self.x.affect_mask() | self.y.affect_mask()
    }
}
impl_length_comp_conversions! {
    fn from(x: X, y: Y) -> Vector {
        Vector::new(x, y)
    }
}
impl_from_and_into_var! {
    fn from(p: PxVector) -> Vector {
        Vector::new(p.x, p.y)
    }
    fn from(p: DipVector) -> Vector {
        Vector::new(p.x, p.y)
    }
    fn from(p: Point) -> Vector {
        p.as_vector()
    }
    fn from(s: Size) -> Vector {
        s.as_vector()
    }

    /// Use the length for x and y.
    fn from(length: Length) -> Vector {
        Vector::splat(length)
    }

    /// Conversion to [`Length::Factor`] then to vector.
    fn from(percent: FactorPercent) -> Vector {
        Length::from(percent).into()
    }

    /// Conversion to [`Length::Factor`] then to vector.
    fn from(norm: Factor) -> Vector {
        Length::from(norm).into()
    }

    /// Conversion to [`Length::Dip`] then to vector.
    fn from(f: f32) -> Vector {
        Length::from(f).into()
    }

    /// Conversion to [`Length::Dip`] then to vector.
    fn from(i: i32) -> Vector {
        Length::from(i).into()
    }

    /// Conversion to [`Length::Px`] then to vector.
    fn from(l: Px) -> Vector {
        Length::from(l).into()
    }

    /// Conversion to [`Length::Dip`] then to vector.
    fn from(l: Dip) -> Vector {
        Length::from(l).into()
    }
}
impl ops::Add for Vector {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self {
        self += rhs;
        self
    }
}
impl<'a> ops::Add<&'a Vector> for &Vector {
    type Output = Vector;

    fn add(self, rhs: &'a Vector) -> Self::Output {
        self.clone() + rhs.clone()
    }
}
impl ops::AddAssign for Vector {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}
impl ops::Sub for Vector {
    type Output = Self;

    fn sub(mut self, rhs: Self) -> Self {
        self -= rhs;
        self
    }
}
impl<'a> ops::Sub<&'a Vector> for &Vector {
    type Output = Vector;

    fn sub(self, rhs: &'a Vector) -> Self::Output {
        self.clone() - rhs.clone()
    }
}
impl ops::SubAssign for Vector {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}
impl<S: Into<Factor2d>> ops::Mul<S> for Vector {
    type Output = Self;

    fn mul(mut self, rhs: S) -> Self {
        self *= rhs;
        self
    }
}
impl<S: Into<Factor2d>> ops::Mul<S> for &Vector {
    type Output = Vector;

    fn mul(self, rhs: S) -> Self::Output {
        self.clone() * rhs
    }
}
impl<S: Into<Factor2d>> ops::MulAssign<S> for Vector {
    fn mul_assign(&mut self, rhs: S) {
        let rhs = rhs.into();

        self.x *= rhs.x;
        self.y *= rhs.y;
    }
}
impl<S: Into<Factor2d>> ops::Div<S> for Vector {
    type Output = Self;

    fn div(mut self, rhs: S) -> Self {
        self /= rhs;
        self
    }
}
impl<S: Into<Factor2d>> ops::Div<S> for &Vector {
    type Output = Vector;

    fn div(self, rhs: S) -> Self::Output {
        self.clone() / rhs
    }
}
impl<S: Into<Factor2d>> ops::DivAssign<S> for Vector {
    fn div_assign(&mut self, rhs: S) {
        let rhs = rhs.into();
        self.x /= rhs.x;
        self.y /= rhs.y;
    }
}
impl ops::Neg for Vector {
    type Output = Self;

    fn neg(self) -> Self {
        Vector { x: -self.x, y: -self.y }
    }
}
impl ops::Neg for &Vector {
    type Output = Vector;

    fn neg(self) -> Self::Output {
        -self.clone()
    }
}
