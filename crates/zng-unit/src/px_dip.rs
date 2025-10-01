use std::{cmp, fmt, ops};

use serde::{Deserialize, Serialize};

use crate::{CornerRadius2D, Factor, side_offsets::SideOffsets2D};

/// Same value used in `60`.
const DIP_TO_PX: i32 = 60;

/// Device pixel.
///
/// Represents an actual device pixel, not descaled by the pixel scale factor.
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Px(pub i32);
impl Px {
    /// See [`DipToPx`].
    pub fn from_dip(dip: Dip, scale_factor: Factor) -> Px {
        Px((dip.0 as f32 / DIP_TO_PX as f32 * scale_factor.0).round() as i32)
    }

    /// Compares and returns the maximum of two pixel values.
    pub fn max(self, other: Px) -> Px {
        Px(self.0.max(other.0))
    }

    /// Compares and returns the minimum of two pixel values.
    pub fn min(self, other: Px) -> Px {
        Px(self.0.min(other.0))
    }

    /// Computes the saturating absolute value of `self`.
    pub fn abs(self) -> Px {
        Px(self.0.saturating_abs())
    }

    /// [`i32::MAX`].
    pub const MAX: Px = Px(i32::MAX);

    /// [`i32::MIN`].
    pub const MIN: Px = Px(i32::MIN);
}
impl fmt::Debug for Px {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}px", self.0)
    }
}
impl fmt::Display for Px {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}px", self.0)
    }
}
/// Parses `"##"` and `"##px"` where `##` is an `i32`.
impl std::str::FromStr for Px {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        crate::parse_suffix(s, &["px"]).map(Px)
    }
}
impl num_traits::ToPrimitive for Px {
    fn to_i32(&self) -> Option<i32> {
        Some(self.0)
    }
    fn to_i64(&self) -> Option<i64> {
        Some(self.0 as i64)
    }

    fn to_u64(&self) -> Option<u64> {
        Some(self.0 as u64)
    }
}
impl num_traits::NumCast for Px {
    fn from<T: num_traits::ToPrimitive>(n: T) -> Option<Self> {
        if let Some(p) = n.to_i32() {
            Some(Px(p))
        } else {
            n.to_f32().map(|p| Px(p as i32))
        }
    }
}
impl num_traits::Zero for Px {
    fn zero() -> Self {
        Px(0)
    }

    fn is_zero(&self) -> bool {
        self.0 == 0
    }
}
impl num_traits::One for Px {
    fn one() -> Self {
        Px(1)
    }
}
impl euclid::num::Round for Px {
    fn round(self) -> Self {
        self
    }
}
impl euclid::num::Ceil for Px {
    fn ceil(self) -> Self {
        self
    }
}
impl euclid::num::Floor for Px {
    fn floor(self) -> Self {
        self
    }
}
impl num_traits::Num for Px {
    type FromStrRadixErr = <i32 as num_traits::Num>::FromStrRadixErr;

    fn from_str_radix(str: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
        num_traits::Num::from_str_radix(str, radix).map(Px)
    }
}
impl num_traits::Signed for Px {
    fn abs(&self) -> Self {
        Px(self.0.abs())
    }

    fn abs_sub(&self, other: &Self) -> Self {
        Px(num_traits::Signed::abs_sub(&self.0, &other.0))
    }

    fn signum(&self) -> Self {
        Px(num_traits::Signed::signum(&self.0))
    }

    fn is_positive(&self) -> bool {
        self.0 > 0
    }

    fn is_negative(&self) -> bool {
        self.0 < 0
    }
}
impl From<i32> for Px {
    fn from(px: i32) -> Self {
        Px(px)
    }
}
impl ops::Add for Px {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Px(self.0.saturating_add(rhs.0))
    }
}
impl ops::AddAssign for Px {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}
impl ops::Sub for Px {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Px(self.0.saturating_sub(rhs.0))
    }
}
impl ops::SubAssign for Px {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}
impl ops::Mul<f32> for Px {
    type Output = Px;

    fn mul(self, rhs: f32) -> Self::Output {
        Px((self.0 as f32 * rhs).round() as i32)
    }
}
impl ops::MulAssign<f32> for Px {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}
impl ops::Mul<i32> for Px {
    type Output = Px;

