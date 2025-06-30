use crate::about_eq_hash;

use super::{EQ_GRANULARITY, EQ_GRANULARITY_100, Factor, about_eq};

use std::{
    f32::consts::{PI, TAU},
    fmt, ops,
};

/// Angle in radians.
///
/// See [`AngleUnits`] for more details.
///
/// # Equality
///
/// Equality is determined using [`about_eq`] with `0.00001` granularity.
#[derive(Copy, Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct AngleRadian(pub f32);
impl ops::Add for AngleRadian {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl ops::AddAssign for AngleRadian {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}
impl ops::Sub for AngleRadian {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}
impl ops::SubAssign for AngleRadian {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}
impl ops::Neg for AngleRadian {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}
impl AngleRadian {
    /// Radians in `[0.0 ..= TAU]`.
    pub fn modulo(self) -> Self {
        AngleRadian(self.0.rem_euclid(TAU))
    }

    /// Linear interpolation.
    pub fn lerp(self, to: Self, factor: Factor) -> Self {
        Self(lerp(self.0, to.0, factor))
    }

    /// Spherical linear interpolation.
    ///
    /// Always uses the shortest path from `self` to `to`.
    ///
    /// The [`lerp`] linear interpolation always covers the numeric range between angles, so a transition from 358º to 1º
    /// iterates over almost a full counterclockwise turn to reach the final value, `slerp` simply goes from 358º to 361º modulo
    /// normalized.
    ///
    /// [`lerp`]: Self::lerp
    pub fn slerp(self, to: Self, factor: Factor) -> Self {
        Self(slerp(self.0, to.0, TAU, factor))
    }
}

impl PartialEq for AngleRadian {
    fn eq(&self, other: &Self) -> bool {
        about_eq(self.0, other.0, EQ_GRANULARITY)
    }
}
impl Eq for AngleRadian {}
impl std::hash::Hash for AngleRadian {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        about_eq_hash(self.0, EQ_GRANULARITY, state);
    }
}

impl From<AngleGradian> for AngleRadian {
    fn from(grad: AngleGradian) -> Self {
        AngleRadian(grad.0 * PI / 200.0)
    }
}
impl From<AngleDegree> for AngleRadian {
    fn from(deg: AngleDegree) -> Self {
        AngleRadian(deg.0.to_radians())
    }
}
impl From<AngleTurn> for AngleRadian {
    fn from(turn: AngleTurn) -> Self {
        AngleRadian(turn.0 * TAU)
    }
}
impl From<AngleRadian> for euclid::Angle<f32> {
    fn from(rad: AngleRadian) -> Self {
        euclid::Angle::radians(rad.0)
    }
}

impl fmt::Debug for AngleRadian {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_tuple("AngleRadian").field(&self.0).finish()
        } else {
            write!(f, "{}.rad()", self.0)
        }
    }
}
impl fmt::Display for AngleRadian {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} rad", self.0)
    }
}

/// Angle in gradians.
///
/// See [`AngleUnits`] for more details.
///
/// # Equality
///
/// Equality is determined using [`about_eq`] with `0.001` granularity.
#[derive(Copy, Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct AngleGradian(pub f32);
impl AngleGradian {
    /// Gradians in `[0.0 ..= 400.0]`.
    pub fn modulo(self) -> Self {
        AngleGradian(self.0.rem_euclid(400.0))
    }

    /// Linear interpolation.
    pub fn lerp(self, to: Self, factor: Factor) -> Self {
        Self(lerp(self.0, to.0, factor))
    }

