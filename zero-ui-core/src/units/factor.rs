use derive_more as dm;

use super::{about_eq, Px, PxPoint, PxRect, PxSideOffsets, PxSize, PxVector, Size, EPSILON, EPSILON_100, about_eq_hash};
use crate::impl_from_and_into_var;
use std::{fmt, ops, time::Duration};

/// Multiplication factor in percentage (0%-100%).
///
/// See [`FactorUnits`] for more details.
///
/// # Equality
///
/// Equality is determined using [`about_eq`] with `0.001` epsilon.
#[derive(Copy, Clone, dm::Add, dm::AddAssign, dm::Sub, dm::SubAssign)]
pub struct FactorPercent(pub f32);
impl FactorPercent {
    /// Clamp factor to [0.0..=100.0] range.
    #[inline]
    pub fn clamp_range(self) -> Self {
        FactorPercent(self.0.max(0.0).min(100.0))
    }

    /// Convert to [`Factor`].
    #[inline]
    pub fn as_normal(self) -> Factor {
        self.into()
    }
}
impl ops::Neg for FactorPercent {
    type Output = Self;

    fn neg(self) -> Self::Output {
        FactorPercent(-self.0)
    }
}
impl PartialEq for FactorPercent {
    fn eq(&self, other: &Self) -> bool {
        about_eq(self.0, other.0, EPSILON_100)
    }
}
impl_from_and_into_var! {
    fn from(n: Factor) -> FactorPercent {
        FactorPercent(n.0 * 100.0)
    }
}
impl fmt::Debug for FactorPercent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_tuple("FactorPercent").field(&self.0).finish()
        } else {
            write!(f, "{}.pct()", self.0)
        }
    }
}
impl fmt::Display for FactorPercent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}%", self.0)
    }
}

/// Normalized multiplication factor.
///
/// Values of this type are normalized to generally be in between `0.0` and `1.0` to indicate a fraction
/// of a unit. However, values are not clamped to this range, `Factor(2.0)` is a valid value and so are
/// negative values.
///
/// You can use the *suffix method* `1.0.fct()` to init a factor, see [`FactorUnits`] for more details.
///
/// # Equality
///
/// Equality is determined using [`about_eq`] with `0.00001` epsilon.
#[derive(Copy, Clone, dm::Add, dm::AddAssign, dm::Sub, dm::SubAssign, PartialOrd)]
pub struct Factor(pub f32);
impl Factor {
    /// Clamp factor to `[0.0..=1.0]` range.
    #[inline]
    pub fn clamp_range(self) -> Self {
        Factor(self.0.max(0.0).min(1.0))
    }

    /// Returns the maximum of two factors.
    #[inline]
    pub fn max(self, other: impl Into<Factor>) -> Factor {
        Factor(self.0.max(other.into().0))
    }

    /// Returns the minimum of two factors.
    #[inline]
    pub fn min(self, other: impl Into<Factor>) -> Factor {
        Factor(self.0.min(other.into().0))
    }

    /// Returns `self` if `min <= self <= max`, returns `min` if `self < min` or returns `max` if `self > max`.
    #[inline]
    pub fn clamp(self, min: impl Into<Factor>, max: impl Into<Factor>) -> Factor {
        self.min(max).max(min)
    }

    /// Computes the absolute value of self.
    #[inline]
    pub fn abs(self) -> Factor {
        Factor(self.0.abs())
    }

    /// Convert to [`FactorPercent`].
    #[inline]
    pub fn as_percent(self) -> FactorPercent {
        self.into()
    }

    /// Returns `1.fct() - self`.
    #[inline]
    pub fn flip(self) -> Factor {
        1.fct() - self
    }
}
impl std::hash::Hash for Factor {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        about_eq_hash(self.0, EPSILON, state)
    }
}
impl PartialEq for Factor {
    fn eq(&self, other: &Self) -> bool {
        about_eq(self.0, other.0, EPSILON)
    }
}
impl ops::Mul for Factor {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Factor(self.0 * rhs.0)
    }
}
impl ops::MulAssign for Factor {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}
impl ops::Div for Factor {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Factor(self.0 / rhs.0)
    }
}
impl ops::DivAssign for Factor {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}
impl ops::Mul<Factor> for Px {
    type Output = Px;

    fn mul(self, rhs: Factor) -> Px {
        self * rhs.0
    }
}
impl ops::Div<Factor> for Px {
    type Output = Px;

    fn div(self, rhs: Factor) -> Px {
        self / rhs.0
    }
}
impl ops::MulAssign<Factor> for Px {
    fn mul_assign(&mut self, rhs: Factor) {
        *self = *self * rhs;
    }
}
impl ops::DivAssign<Factor> for Px {
    fn div_assign(&mut self, rhs: Factor) {
        *self = *self / rhs;
    }
}
impl ops::Mul<Factor> for PxPoint {
    type Output = PxPoint;

