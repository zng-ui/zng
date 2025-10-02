use std::{fmt, ops, time::Duration};

use crate::{
    Dip, DipPoint, DipRect, DipSize, DipVector, EQ_GRANULARITY, EQ_GRANULARITY_100, Px, PxPoint, PxRect, PxSize, PxVector, about_eq,
    about_eq_hash, about_eq_ord,
};

/// Extension methods for initializing factor units.
///
/// This trait is implemented for [`f32`] and [`u32`] allowing initialization of factor unit types using the `<number>.<unit>()` syntax.
///
/// # Examples
///
/// ```
/// # use zng_unit::*;
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
    fn pct(self) -> FactorPercent {
        FactorPercent(self)
    }

    fn fct(self) -> Factor {
        self.into()
    }
}
impl FactorUnits for i32 {
    fn pct(self) -> FactorPercent {
        FactorPercent(self as f32)
    }

    fn fct(self) -> Factor {
        Factor(self as f32)
    }
}

/// Normalized multiplication factor.
///
/// Values of this type are normalized to generally be in between `0.0` and `1.0` to indicate a fraction
/// of a unit. However, values are not clamped to this range, `Factor(2.0)` is a valid value and so are
/// negative values.
///
/// # Equality
///
/// Equality is determined to within `0.00001` epsilon.
#[derive(Copy, Clone, serde::Serialize, serde::Deserialize, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Factor(pub f32);
impl Factor {
    /// Clamp factor to `[0.0..=1.0]` range.
    pub fn clamp_range(self) -> Self {
        Factor(self.0.clamp(0.0, 1.0))
    }

    /// Computes the absolute value of self.
    pub fn abs(self) -> Factor {
        Factor(self.0.abs())
    }

    /// Flip factor, around `0.5`,
    ///
    /// Returns `1.0 - self`.
    pub fn flip(self) -> Factor {
        Self(1.0) - self
    }

    /// Factor as percentage.
    pub fn pct(self) -> FactorPercent {
        self.into()
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
        write!(f, "{}", self.0)
    }
}
impl From<f32> for Factor {
    fn from(value: f32) -> Self {
        Factor(value)
    }
}
impl ops::Add for Factor {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl ops::AddAssign for Factor {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}
impl ops::Sub for Factor {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}
impl ops::SubAssign for Factor {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}
impl std::hash::Hash for Factor {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        about_eq_hash(self.0, EQ_GRANULARITY, state)
    }
}
impl PartialEq for Factor {
    fn eq(&self, other: &Self) -> bool {
        about_eq(self.0, other.0, EQ_GRANULARITY)
    }
}
impl Eq for Factor {}
impl std::cmp::PartialOrd for Factor {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl std::cmp::Ord for Factor {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        about_eq_ord(self.0, other.0, EQ_GRANULARITY)
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
/// Parses `"##"` and `"##.fct()"` where `##` is a `f32`.
impl std::str::FromStr for Factor {
    type Err = std::num::ParseFloatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        crate::parse_suffix(s, &[".fct()"]).map(Factor)
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

impl ops::Mul<Factor> for Dip {
    type Output = Dip;

    fn mul(self, rhs: Factor) -> Dip {
        self * rhs.0
    }
}
impl ops::Div<Factor> for Dip {
    type Output = Dip;

    fn div(self, rhs: Factor) -> Dip {
        self / rhs.0
    }
}
impl ops::MulAssign<Factor> for Dip {
    fn mul_assign(&mut self, rhs: Factor) {
        *self = *self * rhs;
    }
}
impl ops::DivAssign<Factor> for Dip {
    fn div_assign(&mut self, rhs: Factor) {
        *self = *self / rhs;
    }
}

impl ops::Mul<Factor> for PxPoint {
    type Output = PxPoint;

    fn mul(mut self, rhs: Factor) -> PxPoint {
        self.x *= rhs;
        self.y *= rhs;
        self
    }
}
impl ops::Div<Factor> for PxPoint {
    type Output = PxPoint;