    /// Spherical linear interpolation.
    ///
    /// Always uses the shortest path from `self` to `to`.
    ///
    /// The [`lerp`] linear interpolation always covers the numeric range between angles, so a transition from 358º to 1º
    /// iterates over almost a full counterclockwise turn to reach the final value, `slerp` simply goes from 358º to 361º modulo
    /// normalized.
    ///
    /// [`lerp`]: Self::lerp
    pub fn slerp(self, to: Self, factor: Factor) -> Self {
        Self(slerp(self.0, to.0, 400.0, factor))
    }
}
impl ops::Add for AngleGradian {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl ops::AddAssign for AngleGradian {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}
impl ops::Sub for AngleGradian {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}
impl ops::SubAssign for AngleGradian {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}
impl ops::Neg for AngleGradian {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl PartialEq for AngleGradian {
    fn eq(&self, other: &Self) -> bool {
        about_eq(self.0, other.0, EQ_GRANULARITY_100)
    }
}
impl Eq for AngleGradian {}
impl std::hash::Hash for AngleGradian {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        about_eq_hash(self.0, EQ_GRANULARITY_100, state);
    }
}
impl From<AngleRadian> for AngleGradian {
    fn from(rad: AngleRadian) -> Self {
        AngleGradian(rad.0 * 200.0 / PI)
    }
}
impl From<AngleDegree> for AngleGradian {
    fn from(deg: AngleDegree) -> Self {
        AngleGradian(deg.0 * 10.0 / 9.0)
    }
}
impl From<AngleTurn> for AngleGradian {
    fn from(turn: AngleTurn) -> Self {
        AngleGradian(turn.0 * 400.0)
    }
}
impl fmt::Debug for AngleGradian {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_tuple("AngleGradian").field(&self.0).finish()
        } else {
            write!(f, "{}.grad()", self.0)
        }
    }
}
impl fmt::Display for AngleGradian {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} gon", self.0)
    }
}

/// Angle in degrees.
///
/// See [`AngleUnits`] for more details.
///
/// # Equality
///
/// Equality is determined using [`about_eq`] with `0.001` granularity.
#[derive(Copy, Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct AngleDegree(pub f32);
impl AngleDegree {
    /// Degrees in `[0.0 ..= 360.0]`.
    pub fn modulo(self) -> Self {
        AngleDegree(self.0.rem_euclid(360.0))
    }

    /// Linear interpolation.
    pub fn lerp(self, to: Self, factor: Factor) -> Self {
        Self(lerp(self.0, to.0, factor))
    }

    /// Spherical linear interpolation.
    ///
    /// Always uses the shortest path from `self` to `to`.
    ///
    /// The [`lerp`] linear interpolation always covers the numeric range between angles, so a transition from 358º to 1º
    /// iterates over almost a full counterclockwise turn to reach the final value, `slerp` simply goes from 358º to 361º modulo
    /// normalized.
    ///
    /// [`lerp`]: Self::lerp
    pub fn slerp(self, to: Self, factor: Factor) -> Self {
        Self(slerp(self.0, to.0, 360.0, factor))
    }
}
impl ops::Add for AngleDegree {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl ops::AddAssign for AngleDegree {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}
impl ops::Sub for AngleDegree {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}
impl ops::SubAssign for AngleDegree {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}
impl ops::Neg for AngleDegree {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl PartialEq for AngleDegree {
    fn eq(&self, other: &Self) -> bool {
        about_eq(self.0, other.0, EQ_GRANULARITY_100)
    }
}
impl Eq for AngleDegree {}
impl std::hash::Hash for AngleDegree {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        about_eq_hash(self.0, EQ_GRANULARITY_100, state);
    }
}
impl From<AngleRadian> for AngleDegree {
    fn from(rad: AngleRadian) -> Self {
        AngleDegree(rad.0.to_degrees())
    }
}
impl From<AngleGradian> for AngleDegree {
    fn from(grad: AngleGradian) -> Self {
        AngleDegree(grad.0 * 9.0 / 10.0)
    }
}
impl From<AngleTurn> for AngleDegree {
    fn from(turn: AngleTurn) -> Self {
        AngleDegree(turn.0 * 360.0)
    }
}
impl fmt::Debug for AngleDegree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_tuple("AngleDegree").field(&self.0).finish()
        } else {
            write!(f, "{}.deg()", self.0)
        }
    }
}
impl fmt::Display for AngleDegree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}º", self.0)
    }
}

/// Angle in turns (complete rotations).
///
/// See [`AngleUnits`] for more details.
///
/// # Equality
///
/// Equality is determined using [`about_eq`] with `0.00001` granularity.
#[derive(Copy, Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct AngleTurn(pub f32);
impl AngleTurn {
    /// Turns in `[0.0 ..= 1.0]`.
    pub fn modulo(self) -> Self {
        AngleTurn(self.0.rem_euclid(1.0))
    }