    fn mul(self, rhs: Factor) -> PxPoint {
        self * Factor2d::uniform(rhs)
    }
}
impl ops::Div<Factor> for PxPoint {
    type Output = PxPoint;

    fn div(self, rhs: Factor) -> PxPoint {
        self / Factor2d::uniform(rhs)
    }
}
impl ops::MulAssign<Factor> for PxPoint {
    fn mul_assign(&mut self, rhs: Factor) {
        *self = *self * rhs;
    }
}
impl ops::DivAssign<Factor> for PxPoint {
    fn div_assign(&mut self, rhs: Factor) {
        *self = *self / rhs;
    }
}
impl ops::Mul<Factor> for PxVector {
    type Output = PxVector;

    fn mul(self, rhs: Factor) -> PxVector {
        self * Factor2d::uniform(rhs)
    }
}
impl ops::Div<Factor> for PxVector {
    type Output = PxVector;

    fn div(self, rhs: Factor) -> PxVector {
        self / Factor2d::uniform(rhs)
    }
}
impl ops::MulAssign<Factor> for PxVector {
    fn mul_assign(&mut self, rhs: Factor) {
        *self = *self * rhs;
    }
}
impl ops::DivAssign<Factor> for PxVector {
    fn div_assign(&mut self, rhs: Factor) {
        *self = *self / rhs;
    }
}
impl ops::Mul<Factor> for PxSize {
    type Output = PxSize;

    fn mul(self, rhs: Factor) -> PxSize {
        self * Factor2d::uniform(rhs)
    }
}
impl ops::Div<Factor> for PxSize {
    type Output = PxSize;

    fn div(self, rhs: Factor) -> PxSize {
        self / Factor2d::uniform(rhs)
    }
}
impl ops::MulAssign<Factor> for PxSize {
    fn mul_assign(&mut self, rhs: Factor) {
        *self = *self * rhs;
    }
}
impl ops::DivAssign<Factor> for PxSize {
    fn div_assign(&mut self, rhs: Factor) {
        *self = *self / rhs;
    }
}
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
impl ops::Mul<Factor> for PxRect {
    type Output = PxRect;

    fn mul(self, rhs: Factor) -> PxRect {
        self * Factor2d::uniform(rhs)
    }
}
impl ops::Div<Factor> for PxRect {
    type Output = PxRect;

    fn div(self, rhs: Factor) -> PxRect {
        self / Factor2d::uniform(rhs)
    }
}
impl ops::MulAssign<Factor> for PxRect {
    fn mul_assign(&mut self, rhs: Factor) {
        *self = *self * rhs;
    }
}
impl ops::DivAssign<Factor> for PxRect {
    fn div_assign(&mut self, rhs: Factor) {
        *self = *self / rhs;
    }
}
impl ops::Neg for Factor {
    type Output = Factor;

    fn neg(self) -> Self::Output {
        Factor(-self.0)
    }
}

impl_from_and_into_var! {
    fn from(percent: FactorPercent) -> Factor {
        Factor(percent.0 / 100.0)
    }

    fn from(f: f32) -> Factor {
        Factor(f)
    }

    fn from(f: f64) -> Factor {
        Factor(f as f32)
    }

    /// | Input  | Output  |
    /// |--------|---------|
    /// |`true`  | `1.0`   |
    /// |`false` | `0.0`   |
    fn from(b: bool) -> Factor {
        Factor(if b { 1.0 } else { 0.0 })
    }
}
impl fmt::Debug for Factor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_tuple("Factor").field(&self.0).finish()
        } else {
            write!(f, "{}.fct()", self.0)
        }
    }
}
impl fmt::Display for Factor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// Extension methods for initializing factor units.
///
/// This trait is implemented for [`f32`] and [`u32`] allowing initialization of factor unit types using the `<number>.<unit>()` syntax.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::units::*;
/// let percent = 100.pct();
/// ```
pub trait FactorUnits {
    /// Percent factor.
    fn pct(self) -> FactorPercent;

    /// Normalized factor.
    ///
    /// # Note
    ///
    /// [`Factor`] implements `From<f32>`.
    fn fct(self) -> Factor;
}
impl FactorUnits for f32 {
    #[inline]
    fn pct(self) -> FactorPercent {
        FactorPercent(self)
    }

    #[inline]
    fn fct(self) -> Factor {
        self.into()
    }
}
impl FactorUnits for i32 {
    #[inline]
    fn pct(self) -> FactorPercent {
        FactorPercent(self as f32)
    }

    #[inline]
    fn fct(self) -> Factor {
        Factor(self as f32)
    }
}

