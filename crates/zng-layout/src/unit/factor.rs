use zng_unit::FactorUnits;

use super::{
    DipPoint, DipRect, DipSideOffsets, DipSize, DipVector, Factor, FactorPercent, PxPoint, PxRect, PxSideOffsets, PxSize, PxVector, Size,
};
use std::{fmt, ops};
use zng_var::impl_from_and_into_var;

impl ops::Mul<Factor> for Factor2d {
    type Output = Factor2d;

    fn mul(self, rhs: Factor) -> Factor2d {
        Factor2d::new(self.x * rhs, self.y * rhs)
    }
}
impl ops::Div<Factor> for Factor2d {
    type Output = Factor2d;

    fn div(self, rhs: Factor) -> Factor2d {
        Factor2d::new(self.x / rhs, self.y / rhs)
    }
}
impl ops::MulAssign<Factor> for Factor2d {
    fn mul_assign(&mut self, rhs: Factor) {
        *self = *self * rhs;
    }
}
impl ops::DivAssign<Factor> for Factor2d {
    fn div_assign(&mut self, rhs: Factor) {
        *self = *self / rhs;
    }
}

/// Scale factor applied to ***x*** and ***y*** dimensions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Factor2d {
    /// Scale factor applied in the ***x*** dimension.
    pub x: Factor,
    /// Scale factor applied in the ***y*** dimension.
    pub y: Factor,
}
impl_from_and_into_var! {
    fn from<X: Into<Factor>, Y: Into<Factor>>((x, y): (X, Y)) -> Factor2d {
        Factor2d { x: x.into(), y: y.into() }
    }

    fn from(xy: Factor) -> Factor2d {
        Factor2d { x: xy, y: xy }
    }

    fn from(xy: FactorPercent) -> Factor2d {
        xy.fct().into()
    }

    /// To relative width and height.
    fn from(scale: Factor2d) -> Size {
        Size {
            width: scale.x.into(),
            height: scale.y.into(),
        }
    }
}
impl Factor2d {
    /// New scale with different scales for each dimension.
    pub fn new(x: impl Into<Factor>, y: impl Into<Factor>) -> Self {
        Factor2d { x: x.into(), y: y.into() }
    }

    /// Uniform scale applied to both ***x*** and ***y***.
    pub fn uniform(xy: impl Into<Factor>) -> Self {
        let xy = xy.into();
        xy.into()
    }

    /// No scaling.
    pub fn identity() -> Self {
        Self::uniform(1.0)
    }

    /// If the scale is the same for both ***x*** and ***y***.
    pub fn is_uniform(self) -> bool {
        self.x == self.y
    }

    /// Computes the absolute value of x and y.
    pub fn abs(mut self) -> Self {
        self.x = self.x.abs();
        self.y = self.y.abs();
        self
    }

    /// Swap x and y.
    pub fn yx(self) -> Self {
        Self::new(self.y, self.x)
    }
}
impl fmt::Display for Factor2d {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_uniform() {
            write!(f, "{}", FactorPercent::from(self.x))
        } else {
            write!(f, "({}, {})", FactorPercent::from(self.x), FactorPercent::from(self.y))
        }
    }
}
impl ops::Mul<Factor2d> for PxSize {
    type Output = PxSize;

    fn mul(self, rhs: Factor2d) -> PxSize {
        PxSize::new(self.width * rhs.x, self.height * rhs.y)
    }
}
impl ops::Mul<Factor2d> for DipSize {
    type Output = DipSize;

    fn mul(self, rhs: Factor2d) -> DipSize {
        DipSize::new(self.width * rhs.x, self.height * rhs.y)
    }
}
impl ops::Div<Factor2d> for PxSize {
    type Output = PxSize;

    fn div(self, rhs: Factor2d) -> PxSize {
        PxSize::new(self.width / rhs.x, self.height / rhs.y)
    }
}
impl ops::Div<Factor2d> for DipSize {
    type Output = DipSize;