    fn mul(self, rhs: i32) -> Self::Output {
        Px(self.0 * rhs)
    }
}
impl ops::MulAssign<i32> for Px {
    fn mul_assign(&mut self, rhs: i32) {
        *self = *self * rhs;
    }
}
impl ops::Mul<Px> for Px {
    type Output = Px;

    fn mul(self, rhs: Px) -> Self::Output {
        Px(self.0.saturating_mul(rhs.0))
    }
}
impl ops::MulAssign<Px> for Px {
    fn mul_assign(&mut self, rhs: Px) {
        *self = *self * rhs;
    }
}
impl ops::Div<f32> for Px {
    type Output = Px;

    fn div(self, rhs: f32) -> Self::Output {
        Px((self.0 as f32 / rhs).round() as i32)
    }
}
impl ops::Div<i32> for Px {
    type Output = Px;

    fn div(self, rhs: i32) -> Self::Output {
        Px(self.0 / rhs)
    }
}
impl ops::Div<Px> for Px {
    type Output = Px;

    fn div(self, rhs: Px) -> Self::Output {
        Px(self.0 / rhs.0)
    }
}
impl ops::DivAssign<f32> for Px {
    fn div_assign(&mut self, rhs: f32) {
        *self = *self / rhs;
    }
}
impl ops::DivAssign<i32> for Px {
    fn div_assign(&mut self, rhs: i32) {
        *self = *self / rhs;
    }
}
impl ops::DivAssign<Px> for Px {
    fn div_assign(&mut self, rhs: Px) {
        *self = *self / rhs;
    }
}
impl ops::Neg for Px {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Px(self.0.saturating_neg())
    }
}
impl ops::Rem for Px {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        Px(self.0 % rhs.0)
    }
}
impl std::iter::Sum for Px {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Px(0), |a, b| a + b)
    }
}
impl PartialEq<i32> for Px {
    fn eq(&self, other: &i32) -> bool {
        *self == Px(*other)
    }
}
impl PartialOrd<i32> for Px {
    fn partial_cmp(&self, other: &i32) -> Option<cmp::Ordering> {
        self.partial_cmp(&Px(*other))
    }
}

/// Device independent pixel.
///
/// Represent a device pixel descaled by the pixel scale factor.
///
/// Internally this is an `i32` that represents 1/60th of a pixel.
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, bytemuck::Zeroable, bytemuck::Pod)]
#[serde(from = "f32")]
#[serde(into = "f32")]
#[repr(transparent)]
pub struct Dip(i32);
impl Dip {
    /// New from round integer value.
    pub const fn new(dip: i32) -> Self {
        Dip(dip * DIP_TO_PX)
    }

    /// new from floating point.
    pub fn new_f32(dip: f32) -> Self {
        Dip((dip * DIP_TO_PX as f32).round() as i32)
    }

    /// See [`PxToDip`].
    pub fn from_px(px: Px, scale_factor: Factor) -> Dip {
        Dip((px.0 as f32 / scale_factor.0 * DIP_TO_PX as f32).round() as i32)
    }

    /// Returns `self` as [`f32`].
    pub fn to_f32(self) -> f32 {
        self.0 as f32 / DIP_TO_PX as f32
    }

    /// Returns `self` as [`i32`].
    pub fn to_i32(self) -> i32 {
        self.0 / DIP_TO_PX
    }

    /// Compares and returns the maximum of two pixel values.
    pub fn max(self, other: Dip) -> Dip {
        Dip(self.0.max(other.0))
    }

    /// Compares and returns the minimum of two pixel values.
    pub fn min(self, other: Dip) -> Dip {
        Dip(self.0.min(other.0))
    }

    /// Computes the saturating absolute value of `self`.
    pub fn abs(self) -> Dip {
        Dip(self.0.saturating_abs())
    }