    /// Linear interpolation.
    pub fn lerp(self, to: Self, factor: Factor) -> Self {
        Self(lerp(self.0, to.0, factor))
    }

    /// Spherical linear interpolation.
    ///
    /// Always uses the shortest path from `self` to `to`.
    ///
    /// The [`lerp`] linear interpolation always covers the numeric range between angles, so a transition from 358º to 1º
    /// iterates over almost a full counterclockwise turn to reach the final value, `slerp` simply goes from 358º to 361º modulo
    /// normalized.
    ///
    /// [`lerp`]: Self::lerp
    pub fn slerp(self, to: Self, factor: Factor) -> Self {
        Self(slerp(self.0, to.0, 1.0, factor))
    }
}
impl ops::Add for AngleTurn {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl ops::AddAssign for AngleTurn {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}
impl ops::Sub for AngleTurn {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}
impl ops::SubAssign for AngleTurn {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}
impl ops::Neg for AngleTurn {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl fmt::Debug for AngleTurn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_tuple("AngleTurn").field(&self.0).finish()
        } else {
            write!(f, "{}.turn()", self.0)
        }
    }
}
impl fmt::Display for AngleTurn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if (self.0 - 1.0).abs() < 0.0001 {
            write!(f, "1 turn")
        } else {
            write!(f, "{} turns", self.0)
        }
    }
}
impl PartialEq for AngleTurn {
    fn eq(&self, other: &Self) -> bool {
        about_eq(self.0, other.0, EQ_GRANULARITY)
    }
}
impl Eq for AngleTurn {}
impl std::hash::Hash for AngleTurn {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        about_eq_hash(self.0, EQ_GRANULARITY, state);
    }
}

impl From<AngleRadian> for AngleTurn {
    fn from(rad: AngleRadian) -> Self {
        AngleTurn(rad.0 / TAU)
    }
}
impl From<AngleGradian> for AngleTurn {
    fn from(grad: AngleGradian) -> Self {
        AngleTurn(grad.0 / 400.0)
    }
}
impl From<AngleDegree> for AngleTurn {
    fn from(deg: AngleDegree) -> Self {
        AngleTurn(deg.0 / 360.0)
    }
}

/// Extension methods for initializing angle units.
///
/// This trait is implemented for [`f32`] and [`u32`] allowing initialization of angle unit types using the `<number>.<unit>()` syntax.
///
/// # Examples
///
/// ```
/// # use zng_unit::*;
/// let radians = 6.28318.rad();
/// let gradians = 400.grad();
/// let degrees = 360.deg();
/// let turns = 1.turn();
/// ```
pub trait AngleUnits {
    /// Radians
    fn rad(self) -> AngleRadian;
    /// Gradians
    fn grad(self) -> AngleGradian;
    /// Degrees
    fn deg(self) -> AngleDegree;
    /// Turns
    fn turn(self) -> AngleTurn;
}
impl AngleUnits for f32 {
    fn rad(self) -> AngleRadian {
        AngleRadian(self)
    }

    fn grad(self) -> AngleGradian {
        AngleGradian(self)
    }

    fn deg(self) -> AngleDegree {
        AngleDegree(self)
    }

    fn turn(self) -> AngleTurn {
        AngleTurn(self)
    }
}
impl AngleUnits for i32 {
    fn rad(self) -> AngleRadian {
        AngleRadian(self as f32)
    }

    fn grad(self) -> AngleGradian {
        AngleGradian(self as f32)
    }

    fn deg(self) -> AngleDegree {
        AngleDegree(self as f32)
    }

    fn turn(self) -> AngleTurn {
        AngleTurn(self as f32)
    }
}

fn lerp(from: f32, to: f32, factor: Factor) -> f32 {
    from + (to - from) * factor.0
}

fn slerp(from: f32, to: f32, turn: f32, factor: Factor) -> f32 {
    let angle_to = {
        let d = (to - from) % turn;
        2.0 * d % turn - d
    };
    from + angle_to * factor.0
}
