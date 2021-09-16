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

use std::{cmp, fmt, ops};

use webrender_api::units as wr;

pub use webrender_api::euclid;

use serde::{Deserialize, Serialize};

/// Same value used in `60`.
const DIP_TO_PX: i32 = 60;

/// Device pixel.
///
/// Represents an actual device pixel, not scaled/descaled by the pixel scale factor.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Px(pub i32);
impl Px {
    /// See [`DipToPx`].
    pub fn from_dip(dip: Dip, scale_factor: f32) -> Px {
        Px((dip.0 as f32 / DIP_TO_PX as f32 * scale_factor).round() as i32)
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
impl fmt::Display for Px {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}px", self.0)
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

/// Device independent pixel.
///
/// Represent a device pixel descaled by the pixel scale factor.
///
/// Internally this is an `i32` that represents 1/60th of a pixel.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Dip(i32);
impl Dip {
    /// New from round integer value.
    pub fn new(dip: i32) -> Self {
        Dip(dip * DIP_TO_PX)
    }

    /// new from floating point.
    pub fn new_f32(dip: f32) -> Self {
        Dip((dip * DIP_TO_PX as f32).round() as i32)
    }

    /// See [`PxToDip`].
    pub fn from_px(px: Px, scale_factor: f32) -> Dip {
        Dip((px.0 as f32 / scale_factor).round() as i32)
    }

    /// Returns `self` as [`f32`].
    pub fn to_f32(self) -> f32 {
        self.0 as f32 / DIP_TO_PX as f32
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
impl fmt::Display for Dip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
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
        Dip(self.0.saturating_mul(rhs.0))
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
        Dip(self.0 / rhs.0)
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
pub type PxPoint = euclid::Point2D<Px, ()>;

/// A point in device independent pixels.
pub type DipPoint = euclid::Point2D<Dip, ()>;

/// A size in device pixels.
pub type PxSize = euclid::Size2D<Px, ()>;

/// A size in device pixels.
pub type DipSize = euclid::Size2D<Dip, ()>;

/// A rectangle in device pixels.
pub type PxRect = euclid::Rect<Px, ()>;

/// A rectangle in device independent pixels.
pub type DipRect = euclid::Rect<Dip, ()>;

/// Side-offsets in device pixels.
pub type PxSideOffsets = euclid::SideOffsets2D<Px, ()>;
/// Side-offsets in device independent pixels.
pub type DipSideOffsets = euclid::SideOffsets2D<Dip, ()>;

/// Ellipses that define the radius of the four corners of a 2D box.
#[derive(Serialize, Deserialize)]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: Deserialize<'de>"))]
pub struct CornerRadius<T, U> {
    /// Top-left corner radius.
    pub top_left: euclid::Size2D<T, U>,
    /// Top-right corner radius.
    pub top_right: euclid::Size2D<T, U>,
    /// Bottom-left corner radius.
    pub bottom_left: euclid::Size2D<T, U>,
    /// Bottom-right corner radius.
    pub bottom_right: euclid::Size2D<T, U>,
}
impl<T: Clone, U> Clone for CornerRadius<T, U> {
    fn clone(&self) -> Self {
        Self {
            top_left: self.top_left.clone(),
            top_right: self.top_right.clone(),
            bottom_left: self.bottom_left.clone(),
            bottom_right: self.bottom_right.clone(),
        }
    }
}
impl<T: Copy, U> Copy for CornerRadius<T, U> {}
impl<T: Copy + num_traits::Zero, U> CornerRadius<T, U> {
    /// New with distinct values.
    pub fn new(
        top_left: euclid::Size2D<T, U>,
        top_right: euclid::Size2D<T, U>,
        bottom_left: euclid::Size2D<T, U>,
        bottom_right: euclid::Size2D<T, U>,
    ) -> Self {
        Self {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
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
}
impl<T: fmt::Debug, U> fmt::Debug for CornerRadius<T, U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CornerRadius")
            .field("top_left", &self.top_left)
            .field("top_right", &self.top_right)
            .field("bottom_left", &self.bottom_left)
            .field("bottom_right", &self.bottom_right)
            .finish()
    }
}

/// Corner-radius in device pixels.
pub type PxCornerRadius = CornerRadius<Px, ()>;

/// Corner-radius in device independent pixels.
pub type DipCornerRadius = CornerRadius<Dip, ()>;

pub use wr::LayoutTransform;

/// Conversion from [`Px`] to [`Dip`] units.
pub trait PxToDip {
    /// `Self` equivalent in [`Dip`] units.
    type AsDip;

    /// Divide the [`Px`] self by the scale.
    fn to_dip(self, scale_factor: f32) -> Self::AsDip;
}

/// Conversion from [`Dip`] to [`Px`] units.
pub trait DipToPx {
    /// `Self` equivalent in [`Px`] units.
    type AsPx;

    /// Multiply the [`Dip`] self by the scale.
    fn to_px(self, scale_factor: f32) -> Self::AsPx;
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

/// Conversion from `winit` logical units to [`Dip`].
///
/// All conversions are 1 to 1.
#[cfg(feature = "full")]
pub(crate) trait WinitToDip {
    /// `Self` equivalent in [`Dip`] units.
    type AsDip;

    /// Returns `self` in [`Dip`] units.
    fn to_dip(self) -> Self::AsDip;
}

/// Conversion from `winit` physical units to [`Dip`].
///
/// All conversions are 1 to 1.
#[cfg(feature = "full")]
pub(crate) trait WinitToPx {
    /// `Self` equivalent in [`Px`] units.
    type AsPx;

    /// Returns `self` in [`Px`] units.
    fn to_px(self) -> Self::AsPx;
}

/// Conversion from [`Dip`] to `winit` logical units.
#[cfg(feature = "full")]
pub(crate) trait DipToWinit {
    /// `Self` equivalent in `winit` logical units.
    type AsWinit;

    /// Returns `self` in `winit` logical units.
    fn to_winit(self) -> Self::AsWinit;
}

impl PxToDip for Px {
    type AsDip = Dip;

    fn to_dip(self, scale_factor: f32) -> Self::AsDip {
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

    fn to_px(self, scale_factor: f32) -> Self::AsPx {
        Px::from_dip(self, scale_factor)
    }
}

impl PxToDip for PxPoint {
    type AsDip = DipPoint;

    fn to_dip(self, scale_factor: f32) -> Self::AsDip {
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

    fn to_px(self, scale_factor: f32) -> Self::AsPx {
        PxPoint::new(self.x.to_px(scale_factor), self.y.to_px(scale_factor))
    }
}
#[cfg(feature = "full")]
impl DipToWinit for DipPoint {
    type AsWinit = glutin::dpi::LogicalPosition<f32>;

    fn to_winit(self) -> Self::AsWinit {
        glutin::dpi::LogicalPosition::new(self.x.to_f32(), self.y.to_f32())
    }
}
#[cfg(feature = "full")]
impl WinitToDip for glutin::dpi::LogicalPosition<f64> {
    type AsDip = DipPoint;

    fn to_dip(self) -> Self::AsDip {
        DipPoint::new(Dip::new_f32(self.x as f32), Dip::new_f32(self.y as f32))
    }
}
#[cfg(feature = "full")]
impl WinitToPx for glutin::dpi::PhysicalPosition<i32> {
    type AsPx = PxPoint;

    fn to_px(self) -> Self::AsPx {
        PxPoint::new(Px(self.x), Px(self.y))
    }
}
#[cfg(feature = "full")]
impl WinitToPx for glutin::dpi::PhysicalPosition<f64> {
    type AsPx = PxPoint;

    fn to_px(self) -> Self::AsPx {
        PxPoint::new(Px(self.x as i32), Px(self.y as i32))
    }
}

impl PxToDip for PxSize {
    type AsDip = DipSize;

    fn to_dip(self, scale_factor: f32) -> Self::AsDip {
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
impl DipToPx for DipSize {
    type AsPx = PxSize;

    fn to_px(self, scale_factor: f32) -> Self::AsPx {
        PxSize::new(self.width.to_px(scale_factor), self.height.to_px(scale_factor))
    }
}
#[cfg(feature = "full")]
impl DipToWinit for DipSize {
    type AsWinit = glutin::dpi::LogicalSize<f32>;

    fn to_winit(self) -> Self::AsWinit {
        glutin::dpi::LogicalSize::new(self.width.to_f32(), self.height.to_f32())
    }
}
#[cfg(feature = "full")]
impl WinitToDip for glutin::dpi::LogicalSize<f64> {
    type AsDip = DipSize;

    fn to_dip(self) -> Self::AsDip {
        DipSize::new(Dip::new_f32(self.width as f32), Dip::new_f32(self.height as f32))
    }
}
#[cfg(feature = "full")]
impl WinitToPx for glutin::dpi::PhysicalSize<u32> {
    type AsPx = PxSize;

    fn to_px(self) -> Self::AsPx {
        PxSize::new(Px(self.width as i32), Px(self.height as i32))
    }
}

impl PxToDip for PxRect {
    type AsDip = DipRect;

    fn to_dip(self, scale_factor: f32) -> Self::AsDip {
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
impl DipToPx for DipRect {
    type AsPx = PxRect;

    fn to_px(self, scale_factor: f32) -> Self::AsPx {
        PxRect::new(self.origin.to_px(scale_factor), self.size.to_px(scale_factor))
    }
}

impl DipToPx for DipSideOffsets {
    type AsPx = PxSideOffsets;

    fn to_px(self, scale_factor: f32) -> Self::AsPx {
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

    fn to_dip(self, scale_factor: f32) -> Self::AsDip {
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

    fn to_px(self, scale_factor: f32) -> Self::AsPx {
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

    fn to_dip(self, scale_factor: f32) -> Self::AsDip {
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
    pub fn to_border_radius(self) -> webrender_api::BorderRadius {
        webrender_api::BorderRadius {
            top_left: self.top_left.to_wr(),
            top_right: self.top_right.to_wr(),
            bottom_left: self.bottom_left.to_wr(),
            bottom_right: self.bottom_right.to_wr(),
        }
    }
}