    /// Maximum DIP value.
    pub const MAX: Dip = Dip(i32::MAX / DIP_TO_PX);
    /// Minimum DIP value.
    pub const MIN: Dip = Dip(i32::MIN / DIP_TO_PX);
}
impl fmt::Debug for Dip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl fmt::Display for Dip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.to_f32(), f)?;
        write!(f, "dip")
    }
}
/// Parses `"##"` and `"##dip"` where `##` is an `f32`.
impl std::str::FromStr for Dip {
    type Err = std::num::ParseFloatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        crate::parse_suffix(s, &["dip"]).map(Dip::new_f32)
    }
}
impl From<i32> for Dip {
    fn from(dip: i32) -> Self {
        Dip::new(dip)
    }
}
impl From<f32> for Dip {
    fn from(dip: f32) -> Self {
        Dip::new_f32(dip)
    }
}
impl From<Dip> for f32 {
    fn from(value: Dip) -> Self {
        value.to_f32()
    }
}
impl ops::Add for Dip {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Dip(self.0.saturating_add(rhs.0))
    }
}
impl ops::AddAssign for Dip {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}
impl ops::Sub for Dip {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Dip(self.0.saturating_sub(rhs.0))
    }
}
impl ops::SubAssign for Dip {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}
impl ops::Neg for Dip {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Dip(self.0.saturating_neg())
    }
}
impl ops::Mul<f32> for Dip {
    type Output = Dip;

    fn mul(self, rhs: f32) -> Self::Output {
        Dip((self.0 as f32 * rhs).round() as i32)
    }
}
impl ops::MulAssign<f32> for Dip {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}
impl ops::Mul<Dip> for Dip {
    type Output = Dip;

    fn mul(self, rhs: Dip) -> Self::Output {
        Dip(self.0.saturating_mul(rhs.to_i32()))
    }
}
impl ops::MulAssign<Dip> for Dip {
    fn mul_assign(&mut self, rhs: Dip) {
        *self = *self * rhs;
    }
}
impl ops::Div<f32> for Dip {
    type Output = Dip;

    fn div(self, rhs: f32) -> Self::Output {
        Dip((self.0 as f32 / rhs).round() as i32)
    }
}
impl ops::DivAssign<f32> for Dip {
    fn div_assign(&mut self, rhs: f32) {
        *self = *self / rhs;
    }
}
impl ops::Div<Dip> for Dip {
    type Output = Dip;

    fn div(self, rhs: Dip) -> Self::Output {
        Dip::new(self.0 / rhs.0)
    }
}
impl ops::DivAssign<Dip> for Dip {
    fn div_assign(&mut self, rhs: Dip) {
        *self = *self / rhs;
    }
}
impl ops::Rem for Dip {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        Dip(self.0 % rhs.0)
    }
}
impl ops::RemAssign for Dip {
    fn rem_assign(&mut self, rhs: Self) {
        *self = *self % rhs;
    }
}
impl num_traits::ToPrimitive for Dip {
    fn to_i64(&self) -> Option<i64> {
        Some(Dip::to_i32(*self) as i64)
    }

    fn to_u64(&self) -> Option<u64> {
        if self.0 >= 0 { Some(Dip::to_i32(*self) as u64) } else { None }
    }

    fn to_f32(&self) -> Option<f32> {
        Some(Dip::to_f32(*self))
    }

    fn to_f64(&self) -> Option<f64> {
        Some(Dip::to_f32(*self) as f64)
    }
}
impl num_traits::NumCast for Dip {
    fn from<T: num_traits::ToPrimitive>(n: T) -> Option<Self> {
        #[expect(clippy::manual_map)]
        if let Some(n) = n.to_f32() {
            Some(Dip::new_f32(n))
        } else if let Some(n) = n.to_i32() {
            Some(Dip::new(n))
        } else {
            None
        }
    }
}
impl num_traits::Zero for Dip {
    fn zero() -> Self {
        Dip(0)
    }

    fn is_zero(&self) -> bool {
        self.0 == 0
    }
}
impl num_traits::One for Dip {
    fn one() -> Self {
        Dip::new(1)
    }
}
impl euclid::num::Round for Dip {
    fn round(self) -> Self {
        Dip::new_f32(self.to_f32().round())
    }
}
impl euclid::num::Ceil for Dip {
    fn ceil(self) -> Self {
        Dip::new_f32(self.to_f32().ceil())
    }
}
impl euclid::num::Floor for Dip {
    fn floor(self) -> Self {
        Dip::new_f32(self.to_f32().floor())
    }
}
impl num_traits::Signed for Dip {
    fn abs(&self) -> Self {
        Dip(self.0.abs())
    }

    fn abs_sub(&self, other: &Self) -> Self {
        Dip(num_traits::Signed::abs_sub(&self.0, &other.0))
    }

    fn signum(&self) -> Self {
        match self.0.cmp(&0) {
            cmp::Ordering::Less => Dip::new(-1),
            cmp::Ordering::Equal => Dip(0),
            cmp::Ordering::Greater => Dip::new(1),
        }
    }

