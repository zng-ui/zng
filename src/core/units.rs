//! Angle, factor and length units.

use derive_more as dm;
use std::f32::consts::*;

const TAU: f32 = 2.0 * PI;

/// Angle in radians.
#[derive(Debug, dm::Display, Copy, Clone, dm::Add, dm::AddAssign, dm::Sub, dm::SubAssign, PartialEq)]
#[display(fmt = "{} rad", self.0)]
pub struct AngleRadian(f32);
impl AngleRadian {
    /// Radians in `[0.0 ..= TAU]`.
    #[inline]
    pub fn modulo(self) -> Self {
        AngleGradian::from(self).modulo().into()
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

/// Angle in gradians.
#[derive(Debug, dm::Display, Copy, Clone, dm::Add, dm::AddAssign, dm::Sub, dm::SubAssign, PartialEq)]
#[display(fmt = "{} gon", self.0)]
pub struct AngleGradian(pub f32);
impl AngleGradian {
    /// Gradians in `[0.0 ..= 400.0]`.
    #[inline]
    pub fn modulo(self) -> Self {
        AngleGradian(self.0.rem_euclid(400.0))
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

/// Angle in degrees.
#[derive(Debug, dm::Display, Copy, Clone, dm::Add, dm::AddAssign, dm::Sub, dm::SubAssign, PartialEq)]
#[display(fmt = "{}º", self.0)]
pub struct AngleDegree(pub f32);
impl AngleDegree {
    /// Degrees in `[0.0 ..= 360.0]`.
    #[inline]
    pub fn modulo(self) -> Self {
        AngleDegree(self.0.rem_euclid(360.0))
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

/// Angle in turns (complete rotations)
#[derive(Debug, dm::Display, Copy, Clone, dm::Add, dm::AddAssign, dm::Sub, dm::SubAssign, PartialEq)]
#[display(fmt = "{} tr", self.0)]
pub struct AngleTurn(pub f32);
impl AngleTurn {
    /// Turns in `[0.0 ..= 1.0]`.
    #[inline]
    pub fn modulo(self) -> Self {
        AngleTurn(self.0.rem_euclid(1.0))
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

/// Extension methods for creating angle unit types.
///
/// This trait is implemented for [`f32`] and [`u32`] allowing initialization of angle unit types using the `<number>.<unit>()` syntax.
///
/// # Example
///
/// ```
/// # use zero_ui::core::units::*;
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
    #[inline]
    fn rad(self) -> AngleRadian {
        AngleRadian(self)
    }

    #[inline]
    fn grad(self) -> AngleGradian {
        AngleGradian(self)
    }

    #[inline]
    fn deg(self) -> AngleDegree {
        AngleDegree(self)
    }

    #[inline]
    fn turn(self) -> AngleTurn {
        AngleTurn(self)
    }
}
impl AngleUnits for u32 {
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

/// Multiplication factor in percentage (0%-100%).
#[derive(Debug, dm::Display, Copy, Clone, dm::Add, dm::AddAssign, dm::Sub, dm::SubAssign, PartialEq)]
#[display(fmt = "{}%", self.0)]
pub struct FactorPercent(pub f32);

impl From<FactorNormal> for FactorPercent {
    fn from(n: FactorNormal) -> Self {
        FactorPercent(n.0 * 100.0)
    }
}

/// Normalized multiplication factor (0.0-1.0).
#[derive(Debug, dm::Display, Copy, Clone, dm::Add, dm::AddAssign, dm::Sub, dm::SubAssign, PartialEq, dm::From)]
pub struct FactorNormal(pub f32);

impl From<FactorPercent> for FactorNormal {
    fn from(percent: FactorPercent) -> Self {
        FactorNormal(percent.0 / 100.0)
    }
}

/// Extension methods for factor unit types.
///
/// This trait is implemented for [`f32`] and [`u32`] allowing initialization of factor unit types using the `<number>.<unit>()` syntax.
///
/// # Example
///
/// ```
/// # use zero_ui::core::units::*;
/// let percent = 100.pct();
/// ```
pub trait FactorUnits {
    /// Percent.
    fn pct(self) -> FactorPercent;

    /// Normal.
    ///
    /// # Note
    ///
    /// [`FactorNormal`] implements `From<f32>`.
    fn normal(self) -> FactorNormal;
}

impl FactorUnits for f32 {
    #[inline]
    fn pct(self) -> FactorPercent {
        FactorPercent(self)
    }

    #[inline]
    fn normal(self) -> FactorNormal {
        self.into()
    }
}
impl FactorUnits for u32 {
    #[inline]
    fn pct(self) -> FactorPercent {
        FactorPercent(self as f32)
    }

    #[inline]
    fn normal(self) -> FactorNormal {
        FactorNormal(self as f32)
    }
}

pub enum Length {
    /// The exact length.
    Exact(f32),
    /// Relative to the available size.
    Relative(FactorNormal),
    /// Relative to the font-size of the widget.
    Em(FactorNormal),
    /// Relative to the font-size of the root widget.
    RootEm(FactorNormal),
    /// Relative to 1% of the width of the viewport.
    ViewportWidth(f32),
    /// Relative to 1% of the height of the viewport.
    ViewportHeight(f32),
    /// Relative to 1% of the smallest of the viewport's dimensions.
    ViewportMin(f32),
    /// Relative to 1% of the largest of the viewport's dimensions.
    ViewportMax(f32),
}
impl From<FactorPercent> for Length {
    /// Conversion to [`Length::Relative`]
    fn from(percent: FactorPercent) -> Self {
        Length::Relative(percent.into())
    }
}
impl From<f32> for Length {
    fn from(f: f32) -> Self {
        Length::Exact(f)
    }
}
impl From<u32> for Length {
    fn from(u: u32) -> Self {
        Length::Exact(u as f32)
    }
}

pub trait LengthUnits {
    fn em(self) -> Length;
    fn rem(self) -> Length;
    fn vw(self) -> Length;
    fn vh(self) -> Length;
    fn vmin(self) -> Length;
    fn vmax(self) -> Length;
}
impl LengthUnits for f32 {
    #[inline]
    fn em(self) -> Length {
        Length::Em(self.into())
    }
    #[inline]
    fn rem(self) -> Length {
        Length::RootEm(self.into())
    }
    #[inline]
    fn vw(self) -> Length {
        Length::ViewportWidth(self)
    }
    #[inline]
    fn vh(self) -> Length {
        Length::ViewportHeight(self)
    }
    #[inline]
    fn vmin(self) -> Length {
        Length::ViewportMin(self)
    }
    #[inline]
    fn vmax(self) -> Length {
        Length::ViewportMax(self)
    }
}
impl LengthUnits for u32 {
    #[inline]
    fn em(self) -> Length {
        Length::Em(self.normal())
    }
    #[inline]
    fn rem(self) -> Length {
        Length::RootEm(self.normal())
    }
    #[inline]
    fn vw(self) -> Length {
        Length::ViewportWidth(self as f32)
    }
    #[inline]
    fn vh(self) -> Length {
        Length::ViewportHeight(self as f32)
    }
    #[inline]
    fn vmin(self) -> Length {
        Length::ViewportMin(self as f32)
    }
    #[inline]
    fn vmax(self) -> Length {
        Length::ViewportMax(self as f32)
    }
}

pub struct Size {
    pub width: Length,
    pub height: Length,
}
impl Size {
    pub fn new<W: Into<Length>, H: Into<Length>>(width: W, height: H) -> Self {
        Size {
            width: width.into(),
            height: height.into(),
        }
    }
}

pub struct Point {
    pub x: Length,
    pub y: Length,
}
impl Point {
    pub fn new<X: Into<Length>, Y: Into<Length>>(x: X, y: Y) -> Self {
        Point { x: x.into(), y: y.into() }
    }
}

pub struct Rect {
    pub origin: Point,
    pub size: Size,
}
impl Rect {
    pub fn new<O: Into<Point>, S: Into<Size>>(origin: O, size: S) -> Self {
        Rect {
            origin: origin.into(),
            size: size.into(),
        }
    }
}

pub struct SideOffsets {
    pub top: Length,
    pub right: Length,
    pub bottom: Length,
    pub left: Length,
}
impl SideOffsets {
    pub fn new<T: Into<Length>, R: Into<Length>, B: Into<Length>, L: Into<Length>>(top: T, right: R, bottom: B, left: L) -> Self {
        SideOffsets {
            top: top.into(),
            right: right.into(),
            bottom: bottom.into(),
            left: left.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn zero() {
        all_equal(0.rad(), 0.grad(), 0.deg(), 0.turn());
    }

    #[test]
    pub fn half_circle() {
        all_equal(PI.rad(), 200.grad(), 180.deg(), 0.5.turn())
    }

    #[test]
    pub fn full_circle() {
        all_equal(TAU.rad(), 400.grad(), 360.deg(), 1.turn())
    }

    #[test]
    pub fn one_and_a_half_circle() {
        all_equal((TAU + PI).rad(), 600.grad(), 540.deg(), 1.5.turn())
    }

    #[test]
    pub fn modulo_rad() {
        assert_eq!(PI.rad(), (TAU + PI).rad().modulo());
    }

    #[test]
    pub fn modulo_grad() {
        assert_eq!(200.grad(), 600.grad().modulo());
    }

    #[test]
    pub fn modulo_deg() {
        assert_eq!(180.deg(), 540.deg().modulo());
    }

    #[test]
    pub fn modulo_turn() {
        assert_eq!(0.5.turn(), 1.5.turn().modulo());
    }

    fn all_equal(rad: AngleRadian, grad: AngleGradian, deg: AngleDegree, turn: AngleTurn) {
        assert_eq!(rad, AngleRadian::from(grad));
        assert_eq!(rad, AngleRadian::from(deg));
        assert_eq!(rad, AngleRadian::from(turn));

        assert_eq!(grad, AngleGradian::from(rad));
        assert_eq!(grad, AngleGradian::from(deg));
        assert_eq!(grad, AngleGradian::from(turn));

        assert_eq!(deg, AngleDegree::from(rad));
        assert_eq!(deg, AngleDegree::from(grad));
        assert_eq!(deg, AngleDegree::from(turn));

        assert_eq!(turn, AngleTurn::from(rad));
        assert_eq!(turn, AngleTurn::from(grad));
        assert_eq!(turn, AngleTurn::from(deg));
    }
}