    fn div(self, rhs: Factor2d) -> DipSize {
        DipSize::new(self.width / rhs.x, self.height / rhs.y)
    }
}
impl ops::MulAssign<Factor2d> for PxSize {
    fn mul_assign(&mut self, rhs: Factor2d) {
        *self = *self * rhs;
    }
}
impl ops::MulAssign<Factor2d> for DipSize {
    fn mul_assign(&mut self, rhs: Factor2d) {
        *self = *self * rhs;
    }
}
impl ops::DivAssign<Factor2d> for PxSize {
    fn div_assign(&mut self, rhs: Factor2d) {
        *self = *self / rhs;
    }
}
impl ops::DivAssign<Factor2d> for DipSize {
    fn div_assign(&mut self, rhs: Factor2d) {
        *self = *self / rhs;
    }
}
impl ops::Mul<Factor2d> for PxPoint {
    type Output = PxPoint;

    fn mul(self, rhs: Factor2d) -> PxPoint {
        PxPoint::new(self.x * rhs.x, self.y * rhs.y)
    }
}
impl ops::Div<Factor2d> for PxPoint {
    type Output = PxPoint;

    fn div(self, rhs: Factor2d) -> PxPoint {
        PxPoint::new(self.x / rhs.x, self.y / rhs.y)
    }
}
impl ops::MulAssign<Factor2d> for PxPoint {
    fn mul_assign(&mut self, rhs: Factor2d) {
        *self = *self * rhs;
    }
}
impl ops::DivAssign<Factor2d> for PxPoint {
    fn div_assign(&mut self, rhs: Factor2d) {
        *self = *self / rhs;
    }
}

impl ops::Mul<Factor2d> for DipPoint {
    type Output = DipPoint;

    fn mul(self, rhs: Factor2d) -> DipPoint {
        DipPoint::new(self.x * rhs.x, self.y * rhs.y)
    }
}
impl ops::Div<Factor2d> for DipPoint {
    type Output = DipPoint;

    fn div(self, rhs: Factor2d) -> DipPoint {
        DipPoint::new(self.x / rhs.x, self.y / rhs.y)
    }
}
impl ops::MulAssign<Factor2d> for DipPoint {
    fn mul_assign(&mut self, rhs: Factor2d) {
        *self = *self * rhs;
    }
}
impl ops::DivAssign<Factor2d> for DipPoint {
    fn div_assign(&mut self, rhs: Factor2d) {
        *self = *self / rhs;
    }
}

impl ops::Mul<Factor2d> for PxVector {
    type Output = PxVector;

    fn mul(self, rhs: Factor2d) -> PxVector {
        PxVector::new(self.x * rhs.x, self.y * rhs.y)
    }
}
impl ops::Div<Factor2d> for PxVector {
    type Output = PxVector;

    fn div(self, rhs: Factor2d) -> PxVector {
        PxVector::new(self.x / rhs.x, self.y / rhs.y)
    }
}
impl ops::MulAssign<Factor2d> for PxVector {
    fn mul_assign(&mut self, rhs: Factor2d) {
        *self = *self * rhs;
    }
}
impl ops::DivAssign<Factor2d> for PxVector {
    fn div_assign(&mut self, rhs: Factor2d) {
        *self = *self / rhs;
    }
}

impl ops::Mul<Factor2d> for DipVector {
    type Output = DipVector;

    fn mul(self, rhs: Factor2d) -> DipVector {
        DipVector::new(self.x * rhs.x, self.y * rhs.y)
    }
}
impl ops::Div<Factor2d> for DipVector {
    type Output = DipVector;

    fn div(self, rhs: Factor2d) -> DipVector {
        DipVector::new(self.x / rhs.x, self.y / rhs.y)
    }
}
impl ops::MulAssign<Factor2d> for DipVector {
    fn mul_assign(&mut self, rhs: Factor2d) {
        *self = *self * rhs;
    }
}
impl ops::DivAssign<Factor2d> for DipVector {
    fn div_assign(&mut self, rhs: Factor2d) {
        *self = *self / rhs;
    }
}

impl ops::Mul<Factor2d> for Factor2d {
    type Output = Factor2d;

    fn mul(self, rhs: Factor2d) -> Factor2d {
        Factor2d::new(self.x * rhs.x, self.y * rhs.y)
    }
}
impl ops::Div<Factor2d> for Factor2d {
    type Output = Factor2d;