    fn div(mut self, rhs: Factor) -> PxPoint {
        self.x /= rhs;
        self.y /= rhs;
        self
    }
}
impl ops::MulAssign<Factor> for PxPoint {
    fn mul_assign(&mut self, rhs: Factor) {
        self.x *= rhs;
        self.y *= rhs;
    }
}
impl ops::DivAssign<Factor> for PxPoint {
    fn div_assign(&mut self, rhs: Factor) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl ops::Mul<Factor> for euclid::Point2D<f32, Px> {
    type Output = euclid::Point2D<f32, Px>;

    fn mul(mut self, rhs: Factor) -> euclid::Point2D<f32, Px> {
        self.x *= rhs.0;
        self.y *= rhs.0;
        self
    }
}
impl ops::Div<Factor> for euclid::Point2D<f32, Px> {
    type Output = euclid::Point2D<f32, Px>;

    fn div(mut self, rhs: Factor) -> euclid::Point2D<f32, Px> {
        self.x /= rhs.0;
        self.y /= rhs.0;
        self
    }
}
impl ops::MulAssign<Factor> for euclid::Point2D<f32, Px> {
    fn mul_assign(&mut self, rhs: Factor) {
        self.x *= rhs.0;
        self.y *= rhs.0;
    }
}
impl ops::DivAssign<Factor> for euclid::Point2D<f32, Px> {
    fn div_assign(&mut self, rhs: Factor) {
        self.x /= rhs.0;
        self.y /= rhs.0;
    }
}

impl ops::Mul<Factor> for DipPoint {
    type Output = DipPoint;

    fn mul(mut self, rhs: Factor) -> DipPoint {
        self.x *= rhs;
        self.y *= rhs;
        self
    }
}
impl ops::Div<Factor> for DipPoint {
    type Output = DipPoint;

    fn div(mut self, rhs: Factor) -> DipPoint {
        self.x /= rhs;
        self.y /= rhs;
        self
    }
}
impl ops::MulAssign<Factor> for DipPoint {
    fn mul_assign(&mut self, rhs: Factor) {
        self.x *= rhs;
        self.y *= rhs;
    }
}
impl ops::DivAssign<Factor> for DipPoint {
    fn div_assign(&mut self, rhs: Factor) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl ops::Mul<Factor> for PxVector {
    type Output = PxVector;

    fn mul(mut self, rhs: Factor) -> PxVector {
        self.x *= rhs;
        self.y *= rhs;
        self
    }
}
impl ops::Div<Factor> for PxVector {
    type Output = PxVector;

    fn div(mut self, rhs: Factor) -> PxVector {
        self.x /= rhs;
        self.y /= rhs;
        self
    }
}
impl ops::MulAssign<Factor> for PxVector {
    fn mul_assign(&mut self, rhs: Factor) {
        self.x *= rhs;
        self.y *= rhs;
    }
}
impl ops::DivAssign<Factor> for PxVector {
    fn div_assign(&mut self, rhs: Factor) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl ops::Mul<Factor> for DipVector {
    type Output = DipVector;

    fn mul(mut self, rhs: Factor) -> DipVector {
        self.x *= rhs;
        self.y *= rhs;
        self
    }
}
impl ops::Div<Factor> for DipVector {
    type Output = DipVector;

    fn div(mut self, rhs: Factor) -> DipVector {
        self.x /= rhs;
        self.y /= rhs;
        self
    }
}
impl ops::MulAssign<Factor> for DipVector {
    fn mul_assign(&mut self, rhs: Factor) {
        self.x *= rhs;
        self.y *= rhs;
    }
}
impl ops::DivAssign<Factor> for DipVector {
    fn div_assign(&mut self, rhs: Factor) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl ops::Mul<Factor> for PxSize {
    type Output = PxSize;

    fn mul(mut self, rhs: Factor) -> PxSize {
        self.width *= rhs;
        self.height *= rhs;
        self
    }
}
impl ops::Div<Factor> for PxSize {
    type Output = PxSize;

    fn div(mut self, rhs: Factor) -> PxSize {
        self.width /= rhs;
        self.height /= rhs;
        self
    }
}
impl ops::MulAssign<Factor> for PxSize {
    fn mul_assign(&mut self, rhs: Factor) {
        self.width *= rhs;
        self.height *= rhs;
    }
}
impl ops::DivAssign<Factor> for PxSize {
    fn div_assign(&mut self, rhs: Factor) {
        self.width /= rhs;
        self.height /= rhs;
    }
}

impl ops::Mul<Factor> for euclid::Size2D<f32, Px> {
    type Output = euclid::Size2D<f32, Px>;

    fn mul(mut self, rhs: Factor) -> euclid::Size2D<f32, Px> {
        self.width *= rhs.0;
        self.height *= rhs.0;
        self
    }
}
impl ops::Div<Factor> for euclid::Size2D<f32, Px> {
    type Output = euclid::Size2D<f32, Px>;