    fn is_positive(&self) -> bool {
        self.0 > 0
    }

    fn is_negative(&self) -> bool {
        self.0 < 0
    }
}
impl num_traits::Num for Dip {
    type FromStrRadixErr = <i32 as num_traits::Num>::FromStrRadixErr;

    fn from_str_radix(str: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
        num_traits::Num::from_str_radix(str, radix).map(Dip::new)
    }
}
impl PartialEq<i32> for Dip {
    fn eq(&self, other: &i32) -> bool {
        *self == Dip::new(*other)
    }
}
impl PartialOrd<i32> for Dip {
    fn partial_cmp(&self, other: &i32) -> Option<cmp::Ordering> {
        self.partial_cmp(&Dip::new(*other))
    }
}
impl PartialEq<f32> for Dip {
    fn eq(&self, other: &f32) -> bool {
        *self == Dip::new_f32(*other)
    }
}
impl PartialOrd<f32> for Dip {
    fn partial_cmp(&self, other: &f32) -> Option<cmp::Ordering> {
        self.partial_cmp(&Dip::new_f32(*other))
    }
}

/// A point in device pixels.
pub type PxPoint = euclid::Point2D<Px, Px>;

/// A point in device independent pixels.
pub type DipPoint = euclid::Point2D<Dip, Dip>;

/// A vector in device pixels.
pub type PxVector = euclid::Vector2D<Px, Px>;

/// A vector in device independent pixels.
pub type DipVector = euclid::Vector2D<Dip, Dip>;

/// A size in device pixels.
pub type PxSize = euclid::Size2D<Px, Px>;

/// A size in device pixels.
pub type DipSize = euclid::Size2D<Dip, Dip>;

/// A rectangle in device pixels.
pub type PxRect = euclid::Rect<Px, Px>;

/// A rectangle box in device pixels.
pub type PxBox = euclid::Box2D<Px, Px>;

/// A rectangle in device independent pixels.
pub type DipRect = euclid::Rect<Dip, Dip>;

/// A rectangle box in device independent pixels.
pub type DipBox = euclid::Box2D<Dip, Dip>;

/// Side-offsets in device pixels.
pub type PxSideOffsets = SideOffsets2D<Px, Px>;
/// Side-offsets in device independent pixels.
pub type DipSideOffsets = SideOffsets2D<Dip, Dip>;

/// Corner-radius in device pixels.
pub type PxCornerRadius = CornerRadius2D<Px, Px>;

/// Corner-radius in device independent pixels.
pub type DipCornerRadius = CornerRadius2D<Dip, Dip>;

/// Conversion from [`Px`] to [`Dip`] units.
pub trait PxToDip {
    /// `Self` equivalent in [`Dip`] units.
    type AsDip;

    /// Divide the [`Px`] self by the scale.
    fn to_dip(self, scale_factor: Factor) -> Self::AsDip;
}

/// Conversion from [`Dip`] to [`Px`] units.
pub trait DipToPx {
    /// `Self` equivalent in [`Px`] units.
    type AsPx;

    /// Multiply the [`Dip`] self by the scale.
    fn to_px(self, scale_factor: Factor) -> Self::AsPx;
}

impl PxToDip for Px {
    type AsDip = Dip;

    fn to_dip(self, scale_factor: Factor) -> Self::AsDip {
        Dip::from_px(self, scale_factor)
    }
}

impl DipToPx for Dip {
    type AsPx = Px;

    fn to_px(self, scale_factor: Factor) -> Self::AsPx {
        Px::from_dip(self, scale_factor)
    }
}

impl PxToDip for PxPoint {
    type AsDip = DipPoint;

    fn to_dip(self, scale_factor: Factor) -> Self::AsDip {
        DipPoint::new(self.x.to_dip(scale_factor), self.y.to_dip(scale_factor))
    }
}

impl DipToPx for DipPoint {
    type AsPx = PxPoint;

    fn to_px(self, scale_factor: Factor) -> Self::AsPx {
        PxPoint::new(self.x.to_px(scale_factor), self.y.to_px(scale_factor))
    }
}
impl DipToPx for euclid::Point2D<f32, Dip> {
    type AsPx = euclid::Point2D<f32, Px>;

    fn to_px(self, scale_factor: Factor) -> Self::AsPx {
        euclid::point2(self.x * scale_factor.0, self.y * scale_factor.0)
    }
}

