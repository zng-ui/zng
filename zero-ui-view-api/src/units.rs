//! Pixel units.
//!
//! This module defines two pixel units [`Px`] that is dependent on the monitor device, and [`Dip`] that is not.
//!
//! All windowing and monitor API uses [`Dip`] units with the scale-factor reported separately and all rendering API uses [`Px`] units.
//!
//! The `webrender` crate only operates in pixel scale 1.0,
//! even thought the documentation of [`webrender_api::units`] indicates that the `LayoutPixel` unit is equivalent to [`Dip`],
//! **it isn't**.
//
//! The recommended way of using these units is defining your own public API to only use [`Dip`] units, and then convert
//! to [`Px`] units to compute layout and render. Working like this should make the window content have the same apparent
//! dimensions in all monitor devices. For rendering the [`Px`] unit can be converted to `webrender` units using [`PxToWr`].

use std::{cmp, fmt, marker::PhantomData, ops, time::Duration};

use webrender_api::units as wr;

pub use webrender_api::euclid;

use serde::{Deserialize, Serialize};

/// Same value used in `60`.
const DIP_TO_PX: i32 = 60;

/// Device pixel.
///
/// Represents an actual device pixel, not scaled/descaled by the pixel scale factor.
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, bytemuck::NoUninit)]
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

/// Device independent pixel.
///
/// Represent a device pixel descaled by the pixel scale factor.
///
/// Internally this is an `i32` that represents 1/60th of a pixel.
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(from = "f32")]
#[serde(into = "f32")]
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
        if self.0 >= 0 {
            Some(Dip::to_i32(*self) as u64)
        } else {
            None
        }
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
        #[allow(clippy::manual_map)]
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

/// A group of 2D side offsets, which correspond to top/right/bottom/left for borders, padding,
/// and margins in CSS, optionally tagged with a unit.
#[derive(Serialize, Deserialize)]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: Deserialize<'de>"))]
pub struct SideOffsets2D<T, U> {
    /// Top offset.
    pub top: T,
    /// Right offset.
    pub right: T,
    /// Bottom offset.
    pub bottom: T,
    /// Left offset.
    pub left: T,
    #[doc(hidden)]
    #[serde(skip)] // euclid does not skip this field
    pub _unit: PhantomData<U>,
}
impl<T, U> From<euclid::SideOffsets2D<T, U>> for SideOffsets2D<T, U> {
    fn from(value: euclid::SideOffsets2D<T, U>) -> Self {
        Self {
            top: value.top,
            right: value.right,
            bottom: value.bottom,
            left: value.left,
            _unit: PhantomData,
        }
    }
}
impl<T, U> From<SideOffsets2D<T, U>> for euclid::SideOffsets2D<T, U> {
    fn from(value: SideOffsets2D<T, U>) -> Self {
        Self {
            top: value.top,
            right: value.right,
            bottom: value.bottom,
            left: value.left,
            _unit: PhantomData,
        }
    }
}
impl<T: Copy, U> Copy for SideOffsets2D<T, U> {}
impl<T: Clone, U> Clone for SideOffsets2D<T, U> {
    fn clone(&self) -> Self {
        SideOffsets2D {
            top: self.top.clone(),
            right: self.right.clone(),
            bottom: self.bottom.clone(),
            left: self.left.clone(),
            _unit: PhantomData,
        }
    }
}
impl<T, U> Eq for SideOffsets2D<T, U> where T: Eq {}
impl<T, U> PartialEq for SideOffsets2D<T, U>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.top == other.top && self.right == other.right && self.bottom == other.bottom && self.left == other.left
    }
}
impl<T, U> std::hash::Hash for SideOffsets2D<T, U>
where
    T: std::hash::Hash,
{
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.top.hash(h);
        self.right.hash(h);
        self.bottom.hash(h);
        self.left.hash(h);
    }
}
impl<T: fmt::Debug, U> fmt::Debug for SideOffsets2D<T, U> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({:?},{:?},{:?},{:?})", self.top, self.right, self.bottom, self.left)
    }
}
impl<T: Default, U> Default for SideOffsets2D<T, U> {
    fn default() -> Self {
        SideOffsets2D {
            top: Default::default(),
            right: Default::default(),
            bottom: Default::default(),
            left: Default::default(),
            _unit: PhantomData,
        }
    }
}
impl<T, U> SideOffsets2D<T, U> {
    /// Constructor taking a scalar for each side.
    ///
    /// Sides are specified in top-right-bottom-left order following
    /// CSS's convention.
    pub const fn new(top: T, right: T, bottom: T, left: T) -> Self {
        SideOffsets2D {
            top,
            right,
            bottom,
            left,
            _unit: PhantomData,
        }
    }

    /// Construct side offsets from min and a max vector offsets.
    ///
    /// The outer rect of the resulting side offsets is equivalent to translating
    /// a rectangle's upper-left corner with the min vector and translating the
    /// bottom-right corner with the max vector.
    pub fn from_vectors_outer(min: euclid::Vector2D<T, U>, max: euclid::Vector2D<T, U>) -> Self
    where
        T: ops::Neg<Output = T>,
    {
        SideOffsets2D {
            left: -min.x,
            top: -min.y,
            right: max.x,
            bottom: max.y,
            _unit: PhantomData,
        }
    }