    fn div(mut self, rhs: Factor) -> euclid::Size2D<f32, Px> {
        self.width /= rhs.0;
        self.height /= rhs.0;
        self
    }
}
impl ops::MulAssign<Factor> for euclid::Size2D<f32, Px> {
    fn mul_assign(&mut self, rhs: Factor) {
        self.width *= rhs.0;
        self.height *= rhs.0;
    }
}
impl ops::DivAssign<Factor> for euclid::Size2D<f32, Px> {
    fn div_assign(&mut self, rhs: Factor) {
        self.width /= rhs.0;
        self.height /= rhs.0;
    }
}

impl ops::Mul<Factor> for DipSize {
    type Output = DipSize;

    fn mul(mut self, rhs: Factor) -> DipSize {
        self.width *= rhs;
        self.height *= rhs;
        self
    }
}
impl ops::Div<Factor> for DipSize {
    type Output = DipSize;

    fn div(mut self, rhs: Factor) -> DipSize {
        self.width /= rhs;
        self.height /= rhs;
        self
    }
}
impl ops::MulAssign<Factor> for DipSize {
    fn mul_assign(&mut self, rhs: Factor) {
        self.width *= rhs;
        self.height *= rhs;
    }
}
impl ops::DivAssign<Factor> for DipSize {
    fn div_assign(&mut self, rhs: Factor) {
        self.width /= rhs;
        self.height /= rhs;
    }
}
impl ops::Mul<Factor> for PxRect {
    type Output = PxRect;

    fn mul(mut self, rhs: Factor) -> PxRect {
        self.origin *= rhs;
        self.size *= rhs;
        self
    }
}
impl ops::Div<Factor> for PxRect {
    type Output = PxRect;

    fn div(mut self, rhs: Factor) -> PxRect {
        self.origin /= rhs;
        self.size /= rhs;
        self
    }
}
impl ops::MulAssign<Factor> for PxRect {
    fn mul_assign(&mut self, rhs: Factor) {
        self.origin *= rhs;
        self.size *= rhs;
    }
}
impl ops::DivAssign<Factor> for PxRect {
    fn div_assign(&mut self, rhs: Factor) {
        self.origin /= rhs;
        self.size /= rhs;
    }
}

impl ops::Mul<Factor> for DipRect {
    type Output = DipRect;

    fn mul(mut self, rhs: Factor) -> DipRect {
        self.origin *= rhs;
        self.size *= rhs;
        self
    }
}
impl ops::Div<Factor> for DipRect {
    type Output = DipRect;

    fn div(mut self, rhs: Factor) -> DipRect {
        self.origin /= rhs;
        self.size /= rhs;
        self
    }
}
impl ops::MulAssign<Factor> for DipRect {
    fn mul_assign(&mut self, rhs: Factor) {
        self.origin *= rhs;
        self.size *= rhs;
    }
}
impl ops::DivAssign<Factor> for DipRect {
    fn div_assign(&mut self, rhs: Factor) {
        self.origin /= rhs;
        self.size /= rhs;
    }
}

impl ops::Neg for Factor {
    type Output = Factor;

    fn neg(self) -> Self::Output {
        Factor(-self.0)
    }
}
impl From<bool> for Factor {
    fn from(value: bool) -> Self {
        if value { Factor(1.0) } else { Factor(0.0) }
    }
}

macro_rules! impl_for_integer {
    ($($T:ty),+ $(,)?) => {$(
        impl ops::Mul<Factor> for $T {
            type Output = $T;

            fn mul(self, rhs: Factor) -> $T {
                (self as f64 * rhs.0 as f64).round() as $T
            }
        }
        impl ops::Div<Factor> for $T {
            type Output = $T;

            fn div(self, rhs: Factor) -> $T {
                (self as f64 / rhs.0 as f64).round() as $T
            }
        }
        impl ops::MulAssign<Factor> for $T {
            fn mul_assign(&mut self, rhs: Factor) {
                *self = *self * rhs;
            }
        }
        impl ops::DivAssign<Factor> for $T {
            fn div_assign(&mut self, rhs: Factor) {
                *self = *self / rhs;
            }
        }
    )+}
}
impl_for_integer! { u8, i8, u16, i16, u32, i32, u64, i64, usize, isize, u128, i128 }

impl ops::Mul<Factor> for f32 {
    type Output = f32;