    fn div(self, rhs: Factor2d) -> Factor2d {
        Factor2d::new(self.x / rhs.x, self.y / rhs.y)
    }
}
impl ops::MulAssign<Factor2d> for Factor2d {
    fn mul_assign(&mut self, rhs: Factor2d) {
        *self = *self * rhs;
    }
}
impl ops::DivAssign<Factor2d> for Factor2d {
    fn div_assign(&mut self, rhs: Factor2d) {
        *self = *self / rhs;
    }
}
impl ops::Mul<Factor2d> for PxRect {
    type Output = PxRect;

    fn mul(self, rhs: Factor2d) -> PxRect {
        PxRect::new(self.origin * rhs, self.size * rhs)
    }
}
impl ops::Div<Factor2d> for PxRect {
    type Output = PxRect;

    fn div(self, rhs: Factor2d) -> PxRect {
        PxRect::new(self.origin / rhs, self.size / rhs)
    }
}
impl ops::MulAssign<Factor2d> for PxRect {
    fn mul_assign(&mut self, rhs: Factor2d) {
        *self = *self * rhs;
    }
}
impl ops::DivAssign<Factor2d> for PxRect {
    fn div_assign(&mut self, rhs: Factor2d) {
        *self = *self / rhs;
    }
}

impl ops::Mul<Factor2d> for DipRect {
    type Output = DipRect;

    fn mul(self, rhs: Factor2d) -> DipRect {
        DipRect::new(self.origin * rhs, self.size * rhs)
    }
}
impl ops::Div<Factor2d> for DipRect {
    type Output = DipRect;

    fn div(self, rhs: Factor2d) -> DipRect {
        DipRect::new(self.origin / rhs, self.size / rhs)
    }
}
impl ops::MulAssign<Factor2d> for DipRect {
    fn mul_assign(&mut self, rhs: Factor2d) {
        *self = *self * rhs;
    }
}
impl ops::DivAssign<Factor2d> for DipRect {
    fn div_assign(&mut self, rhs: Factor2d) {
        *self = *self / rhs;
    }
}

impl ops::Neg for Factor2d {
    type Output = Self;

    fn neg(mut self) -> Self::Output {
        self.x = -self.x;
        self.y = -self.y;
        self
    }
}

/// Scale factor applied to margins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FactorSideOffsets {
    /// Factor of top offset.
    pub top: Factor,
    /// Factor of right offset.
    pub right: Factor,
    /// Factor of bottom offset.
    pub bottom: Factor,
    /// Factor of left offset.
    pub left: Factor,
}
impl FactorSideOffsets {
    /// Factors applied to each offset.
    pub fn new(top: impl Into<Factor>, right: impl Into<Factor>, bottom: impl Into<Factor>, left: impl Into<Factor>) -> Self {
        Self {
            top: top.into(),
            right: right.into(),
            bottom: bottom.into(),
            left: left.into(),
        }
    }

    /// Same scale applied to parallel offsets.
    pub fn new_vh(top_bottom: impl Into<Factor>, left_right: impl Into<Factor>) -> Self {
        let tb = top_bottom.into();
        let lr = left_right.into();

        Self::new(tb, lr, tb, lr)
    }

    /// Uniform scale applied to all offsets.
    pub fn new_all(uniform: impl Into<Factor>) -> Self {
        let u = uniform.into();
        Self::new(u, u, u, u)
    }

    /// Uniform 0%.
    pub fn zero() -> Self {
        Self::new_all(0.fct())
    }

    /// Uniform 100%.
    pub fn one() -> Self {
        Self::new_all(1.fct())
    }
}
impl ops::Mul<FactorSideOffsets> for FactorSideOffsets {
    type Output = FactorSideOffsets;

    fn mul(self, rhs: FactorSideOffsets) -> FactorSideOffsets {
        FactorSideOffsets::new(
            self.top * rhs.top,
            self.right * rhs.right,
            self.bottom * rhs.bottom,
            self.left * rhs.left,
        )
    }
}
impl ops::Div<FactorSideOffsets> for FactorSideOffsets {
    type Output = FactorSideOffsets;