    /// Construct side offsets from min and a max vector offsets.
    ///
    /// The inner rect of the resulting side offsets is equivalent to translating
    /// a rectangle's upper-left corner with the min vector and translating the
    /// bottom-right corner with the max vector.
    pub fn from_vectors_inner(min: euclid::Vector2D<T, U>, max: euclid::Vector2D<T, U>) -> Self
    where
        T: ops::Neg<Output = T>,
    {
        SideOffsets2D {
            left: min.x,
            top: min.y,
            right: -max.x,
            bottom: -max.y,
            _unit: PhantomData,
        }
    }

    /// Constructor, setting all sides to zero.
    pub fn zero() -> Self
    where
        T: euclid::num::Zero,
    {
        use euclid::num::Zero;
        SideOffsets2D::new(Zero::zero(), Zero::zero(), Zero::zero(), Zero::zero())
    }

    /// Returns `true` if all side offsets are zero.
    pub fn is_zero(&self) -> bool
    where
        T: euclid::num::Zero + PartialEq,
    {
        let zero = T::zero();
        self.top == zero && self.right == zero && self.bottom == zero && self.left == zero
    }

    /// Constructor setting the same value to all sides, taking a scalar value directly.
    pub fn new_all_same(all: T) -> Self
    where
        T: Copy,
    {
        SideOffsets2D::new(all, all, all, all)
    }

    /// Left + right.
    pub fn horizontal(&self) -> T
    where
        T: Copy + ops::Add<T, Output = T>,
    {
        self.left + self.right
    }

    /// Top + bottom.
    pub fn vertical(&self) -> T
    where
        T: Copy + ops::Add<T, Output = T>,
    {
        self.top + self.bottom
    }
}
impl<T, U> ops::Add for SideOffsets2D<T, U>
where
    T: ops::Add<T, Output = T>,
{
    type Output = Self;
    fn add(self, other: Self) -> Self {
        SideOffsets2D::new(
            self.top + other.top,
            self.right + other.right,
            self.bottom + other.bottom,
            self.left + other.left,
        )
    }
}
impl<T, U> ops::AddAssign<Self> for SideOffsets2D<T, U>
where
    T: ops::AddAssign<T>,
{
    fn add_assign(&mut self, other: Self) {
        self.top += other.top;
        self.right += other.right;
        self.bottom += other.bottom;
        self.left += other.left;
    }
}
impl<T, U> ops::Sub for SideOffsets2D<T, U>
where
    T: ops::Sub<T, Output = T>,
{
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        SideOffsets2D::new(
            self.top - other.top,
            self.right - other.right,
            self.bottom - other.bottom,
            self.left - other.left,
        )
    }
}
impl<T, U> ops::SubAssign<Self> for SideOffsets2D<T, U>
where
    T: ops::SubAssign<T>,
{
    fn sub_assign(&mut self, other: Self) {
        self.top -= other.top;
        self.right -= other.right;
        self.bottom -= other.bottom;
        self.left -= other.left;
    }
}

impl<T, U> ops::Neg for SideOffsets2D<T, U>
where
    T: ops::Neg<Output = T>,
{
    type Output = Self;
    fn neg(self) -> Self {
        SideOffsets2D {
            top: -self.top,
            right: -self.right,
            bottom: -self.bottom,
            left: -self.left,
            _unit: PhantomData,
        }
    }
}
impl<T: Copy + ops::Mul, U> ops::Mul<T> for SideOffsets2D<T, U> {
    type Output = SideOffsets2D<T::Output, U>;

    #[inline]
    fn mul(self, scale: T) -> Self::Output {
        SideOffsets2D::new(self.top * scale, self.right * scale, self.bottom * scale, self.left * scale)
    }
}
impl<T: Copy + ops::MulAssign, U> ops::MulAssign<T> for SideOffsets2D<T, U> {
    #[inline]
    fn mul_assign(&mut self, other: T) {
        self.top *= other;
        self.right *= other;
        self.bottom *= other;
        self.left *= other;
    }
}
impl<T: Copy + ops::Mul, U1, U2> ops::Mul<euclid::Scale<T, U1, U2>> for SideOffsets2D<T, U1> {
    type Output = SideOffsets2D<T::Output, U2>;

    #[inline]
    fn mul(self, scale: euclid::Scale<T, U1, U2>) -> Self::Output {
        SideOffsets2D::new(self.top * scale.0, self.right * scale.0, self.bottom * scale.0, self.left * scale.0)
    }
}
impl<T: Copy + ops::MulAssign, U> ops::MulAssign<euclid::Scale<T, U, U>> for SideOffsets2D<T, U> {
    #[inline]
    fn mul_assign(&mut self, other: euclid::Scale<T, U, U>) {
        *self *= other.0;
    }
}
impl<T: Copy + ops::Div, U> ops::Div<T> for SideOffsets2D<T, U> {
    type Output = SideOffsets2D<T::Output, U>;