impl PxToDip for PxSize {
    type AsDip = DipSize;

    fn to_dip(self, scale_factor: Factor) -> Self::AsDip {
        DipSize::new(self.width.to_dip(scale_factor), self.height.to_dip(scale_factor))
    }
}

impl DipToPx for DipSize {
    type AsPx = PxSize;

    fn to_px(self, scale_factor: Factor) -> Self::AsPx {
        PxSize::new(self.width.to_px(scale_factor), self.height.to_px(scale_factor))
    }
}

impl DipToPx for DipVector {
    type AsPx = PxVector;

    fn to_px(self, scale_factor: Factor) -> Self::AsPx {
        PxVector::new(self.x.to_px(scale_factor), self.y.to_px(scale_factor))
    }
}
impl PxToDip for PxVector {
    type AsDip = DipVector;

    fn to_dip(self, scale_factor: Factor) -> Self::AsDip {
        DipVector::new(self.x.to_dip(scale_factor), self.y.to_dip(scale_factor))
    }
}

impl PxToDip for PxRect {
    type AsDip = DipRect;

    fn to_dip(self, scale_factor: Factor) -> Self::AsDip {
        DipRect::new(self.origin.to_dip(scale_factor), self.size.to_dip(scale_factor))
    }
}

impl DipToPx for DipRect {
    type AsPx = PxRect;

    fn to_px(self, scale_factor: Factor) -> Self::AsPx {
        PxRect::new(self.origin.to_px(scale_factor), self.size.to_px(scale_factor))
    }
}

impl PxToDip for PxBox {
    type AsDip = DipBox;

    fn to_dip(self, scale_factor: Factor) -> Self::AsDip {
        DipBox::new(self.min.to_dip(scale_factor), self.max.to_dip(scale_factor))
    }
}

impl DipToPx for DipBox {
    type AsPx = PxBox;

    fn to_px(self, scale_factor: Factor) -> Self::AsPx {
        PxBox::new(self.min.to_px(scale_factor), self.max.to_px(scale_factor))
    }
}

impl DipToPx for DipSideOffsets {
    type AsPx = PxSideOffsets;

    fn to_px(self, scale_factor: Factor) -> Self::AsPx {
        PxSideOffsets::new(
            self.top.to_px(scale_factor),
            self.right.to_px(scale_factor),
            self.bottom.to_px(scale_factor),
            self.left.to_px(scale_factor),
        )
    }
}
impl PxToDip for PxSideOffsets {
    type AsDip = DipSideOffsets;

    fn to_dip(self, scale_factor: Factor) -> Self::AsDip {
        DipSideOffsets::new(
            self.top.to_dip(scale_factor),
            self.right.to_dip(scale_factor),
            self.bottom.to_dip(scale_factor),
            self.left.to_dip(scale_factor),
        )
    }
}

impl DipToPx for DipCornerRadius {
    type AsPx = PxCornerRadius;

    fn to_px(self, scale_factor: Factor) -> Self::AsPx {
        PxCornerRadius::new(
            self.top_left.to_px(scale_factor),
            self.top_right.to_px(scale_factor),
            self.bottom_left.to_px(scale_factor),
            self.bottom_right.to_px(scale_factor),
        )
    }
}
impl PxToDip for PxCornerRadius {
    type AsDip = DipCornerRadius;

    fn to_dip(self, scale_factor: Factor) -> Self::AsDip {
        DipCornerRadius::new(
            self.top_left.to_dip(scale_factor),
            self.top_right.to_dip(scale_factor),
            self.bottom_left.to_dip(scale_factor),
            self.bottom_right.to_dip(scale_factor),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dip_px_1_1_conversion() {
        let px = Dip::new(100).to_px(Factor(1.0));
        assert_eq!(px, Px(100));
    }

    #[test]
    fn px_dip_1_1_conversion() {
        let dip = Px(100).to_dip(Factor(1.0));
        assert_eq!(dip, Dip::new(100));
    }

    #[test]
    fn dip_px_1_15_conversion() {
        let px = Dip::new(100).to_px(Factor(1.5));
        assert_eq!(px, Px(150));
    }

    #[test]
    fn px_dip_1_15_conversion() {
        let dip = Px(150).to_dip(Factor(1.5));
        assert_eq!(dip, Dip::new(100));
    }
}