    fn div(self, rhs: FactorSideOffsets) -> FactorSideOffsets {
        FactorSideOffsets::new(
            self.top / rhs.top,
            self.right / rhs.right,
            self.bottom / rhs.bottom,
            self.left / rhs.left,
        )
    }
}
impl ops::MulAssign<FactorSideOffsets> for FactorSideOffsets {
    fn mul_assign(&mut self, rhs: FactorSideOffsets) {
        self.top *= rhs.top;
        self.right *= rhs.right;
        self.bottom *= rhs.bottom;
        self.left *= rhs.left;
    }
}
impl ops::DivAssign<FactorSideOffsets> for FactorSideOffsets {
    fn div_assign(&mut self, rhs: FactorSideOffsets) {
        self.top /= rhs.top;
        self.right /= rhs.right;
        self.bottom /= rhs.bottom;
        self.left /= rhs.left;
    }
}
impl ops::Mul<FactorSideOffsets> for PxSideOffsets {
    type Output = PxSideOffsets;

    fn mul(self, rhs: FactorSideOffsets) -> PxSideOffsets {
        PxSideOffsets::new(
            self.top * rhs.top,
            self.right * rhs.right,
            self.bottom * rhs.bottom,
            self.left * rhs.left,
        )
    }
}
impl ops::Div<FactorSideOffsets> for PxSideOffsets {
    type Output = PxSideOffsets;

    fn div(self, rhs: FactorSideOffsets) -> PxSideOffsets {
        PxSideOffsets::new(
            self.top / rhs.top,
            self.right / rhs.right,
            self.bottom / rhs.bottom,
            self.left / rhs.left,
        )
    }
}
impl ops::MulAssign<FactorSideOffsets> for PxSideOffsets {
    fn mul_assign(&mut self, rhs: FactorSideOffsets) {
        *self = *self * rhs;
    }
}
impl ops::DivAssign<FactorSideOffsets> for PxSideOffsets {
    fn div_assign(&mut self, rhs: FactorSideOffsets) {
        *self = *self / rhs;
    }
}

impl ops::Mul<FactorSideOffsets> for DipSideOffsets {
    type Output = DipSideOffsets;

    fn mul(self, rhs: FactorSideOffsets) -> DipSideOffsets {
        DipSideOffsets::new(
            self.top * rhs.top,
            self.right * rhs.right,
            self.bottom * rhs.bottom,
            self.left * rhs.left,
        )
    }
}
impl ops::Div<FactorSideOffsets> for DipSideOffsets {
    type Output = DipSideOffsets;

    fn div(self, rhs: FactorSideOffsets) -> DipSideOffsets {
        DipSideOffsets::new(
            self.top / rhs.top,
            self.right / rhs.right,
            self.bottom / rhs.bottom,
            self.left / rhs.left,
        )
    }
}
impl ops::MulAssign<FactorSideOffsets> for DipSideOffsets {
    fn mul_assign(&mut self, rhs: FactorSideOffsets) {
        *self = *self * rhs;
    }
}
impl ops::DivAssign<FactorSideOffsets> for DipSideOffsets {
    fn div_assign(&mut self, rhs: FactorSideOffsets) {
        *self = *self / rhs;
    }
}

impl_from_and_into_var! {
    /// All sides equal.
    fn from(all: Factor) -> FactorSideOffsets {
        FactorSideOffsets::new_all(all)
    }

    /// All sides equal.
    fn from(percent: FactorPercent) -> FactorSideOffsets {
        FactorSideOffsets::new_all(percent)
    }

    /// New dimension, top-bottom, left-right.
    fn from<TB: Into<Factor>, LR: Into<Factor>>((top_bottom, left_right): (TB, LR)) -> FactorSideOffsets {
        FactorSideOffsets::new_vh(top_bottom, left_right)
    }

    /// New top, right, bottom, left.
    fn from<T: Into<Factor>, R: Into<Factor>, B: Into<Factor>, L: Into<Factor>>(
        (top, right, bottom, left): (T, R, B, L),
    ) -> FactorSideOffsets {
        FactorSideOffsets::new(top, right, bottom, left)
    }
}