    #[inline]
    fn div(self, scale: T) -> Self::Output {
        SideOffsets2D::new(self.top / scale, self.right / scale, self.bottom / scale, self.left / scale)
    }
}
impl<T: Copy + ops::DivAssign, U> ops::DivAssign<T> for SideOffsets2D<T, U> {
    #[inline]
    fn div_assign(&mut self, other: T) {
        self.top /= other;
        self.right /= other;
        self.bottom /= other;
        self.left /= other;
    }
}
impl<T: Copy + ops::Div, U1, U2> ops::Div<euclid::Scale<T, U1, U2>> for SideOffsets2D<T, U2> {
    type Output = SideOffsets2D<T::Output, U1>;

    #[inline]
    fn div(self, scale: euclid::Scale<T, U1, U2>) -> Self::Output {
        SideOffsets2D::new(self.top / scale.0, self.right / scale.0, self.bottom / scale.0, self.left / scale.0)
    }
}
impl<T: Copy + ops::DivAssign, U> ops::DivAssign<euclid::Scale<T, U, U>> for SideOffsets2D<T, U> {
    fn div_assign(&mut self, other: euclid::Scale<T, U, U>) {
        *self /= other.0;
    }
}

/// Ellipses that define the radius of the four corners of a 2D box.
#[derive(Serialize, Deserialize)]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: Deserialize<'de>"))]
pub struct CornerRadius2D<T, U> {
    /// Top-left corner radius.
    pub top_left: euclid::Size2D<T, U>,
    /// Top-right corner radius.
    pub top_right: euclid::Size2D<T, U>,
    /// Bottom-right corner radius.
    pub bottom_right: euclid::Size2D<T, U>,
    /// Bottom-left corner radius.
    pub bottom_left: euclid::Size2D<T, U>,
}
impl<T: Default, U> Default for CornerRadius2D<T, U> {
    fn default() -> Self {
        Self {
            top_left: Default::default(),
            top_right: Default::default(),
            bottom_right: Default::default(),
            bottom_left: Default::default(),
        }
    }
}
impl<T: Clone, U> Clone for CornerRadius2D<T, U> {
    fn clone(&self) -> Self {
        Self {
            top_left: self.top_left.clone(),
            top_right: self.top_right.clone(),
            bottom_right: self.bottom_right.clone(),
            bottom_left: self.bottom_left.clone(),
        }
    }
}
impl<T: Copy, U> Copy for CornerRadius2D<T, U> {}
impl<T: Copy + num_traits::Zero, U> CornerRadius2D<T, U> {
    /// New with distinct values.
    pub fn new(
        top_left: euclid::Size2D<T, U>,
        top_right: euclid::Size2D<T, U>,
        bottom_right: euclid::Size2D<T, U>,
        bottom_left: euclid::Size2D<T, U>,
    ) -> Self {
        Self {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }

    /// New all corners same radius.
    pub fn new_all(radius: euclid::Size2D<T, U>) -> Self {
        Self::new(radius, radius, radius, radius)
    }

    /// All zeros.
    pub fn zero() -> Self {
        Self::new_all(euclid::Size2D::zero())
    }

    /// Calculate the corner radius of an outer border around `self` to perfectly fit.
    pub fn inflate(self, offsets: SideOffsets2D<T, U>) -> Self
    where
        T: ops::AddAssign,
    {
        let mut r = self;

        r.top_left.width += offsets.left;
        r.top_left.height += offsets.top;

        r.top_right.width += offsets.right;
        r.top_right.height += offsets.top;

        r.bottom_right.width += offsets.right;
        r.bottom_right.height += offsets.bottom;

        r.bottom_left.width += offsets.left;
        r.bottom_left.height += offsets.bottom;

        r
    }

    /// Calculate the corner radius of an inner border inside `self` to perfectly fit.
    pub fn deflate(self, offsets: SideOffsets2D<T, U>) -> Self
    where
        T: ops::SubAssign + cmp::PartialOrd,
    {
        let mut r = self;

        if r.top_left.width >= offsets.left {
            r.top_left.width -= offsets.left;
        } else {
            r.top_left.width = T::zero();
        }
        if r.top_left.height >= offsets.top {
            r.top_left.height -= offsets.top;
        } else {
            r.top_left.height = T::zero();
        }

        if r.top_right.width >= offsets.right {
            r.top_right.width -= offsets.right;
        } else {
            r.top_right.width = T::zero();
        }
        if r.top_right.height >= offsets.top {
            r.top_right.height -= offsets.top;
        } else {
            r.top_right.height = T::zero();
        }

        if r.bottom_right.width >= offsets.right {
            r.bottom_right.width -= offsets.right;
        } else {
            r.bottom_right.width = T::zero();
        }
        if r.bottom_right.height >= offsets.bottom {
            r.bottom_right.height -= offsets.bottom;
        } else {
            r.bottom_right.height = T::zero();
        }

        if r.bottom_left.width >= offsets.left {
            r.bottom_left.width -= offsets.left;
        } else {
            r.bottom_left.width = T::zero();
        }
        if r.bottom_left.height >= offsets.bottom {
            r.bottom_left.height -= offsets.bottom;
        } else {
            r.bottom_left.height = T::zero();
        }

        r
    }
}
impl<T: fmt::Debug, U> fmt::Debug for CornerRadius2D<T, U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CornerRadius2D")
            .field("top_left", &self.top_left)
            .field("top_right", &self.top_right)
            .field("bottom_right", &self.bottom_right)
            .field("bottom_left", &self.bottom_left)
            .finish()
    }
}
impl<T: PartialEq, U> PartialEq for CornerRadius2D<T, U> {
    fn eq(&self, other: &Self) -> bool {
        self.top_left == other.top_left
            && self.top_right == other.top_right
            && self.bottom_right == other.bottom_right
            && self.bottom_left == other.bottom_left
    }
}
impl<T: Eq, U> Eq for CornerRadius2D<T, U> {}

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