/// Scale factor applied to ***x*** and ***y*** dimensions.
#[derive(Clone, Copy, Debug)]
pub struct Factor2d {
    /// Scale factor applied in the ***x*** dimension.
    pub x: Factor,
    /// Scale factor applied in the ***y*** dimension.
    pub y: Factor,
}
impl_from_and_into_var! {
    fn from<X: Into<Factor> + Clone, Y: Into<Factor> + Clone>((x, y): (X, Y)) -> Factor2d {
        Factor2d { x: x.into(), y: y.into() }
    }

    fn from(xy: Factor) -> Factor2d {
        Factor2d { x: xy, y: xy }
    }

    fn from(xy: FactorPercent) -> Factor2d {
        xy.as_normal().into()
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
}
impl fmt::Display for Factor2d {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_uniform() {
            write!(f, "{}", self.x.as_percent())
        } else {
            write!(f, "({}, {})", self.x.as_percent(), self.y.as_percent())
        }
    }
}
impl ops::Mul<Factor2d> for PxSize {
    type Output = PxSize;

    fn mul(self, rhs: Factor2d) -> PxSize {
        PxSize::new(self.width * rhs.x, self.height * rhs.y)
    }
}
impl ops::Div<Factor2d> for PxSize {
    type Output = PxSize;

    fn div(self, rhs: Factor2d) -> PxSize {
        PxSize::new(self.width / rhs.x, self.height / rhs.y)
    }
}
impl ops::MulAssign<Factor2d> for PxSize {
    fn mul_assign(&mut self, rhs: Factor2d) {
        *self = *self * rhs;
    }
}
impl ops::DivAssign<Factor2d> for PxSize {
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

/// Scale factor applied to margins.
#[derive(Clone, Copy, Debug)]
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
    pub fn new_dimension(top_bottom: impl Into<Factor>, left_right: impl Into<Factor>) -> Self {
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
        *self = *self * rhs;
    }
}
impl ops::DivAssign<FactorSideOffsets> for FactorSideOffsets {
    fn div_assign(&mut self, rhs: FactorSideOffsets) {
        *self = *self / rhs;
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
    fn from<
        TB: Into<Factor> + Clone,
        LR: Into<Factor> + Clone
        >(
            (top_bottom, left_right): (TB, LR)
        )
        -> FactorSideOffsets {
        FactorSideOffsets::new_dimension(top_bottom, left_right)
    }

    /// New top, right, bottom, left.
    fn from<
        T: Into<Factor> + Clone,
        R: Into<Factor> + Clone,
        B: Into<Factor> + Clone,
        L: Into<Factor> + Clone
        >(
            (top, right, bottom, left): (T, R, B, L)
        )
        -> FactorSideOffsets {
        FactorSideOffsets::new(top, right, bottom, left)
    }
}

/// Easing function output.
///
/// Usually in the [0..=1] range, but can overshoot. An easing function converts a [`EasingTime`]
/// into this factor.
///
/// # Examples
///
/// ```
/// use zero_ui_core::units::*;
///
/// /// Cubic animation curve.
/// fn cubic(time: EasingTime) -> EasingStep {
///     let f = time.fct();
///     f * f * f
/// }
/// ```
///
/// Note that all the common easing functions are implemented in [`var::easing`].
///
/// [`var::easing`]: crate::var::easing
pub type EasingStep = Factor;

/// Easing function input.
///
/// Is always in the [0..=1] range. An easing function converts this time into a [`EasingStep`] factor.
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct EasingTime(Factor);
impl_from_and_into_var! {
    fn from(factor: Factor) -> EasingTime {
        EasingTime::new(factor)
    }
}
impl EasingTime {
    /// New from [`Factor`].
    ///
    /// The `factor` is clamped to the [0..=1] range.
    #[inline]
    pub fn new(factor: Factor) -> Self {
        EasingTime(factor.clamp_range())
    }

    /// New easing time from total `duration` and `elapsed` time.
    ///
    /// If `elapsed >= duration` the time is 1.
    #[inline]
    pub fn elapsed(duration: Duration, elapsed: Duration) -> Self {
        if elapsed < duration {
            EasingTime(elapsed.as_secs_f32().fct() / duration.as_secs_f32().fct())
        } else {
            EasingTime(1.fct())
        }
    }

    /// Gets the start time, zero.
    #[inline]
    pub fn start() -> Self {
        EasingTime(0.fct())
    }

    /// Gets the end time, one.
    #[inline]
    pub fn end() -> Self {
        EasingTime(1.fct())
    }

    /// If the time represents the start of the animation.
    #[inline]
    pub fn is_start(self) -> bool {
        self == Self::start()
    }

    /// If the time represents the end of the animation.
    #[inline]
    pub fn is_end(self) -> bool {
        self == Self::end()
    }

    /// Get the time as a [`Factor`].
    #[inline]
    pub fn fct(self) -> Factor {
        self.0
    }

    /// Flip the time.
    ///
    /// Returns `1 - self`.
    #[inline]
    pub fn reverse(self) -> Self {
        EasingTime(self.0.flip())
    }
}