    fn mul(self, rhs: Factor) -> f32 {
        self * rhs.0
    }
}
impl ops::Div<Factor> for f32 {
    type Output = f32;

    fn div(self, rhs: Factor) -> f32 {
        self / rhs.0
    }
}
impl ops::MulAssign<Factor> for f32 {
    fn mul_assign(&mut self, rhs: Factor) {
        *self = *self * rhs;
    }
}
impl ops::DivAssign<Factor> for f32 {
    fn div_assign(&mut self, rhs: Factor) {
        *self = *self / rhs;
    }
}

impl ops::Mul<Factor> for f64 {
    type Output = f64;

    fn mul(self, rhs: Factor) -> f64 {
        self * rhs.0 as f64
    }
}
impl ops::Div<Factor> for f64 {
    type Output = f64;

    fn div(self, rhs: Factor) -> f64 {
        self / rhs.0 as f64
    }
}
impl ops::MulAssign<Factor> for f64 {
    fn mul_assign(&mut self, rhs: Factor) {
        *self = *self * rhs;
    }
}
impl ops::DivAssign<Factor> for f64 {
    fn div_assign(&mut self, rhs: Factor) {
        *self = *self / rhs;
    }
}

impl ops::Mul<Factor> for Duration {
    type Output = Duration;

    fn mul(self, rhs: Factor) -> Duration {
        self.mul_f32(rhs.0)
    }
}
impl ops::Div<Factor> for Duration {
    type Output = Duration;

    fn div(self, rhs: Factor) -> Duration {
        self.div_f32(rhs.0)
    }
}
impl ops::MulAssign<Factor> for Duration {
    fn mul_assign(&mut self, rhs: Factor) {
        *self = *self * rhs;
    }
}
impl ops::DivAssign<Factor> for Duration {
    fn div_assign(&mut self, rhs: Factor) {
        *self = *self / rhs;
    }
}

impl From<Factor> for FactorPercent {
    fn from(value: Factor) -> Self {
        Self(value.0 * 100.0)
    }
}
impl From<FactorPercent> for Factor {
    fn from(value: FactorPercent) -> Self {
        Self(value.0 / 100.0)
    }
}

/// Multiplication factor in percentage (0%-100%).
///
/// See [`FactorUnits`] for more details.
///
/// # Equality
///
/// Equality is determined using [`about_eq`] with `0.001` granularity.
#[derive(Copy, Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct FactorPercent(pub f32);
impl FactorPercent {
    /// Clamp factor to [0.0..=100.0] range.
    pub fn clamp_range(self) -> Self {
        FactorPercent(self.0.clamp(0.0, 100.0))
    }

    /// Convert to [`Factor`].
    pub fn fct(self) -> Factor {
        self.into()
    }
}
impl ops::Add for FactorPercent {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl ops::AddAssign for FactorPercent {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}
impl ops::Sub for FactorPercent {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}
impl ops::SubAssign for FactorPercent {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
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
        about_eq(self.0, other.0, EQ_GRANULARITY_100)
    }
}
impl Eq for FactorPercent {}
impl ops::Mul for FactorPercent {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}
impl ops::MulAssign for FactorPercent {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}
impl ops::Div for FactorPercent {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self(self.0 / rhs.0)
    }
}
impl ops::DivAssign for FactorPercent {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
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
        // round by 2 decimal places, without printing `.00`
        write!(f, "{}%", (self.0 * 100.0).round() / 100.0)
    }
}

impl ops::Mul<Factor> for FactorPercent {
    type Output = FactorPercent;

    fn mul(self, rhs: Factor) -> Self {
        Self(self.0 * rhs.0)
    }
}
impl ops::Div<Factor> for FactorPercent {
    type Output = FactorPercent;

    fn div(self, rhs: Factor) -> Self {
        Self(self.0 / rhs.0)
    }
}
impl ops::MulAssign<Factor> for FactorPercent {
    fn mul_assign(&mut self, rhs: Factor) {
        *self = *self * rhs;
    }
}
impl ops::DivAssign<Factor> for FactorPercent {
    fn div_assign(&mut self, rhs: Factor) {
        *self = *self / rhs;
    }
}

/// Parses `"##"`, `"##%"` and `"##.fct()"` where `##` is a `f32`.
impl std::str::FromStr for FactorPercent {
    type Err = std::num::ParseFloatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        crate::parse_suffix(s, &["%", ".pct()"]).map(FactorPercent)
    }
}