/// Conversion from [`Px`] to `webrender` units.
///
/// All conversions are 1 to 1.
pub trait PxToWr {
    /// `Self` equivalent in [`webrender_api::units::DevicePixel`] units.
    type AsDevice;
    /// `Self` equivalent in [`webrender_api::units::LayoutPixel`] units.
    type AsLayout;
    /// `Self` equivalent in [`webrender_api::units::WorldPixel`] units.
    type AsWorld;

    /// Returns `self` in [`webrender_api::units::DevicePixel`] units.
    fn to_wr_device(self) -> Self::AsDevice;

    /// Returns `self` in [`webrender_api::units::WorldPixel`] units.
    fn to_wr_world(self) -> Self::AsWorld;

    /// Returns `self` in [`webrender_api::units::LayoutPixel`] units.
    fn to_wr(self) -> Self::AsLayout;
}
/// Conversion from `webrender` to [`Px`] units.
pub trait WrToPx {
    /// `Self` equivalent in [`Px`] units.
    type AsPx;

    /// Returns `self` in [`Px`] units.
    fn to_px(self) -> Self::AsPx;
}

impl PxToDip for Px {
    type AsDip = Dip;

    fn to_dip(self, scale_factor: Factor) -> Self::AsDip {
        Dip::from_px(self, scale_factor)
    }
}
impl PxToWr for Px {
    type AsDevice = wr::DeviceIntLength;

    type AsWorld = euclid::Length<f32, wr::WorldPixel>;
    type AsLayout = euclid::Length<f32, wr::LayoutPixel>;

    fn to_wr_device(self) -> Self::AsDevice {
        wr::DeviceIntLength::new(self.0)
    }

    fn to_wr_world(self) -> Self::AsWorld {
        euclid::Length::new(self.0 as f32)
    }

    fn to_wr(self) -> Self::AsLayout {
        euclid::Length::new(self.0 as f32)
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
impl PxToWr for PxPoint {
    type AsDevice = wr::DeviceIntPoint;
    type AsWorld = wr::WorldPoint;
    type AsLayout = wr::LayoutPoint;

    fn to_wr_device(self) -> Self::AsDevice {
        wr::DeviceIntPoint::new(self.x.to_wr_device().0, self.y.to_wr_device().0)
    }

    fn to_wr_world(self) -> Self::AsWorld {
        wr::WorldPoint::new(self.x.to_wr_world().0, self.y.to_wr_world().0)
    }

    fn to_wr(self) -> Self::AsLayout {
        wr::LayoutPoint::new(self.x.to_wr().0, self.y.to_wr().0)
    }
}
impl WrToPx for wr::LayoutPoint {
    type AsPx = PxPoint;

    fn to_px(self) -> Self::AsPx {
        PxPoint::new(Px(self.x.round() as i32), Px(self.y.round() as i32))
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
impl PxToWr for PxSize {
    type AsDevice = wr::DeviceIntSize;
    type AsWorld = wr::WorldSize;
    type AsLayout = wr::LayoutSize;

    fn to_wr_device(self) -> Self::AsDevice {
        wr::DeviceIntSize::new(self.width.to_wr_device().0, self.height.to_wr_device().0)
    }

    fn to_wr_world(self) -> Self::AsWorld {
        wr::WorldSize::new(self.width.to_wr_world().0, self.height.to_wr_world().0)
    }

    fn to_wr(self) -> Self::AsLayout {
        wr::LayoutSize::new(self.width.to_wr().0, self.height.to_wr().0)
    }
}
impl WrToPx for wr::LayoutSize {
    type AsPx = PxSize;

    fn to_px(self) -> Self::AsPx {
        PxSize::new(Px(self.width.round() as i32), Px(self.height.round() as i32))
    }
}
impl DipToPx for DipSize {
    type AsPx = PxSize;

    fn to_px(self, scale_factor: Factor) -> Self::AsPx {
        PxSize::new(self.width.to_px(scale_factor), self.height.to_px(scale_factor))
    }
}
impl WrToPx for wr::DeviceIntSize {
    type AsPx = PxSize;

    fn to_px(self) -> Self::AsPx {
        PxSize::new(Px(self.width), Px(self.height))
    }
}
impl PxToWr for PxVector {
    type AsDevice = wr::DeviceVector2D;

    type AsLayout = wr::LayoutVector2D;

    type AsWorld = wr::WorldVector2D;

    fn to_wr_device(self) -> Self::AsDevice {
        wr::DeviceVector2D::new(self.x.0 as f32, self.y.0 as f32)
    }

    fn to_wr_world(self) -> Self::AsWorld {
        wr::WorldVector2D::new(self.x.0 as f32, self.y.0 as f32)
    }

    fn to_wr(self) -> Self::AsLayout {
        wr::LayoutVector2D::new(self.x.0 as f32, self.y.0 as f32)
    }
}
impl WrToPx for wr::LayoutVector2D {
    type AsPx = PxVector;

    fn to_px(self) -> Self::AsPx {
        PxVector::new(Px(self.x.round() as i32), Px(self.y.round() as i32))
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
impl PxToWr for PxRect {
    type AsDevice = wr::DeviceIntRect;

    type AsWorld = wr::WorldRect;

    type AsLayout = wr::LayoutRect;

    fn to_wr_device(self) -> Self::AsDevice {
        wr::DeviceIntRect::from_origin_and_size(self.origin.to_wr_device(), self.size.to_wr_device())
    }

    fn to_wr_world(self) -> Self::AsWorld {
        wr::WorldRect::from_origin_and_size(self.origin.to_wr_world(), self.size.to_wr_world())
    }

    fn to_wr(self) -> Self::AsLayout {
        wr::LayoutRect::from_origin_and_size(self.origin.to_wr(), self.size.to_wr())
    }
}
impl WrToPx for wr::LayoutRect {
    type AsPx = PxRect;

    fn to_px(self) -> Self::AsPx {
        self.to_rect().to_px()
    }
}
impl WrToPx for euclid::Rect<f32, wr::LayoutPixel> {
    type AsPx = PxRect;

    fn to_px(self) -> Self::AsPx {
        PxRect::new(self.origin.to_px(), self.size.to_px())
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
impl PxToWr for PxBox {
    type AsDevice = wr::DeviceBox2D;

    type AsLayout = wr::LayoutRect;

    type AsWorld = euclid::Box2D<f32, wr::WorldPixel>;

    fn to_wr_device(self) -> Self::AsDevice {
        wr::DeviceBox2D::new(self.min.to_wr_device().cast(), self.max.to_wr_device().cast())
    }

    fn to_wr_world(self) -> Self::AsWorld {
        euclid::Box2D::new(self.min.to_wr_world(), self.max.to_wr_world())
    }

    fn to_wr(self) -> Self::AsLayout {
        wr::LayoutRect::new(self.min.to_wr(), self.max.to_wr())
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
impl PxToWr for PxSideOffsets {
    type AsDevice = wr::DeviceIntSideOffsets;

    type AsLayout = wr::LayoutSideOffsets;

    type AsWorld = euclid::SideOffsets2D<f32, wr::WorldPixel>;

    fn to_wr_device(self) -> Self::AsDevice {
        wr::DeviceIntSideOffsets::new(
            self.top.to_wr_device().0,
            self.right.to_wr_device().0,
            self.bottom.to_wr_device().0,
            self.left.to_wr_device().0,
        )
    }

    fn to_wr_world(self) -> Self::AsWorld {
        euclid::SideOffsets2D::from_lengths(
            self.top.to_wr_world(),
            self.right.to_wr_world(),
            self.bottom.to_wr_world(),
            self.left.to_wr_world(),
        )
    }

    fn to_wr(self) -> Self::AsLayout {
        wr::LayoutSideOffsets::from_lengths(self.top.to_wr(), self.right.to_wr(), self.bottom.to_wr(), self.left.to_wr())
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
impl PxCornerRadius {
    /// Convert to `webrender` border radius.
    pub fn to_wr(self) -> webrender_api::BorderRadius {
        webrender_api::BorderRadius {
            top_left: self.top_left.to_wr(),
            top_right: self.top_right.to_wr(),
            bottom_left: self.bottom_left.to_wr(),
            bottom_right: self.bottom_right.to_wr(),
        }
    }
}

/// A transform in device pixels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PxTransform {
    /// Simple offset.
    Offset(euclid::Vector2D<f32, Px>),
    /// Full transform.
    #[serde(with = "serde_px_transform3d")]
    Transform(euclid::Transform3D<f32, Px, Px>),
}

impl PartialEq for PxTransform {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Offset(l0), Self::Offset(r0)) => l0 == r0,
            (Self::Transform(l0), Self::Transform(r0)) => l0 == r0,
            (a, b) => a.is_identity() && b.is_identity() || a.to_transform() == b.to_transform(),
        }
    }
}
impl Default for PxTransform {
    /// Identity.
    fn default() -> Self {
        Self::identity()
    }
}
impl PxTransform {
    /// Identity transform.
    pub fn identity() -> Self {
        PxTransform::Offset(euclid::vec2(0.0, 0.0))
    }

    /// New simple 2D translation.
    pub fn translation(x: f32, y: f32) -> Self {
        PxTransform::Offset(euclid::vec2(x, y))
    }

    /// New 3D translation.
    pub fn translation_3d(x: f32, y: f32, z: f32) -> Self {
        PxTransform::Transform(euclid::Transform3D::translation(x, y, z))
    }

    /// New 2D rotation.
    pub fn rotation(x: f32, y: f32, theta: euclid::Angle<f32>) -> Self {
        Self::rotation_3d(x, y, 1.0, theta)
    }

    /// New 3D rotation.
    pub fn rotation_3d(x: f32, y: f32, z: f32, theta: euclid::Angle<f32>) -> Self {
        let [x, y, z] = euclid::vec3::<_, ()>(x, y, z).normalize().to_array();
        PxTransform::Transform(euclid::Transform3D::rotation(x, y, z, theta))
    }

    /// New 2D skew.
    pub fn skew(alpha: euclid::Angle<f32>, beta: euclid::Angle<f32>) -> Self {
        PxTransform::Transform(euclid::Transform3D::skew(alpha, beta))
    }

    /// New 2D scale.
    pub fn scale(x: f32, y: f32) -> Self {
        PxTransform::Transform(euclid::Transform3D::scale(x, y, 1.0))
    }

    /// New 3D scale.
    pub fn scale_3d(x: f32, y: f32, z: f32) -> Self {
        PxTransform::Transform(euclid::Transform3D::scale(x, y, z))
    }

    /// New 3D perspective distance.
    pub fn perspective(d: f32) -> Self {
        PxTransform::Transform(euclid::Transform3D::perspective(d))
    }

    /// To full transform.
    pub fn to_transform(self) -> euclid::Transform3D<f32, Px, Px> {
        match self {
            PxTransform::Offset(v) => euclid::Transform3D::translation(v.x, v.y, 0.0),
            PxTransform::Transform(t) => t,
        }
    }

    /// Returns `true` is is the identity transform.
    pub fn is_identity(&self) -> bool {
        match self {
            PxTransform::Offset(offset) => offset == &euclid::Vector2D::zero(),
            PxTransform::Transform(transform) => transform == &euclid::Transform3D::identity(),
        }
    }

    /// Returns the multiplication of the two matrices such that mat's transformation
    /// applies after self's transformation.
    #[must_use]
    pub fn then(&self, other: &PxTransform) -> PxTransform {
        match (self, other) {
            (PxTransform::Offset(a), PxTransform::Offset(b)) => PxTransform::Offset(*a + *b),
            (PxTransform::Offset(a), PxTransform::Transform(b)) => {
                PxTransform::Transform(euclid::Transform3D::translation(a.x, a.y, 0.0).then(b))
            }
            (PxTransform::Transform(a), PxTransform::Offset(b)) => PxTransform::Transform(a.then_translate(b.to_3d())),
            (PxTransform::Transform(a), PxTransform::Transform(b)) => PxTransform::Transform(a.then(b)),
        }
    }

    /// Returns a transform with a translation applied after self's transformation.
    #[must_use]
    pub fn then_translate(&self, offset: euclid::Vector2D<f32, Px>) -> PxTransform {
        match self {
            PxTransform::Offset(a) => PxTransform::Offset(*a + offset),
            PxTransform::Transform(a) => PxTransform::Transform(a.then_translate(offset.to_3d())),
        }
    }

    /// Returns a transform with a translation applied before self's transformation.
    #[must_use]
    pub fn pre_translate(&self, offset: euclid::Vector2D<f32, Px>) -> PxTransform {
        match self {
            PxTransform::Offset(b) => PxTransform::Offset(offset + *b),
            PxTransform::Transform(b) => PxTransform::Transform(euclid::Transform3D::translation(offset.x, offset.y, 0.0).then(b)),
        }
    }

    /// Returns whether it is possible to compute the inverse transform.
    pub fn is_invertible(&self) -> bool {
        match self {
            PxTransform::Offset(_) => true,
            PxTransform::Transform(t) => t.is_invertible(),
        }
    }

    /// Returns the inverse transform if possible.
    pub fn inverse(&self) -> Option<PxTransform> {
        match self {
            PxTransform::Offset(v) => Some(PxTransform::Offset(-*v)),
            PxTransform::Transform(t) => t.inverse().map(PxTransform::Transform),
        }
    }

    /// Returns `true` if this transform can be represented with a `Transform2D`.
    pub fn is_2d(&self) -> bool {
        match self {
            PxTransform::Offset(_) => true,
            PxTransform::Transform(t) => t.is_2d(),
        }
    }

    /// Transform the pixel point.
    ///
    /// Note that if the transform is 3D the point will be transformed with z=0, you can
    /// use [`project_point`] to find the 2D point in the 3D z-plane represented by the 3D
    /// transform.
    ///
    /// [`project_point`]: Self::project_point
    pub fn transform_point(&self, point: PxPoint) -> Option<PxPoint> {
        self.transform_point_f32(point.cast()).map(|p| p.cast())
    }

    /// Transform the pixel point.
    ///
    /// Note that if the transform is 3D the point will be transformed with z=0, you can
    /// use [`project_point_f32`] to find the 2D point in the 3D z-plane represented by the 3D
    /// transform.
    ///
    /// [`project_point_f32`]: Self::project_point_f32
    pub fn transform_point_f32(&self, point: euclid::Point2D<f32, Px>) -> Option<euclid::Point2D<f32, Px>> {
        match self {
            PxTransform::Offset(v) => Some(point + *v),
            PxTransform::Transform(t) => t.transform_point2d(point),
        }
    }

    /// Transform the pixel vector.
    pub fn transform_vector(&self, vector: PxVector) -> PxVector {
        self.transform_vector_f32(vector.cast()).cast()
    }

    /// Transform the pixel vector.
    pub fn transform_vector_f32(&self, vector: euclid::Vector2D<f32, Px>) -> euclid::Vector2D<f32, Px> {
        match self {
            PxTransform::Offset(v) => vector + *v,
            PxTransform::Transform(t) => t.transform_vector2d(vector),
        }
    }

    /// Project the 2D point onto the transform Z-plane.
    pub fn project_point(&self, point: PxPoint) -> Option<PxPoint> {
        self.project_point_f32(point.cast()).map(|p| p.cast())
    }

    /// Project the 2D point onto the transform Z-plane.
    pub fn project_point_f32(&self, point: euclid::Point2D<f32, Px>) -> Option<euclid::Point2D<f32, Px>> {
        match self {
            PxTransform::Offset(v) => Some(point + *v),
            PxTransform::Transform(t) => {
                // source: https://github.com/servo/webrender/blob/master/webrender/src/util.rs#L1181

                // Find a value for z that will transform to 0.

                // The transformed value of z is computed as:
                // z' = point.x * self.m13 + point.y * self.m23 + z * self.m33 + self.m43

                // Solving for z when z' = 0 gives us:
                let z = -(point.x * t.m13 + point.y * t.m23 + t.m43) / t.m33;

                t.transform_point3d(euclid::point3(point.x, point.y, z))
                    .map(|p3| euclid::point2(p3.x, p3.y))
            }
        }
    }

    /// Returns a 2D box that encompasses the result of transforming the given box by this
    /// transform, if the transform makes sense for it, or `None` otherwise.
    pub fn outer_transformed(&self, px_box: PxBox) -> Option<PxBox> {
        self.outer_transformed_f32(px_box.cast()).map(|p| p.cast())
    }

    /// Returns a 2D box that encompasses the result of transforming the given box by this
    /// transform, if the transform makes sense for it, or `None` otherwise.
    pub fn outer_transformed_f32(&self, px_box: euclid::Box2D<f32, Px>) -> Option<euclid::Box2D<f32, Px>> {
        match self {
            PxTransform::Offset(v) => {
                let v = *v;
                let mut r = px_box;
                r.min += v;
                r.max += v;
                Some(r)
            }
            PxTransform::Transform(t) => t.outer_transformed_box2d(&px_box),
        }
    }
}
impl PxToWr for PxTransform {
    type AsDevice = euclid::Transform3D<f32, wr::DevicePixel, wr::DevicePixel>;

    type AsLayout = wr::LayoutTransform;

    type AsWorld = euclid::Transform3D<f32, wr::WorldPixel, wr::WorldPixel>;

    fn to_wr_device(self) -> Self::AsDevice {
        self.to_transform().with_source().with_destination()
    }

    fn to_wr_world(self) -> Self::AsWorld {
        self.to_transform().with_source().with_destination()
    }

    fn to_wr(self) -> Self::AsLayout {
        self.to_transform().with_source().with_destination()
    }
}
impl From<euclid::Vector2D<f32, Px>> for PxTransform {
    fn from(offset: euclid::Vector2D<f32, Px>) -> Self {
        PxTransform::Offset(offset)
    }
}
impl From<PxVector> for PxTransform {
    fn from(offset: PxVector) -> Self {
        PxTransform::Offset(offset.cast())
    }
}
impl From<euclid::Transform3D<f32, Px, Px>> for PxTransform {
    fn from(transform: euclid::Transform3D<f32, Px, Px>) -> Self {
        PxTransform::Transform(transform)
    }
}
impl From<PxTransform> for wr::LayoutTransform {
    fn from(t: PxTransform) -> Self {
        t.to_wr()
    }
}
/// euclid does skip the _unit
mod serde_px_transform3d {
    use super::*;
    use serde::*;

    #[derive(Serialize, Deserialize)]
    struct SerdeTransform3D {
        pub m11: f32,
        pub m12: f32,
        pub m13: f32,
        pub m14: f32,
        pub m21: f32,
        pub m22: f32,
        pub m23: f32,
        pub m24: f32,
        pub m31: f32,
        pub m32: f32,
        pub m33: f32,
        pub m34: f32,
        pub m41: f32,
        pub m42: f32,
        pub m43: f32,
        pub m44: f32,
    }

    pub fn serialize<S: Serializer>(t: &euclid::Transform3D<f32, Px, Px>, serializer: S) -> Result<S::Ok, S::Error> {
        SerdeTransform3D {
            m11: t.m11,
            m12: t.m12,
            m13: t.m13,
            m14: t.m14,
            m21: t.m21,
            m22: t.m22,
            m23: t.m23,
            m24: t.m24,
            m31: t.m31,
            m32: t.m32,
            m33: t.m33,
            m34: t.m34,
            m41: t.m41,
            m42: t.m42,
            m43: t.m43,
            m44: t.m44,
        }
        .serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<euclid::Transform3D<f32, Px, Px>, D::Error> {
        let t = SerdeTransform3D::deserialize(deserializer)?;
        Ok(euclid::Transform3D {
            m11: t.m11,
            m12: t.m12,
            m13: t.m13,
            m14: t.m14,
            m21: t.m21,
            m22: t.m22,
            m23: t.m23,
            m24: t.m24,
            m31: t.m31,
            m32: t.m32,
            m33: t.m33,
            m34: t.m34,
            m41: t.m41,
            m42: t.m42,
            m43: t.m43,
            m44: t.m44,
            _unit: PhantomData,
        })
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
#[derive(Copy, Clone, serde::Serialize, serde::Deserialize, bytemuck::NoUninit)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Factor(pub f32);
impl Factor {
    /// Clamp factor to `[0.0..=1.0]` range.
    pub fn clamp_range(self) -> Self {
        Factor(self.0.clamp(0.0, 1.0))
    }

    /// Returns the maximum of two factors.
    pub fn max(self, other: impl Into<Factor>) -> Factor {
        Factor(self.0.max(other.into().0))
    }

    /// Returns the minimum of two factors.
    pub fn min(self, other: impl Into<Factor>) -> Factor {
        Factor(self.0.min(other.into().0))
    }

    /// Returns `self` if `min <= self <= max`, returns `min` if `self < min` or returns `max` if `self > max`.
    pub fn clamp(self, min: impl Into<Factor>, max: impl Into<Factor>) -> Factor {
        self.min(max).max(min)
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
        about_eq_hash(self.0, EQ_EPSILON, state)
    }
}
impl PartialEq for Factor {
    fn eq(&self, other: &Self) -> bool {
        about_eq(self.0, other.0, EQ_EPSILON)
    }
}
impl std::cmp::PartialOrd for Factor {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(about_eq_ord(self.0, other.0, EQ_EPSILON))
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
        self *= rhs;
        self
    }
}
impl ops::Div<Factor> for PxPoint {
    type Output = PxPoint;

    fn div(mut self, rhs: Factor) -> PxPoint {
        self /= rhs;
        self
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
        if value {
            Factor(1.0)
        } else {
            Factor(0.0)
        }
    }
}

macro_rules! impl_for_integer {
    ($($T:ty,)+) => {$(
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
impl_for_integer! {
    u8, i8, u16, i16, u32, i32, u64, i64, usize, isize, u128, i128,
}

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

/// [`f32`] equality used in floating-point units.
///
/// * [`NaN`](f32::is_nan) values are equal.
/// * [`INFINITY`](f32::INFINITY) values are equal.
/// * [`NEG_INFINITY`](f32::NEG_INFINITY) values are equal.
/// * Finite values are equal if the difference is less than `epsilon`.
///
/// Note that this definition of equality is symmetric and reflexive, but it is **not** transitive, difference less then
/// epsilon can *accumulate* over a chain of comparisons breaking the transitive property:
///
/// ```
/// # use zero_ui_core::units::about_eq;
/// let e = 0.001;
/// let a = 0.0;
/// let b = a + e - 0.0001;
/// let c = b + e - 0.0001;
///
/// assert!(
///     about_eq(a, b, e) &&
///     about_eq(b, c, e) &&
///     !about_eq(a, c, e)
/// )
/// ```
///
/// See also [`about_eq_hash`].
pub fn about_eq(a: f32, b: f32, epsilon: f32) -> bool {
    if a.is_nan() {
        b.is_nan()
    } else if a.is_infinite() {
        b.is_infinite() && a.is_sign_positive() == b.is_sign_positive()
    } else {
        (a - b).abs() < epsilon
    }
}

/// [`f32`] hash compatible with [`about_eq`] equality.
pub fn about_eq_hash<H: std::hash::Hasher>(f: f32, epsilon: f32, state: &mut H) {
    let (group, f) = if f.is_nan() {
        (0u8, 0u64)
    } else if f.is_infinite() {
        (1, if f.is_sign_positive() { 1 } else { 2 })
    } else {
        let inv_epsi = if epsilon > EQ_EPSILON_100 { 100000.0 } else { 100.0 };
        (2, ((f as f64) * inv_epsi) as u64)
    };

    use std::hash::Hash;
    group.hash(state);
    f.hash(state);
}

/// [`f32`] ordering compatible with [`about_eq`] equality.
pub fn about_eq_ord(a: f32, b: f32, epsilon: f32) -> std::cmp::Ordering {
    if about_eq(a, b, epsilon) {
        std::cmp::Ordering::Equal
    } else if a > b {
        std::cmp::Ordering::Greater
    } else {
        std::cmp::Ordering::Less
    }
}

/// Minimal difference between values in around the 0.0..=1.0 scale.
pub const EQ_EPSILON: f32 = 0.00001;
/// Minimal difference between values in around the 1.0..=100.0 scale.
pub const EQ_EPSILON_100: f32 = 0.001;

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
