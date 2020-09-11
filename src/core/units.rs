//! Angle, factor and length units.

use derive_more as dm;
use std::f32::consts::*;

use super::context::LayoutContext;
use crate::core::var::{IntoVar, OwnedVar};

const TAU: f32 = 2.0 * PI;

/// Angle in radians.
///
/// See [`AngleUnits`] for more details.
#[derive(Debug, dm::Display, Copy, Clone, dm::Add, dm::AddAssign, dm::Sub, dm::SubAssign, PartialEq)]
#[display(fmt = "{} rad", self.0)]
pub struct AngleRadian(pub f32);
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
///
/// See [`AngleUnits`] for more details.
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
///
/// See [`AngleUnits`] for more details.
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

/// Angle in turns (complete rotations).
///
/// See [`AngleUnits`] for more details.
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

/// Extension methods for initializing angle units.
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
///
/// See [`FactorUnits`] for more details.
#[derive(Debug, dm::Display, Copy, Clone, dm::Add, dm::AddAssign, dm::Sub, dm::SubAssign, PartialEq)]
#[display(fmt = "{}%", self.0)]
pub struct FactorPercent(pub f32);
impl FactorPercent {
    /// Clamp factor to [0.0..=100.0] range.
    #[inline]
    pub fn clamp_range(self) -> Self {
        FactorPercent(self.0.max(0.0).min(100.0))
    }
}
impl From<FactorNormal> for FactorPercent {
    fn from(n: FactorNormal) -> Self {
        FactorPercent(n.0 * 100.0)
    }
}

/// Normalized multiplication factor (0.0-1.0).
///
/// See [`FactorUnits`] for more details.
#[derive(Debug, dm::Display, Copy, Clone, dm::Add, dm::AddAssign, dm::Sub, dm::SubAssign, PartialEq, dm::From)]
pub struct FactorNormal(pub f32);
impl FactorNormal {
    /// Clamp factor to [0.0..=1.0] range.
    #[inline]
    pub fn clamp_range(self) -> Self {
        FactorNormal(self.0.max(0.0).min(1.0))
    }
}
impl From<FactorPercent> for FactorNormal {
    fn from(percent: FactorPercent) -> Self {
        FactorNormal(percent.0 / 100.0)
    }
}

/// Extension methods for initializing factor units.
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

/// 1D length units.
///
/// See [`LengthUnits`] for more details.
#[derive(Copy, Clone, Debug)]
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
impl Length {
    #[inline]
    pub fn zero() -> Length {
        Length::Exact(0.0)
    }

    /// Length that fills the available space.
    #[inline]
    pub fn fill() -> Length {
        Length::Relative(FactorNormal(1.0))
    }

    /// Compute the length at a context.
    pub fn to_layout(self, available_size: LayoutLength, ctx: &LayoutContext) -> LayoutLength {
        let l = match self {
            Length::Exact(l) => l,
            Length::Relative(s) => available_size.get() * s.0,
            Length::Em(s) => ctx.font_size() * s.0,
            Length::RootEm(s) => ctx.root_font_size() * s.0,
            Length::ViewportWidth(p) => p * ctx.viewport_size().width / 100.0,
            Length::ViewportHeight(p) => p * ctx.viewport_size().height / 100.0,
            Length::ViewportMin(p) => p * ctx.viewport_min() / 100.0,
            Length::ViewportMax(p) => p * ctx.viewport_max() / 100.0,
        };
        LayoutLength::new(ctx.pixel_grid().snap(l))
    }
}

/// Computed [`Length`].
pub type LayoutLength = euclid::Length<f32, webrender::api::units::LayoutPixel>;

/// Extension methods for initializing [`Length`] units.
///
/// This trait is implemented for [`f32`] and [`u32`] allowing initialization of length units using the `<number>.<unit>()` syntax.
///
/// # Example
///
/// ```
/// # use zero_ui::core::units::*;
/// let font_size = 1.em();
/// let root_font_size = 1.rem();
/// let viewport_width = 100.vw();
/// let viewport_height = 100.vh();
/// let viewport_min = 100.vmin();// min(width, height)
/// let viewport_max = 100.vmax();// max(width, height)
///
/// // other length units not provided by `LengthUnits`:
///
/// let exact_size: Length = 500.into();
/// let available_size: Length = 100.pct().into();// FactorUnits
/// let available_size: Length = 1.0.normal().into();// FactorUnits
/// ```
pub trait LengthUnits {
    /// Relative to the font-size of the widget.
    ///
    /// Returns [`Length::Em`].
    fn em(self) -> Length;
    /// Relative to the font-size of the root widget.
    ///
    /// Returns [`Length::RootEm`].
    fn rem(self) -> Length;

    /// Relative to 1% of the width of the viewport.
    ///
    /// Returns [`Length::ViewportWidth`].
    fn vw(self) -> Length;
    /// Relative to 1% of the height of the viewport.
    ///
    /// Returns [`Length::ViewportHeight`].
    fn vh(self) -> Length;

    /// Relative to 1% of the smallest of the viewport's dimensions.
    ///
    /// Returns [`Length::ViewportMin`].
    fn vmin(self) -> Length;
    /// Relative to 1% of the largest of the viewport's dimensions.
    ///
    /// Returns [`Length::ViewportMax`].
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

/// 2D point in [`Length`] units.
#[derive(Copy, Clone, Debug)]
pub struct Point {
    pub x: Length,
    pub y: Length,
}
impl Point {
    pub fn new<X: Into<Length>, Y: Into<Length>>(x: X, y: Y) -> Self {
        Point { x: x.into(), y: y.into() }
    }

    #[inline]
    pub fn zero() -> Self {
        Self::new(Length::zero(), Length::zero())
    }

    /// Swap `x` and `y`.
    #[inline]
    pub fn yx(self) -> Self {
        Point { y: self.x, x: self.y }
    }

    #[inline]
    pub fn to_tuple(self) -> (Length, Length) {
        (self.x, self.y)
    }

    /// Compute the point in a context.
    #[inline]
    pub fn to_layout(self, available_size: LayoutSize, ctx: &LayoutContext) -> LayoutPoint {
        LayoutPoint::from_lengths(
            self.x.to_layout(LayoutLength::new(available_size.width), ctx),
            self.y.to_layout(LayoutLength::new(available_size.height), ctx),
        )
    }
}
impl<X: Into<Length>, Y: Into<Length>> From<(X, Y)> for Point {
    fn from(t: (X, Y)) -> Self {
        Point::new(t.0, t.1)
    }
}
impl<X: Into<Length> + Clone, Y: Into<Length> + Clone> IntoVar<Point> for (X, Y) {
    type Var = OwnedVar<Point>;

    fn into_var(self) -> Self::Var {
        OwnedVar(self.into())
    }
}

/// Computed [`Point`].
pub type LayoutPoint = webrender::api::units::LayoutPoint;

/// 2D size in [`Length`] units.
#[derive(Copy, Clone, Debug)]
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

    #[inline]
    pub fn zero() -> Self {
        Self::new(Length::zero(), Length::zero())
    }

    /// Size that fills the available space.
    #[inline]
    pub fn fill() -> Self {
        Self::new(Length::fill(), Length::fill())
    }

    #[inline]
    pub fn to_tuple(self) -> (Length, Length) {
        (self.width, self.height)
    }

    #[inline]
    pub fn to_layout(self, available_size: LayoutSize, ctx: &LayoutContext) -> LayoutSize {
        LayoutSize::from_lengths(
            self.width.to_layout(LayoutLength::new(available_size.width), ctx),
            self.height.to_layout(LayoutLength::new(available_size.height), ctx),
        )
    }
}
impl<W: Into<Length>, H: Into<Length>> From<(W, H)> for Size {
    fn from(t: (W, H)) -> Self {
        Size::new(t.0, t.1)
    }
}
impl<W: Into<Length> + Clone, H: Into<Length> + Clone> IntoVar<Size> for (W, H) {
    type Var = OwnedVar<Size>;

    fn into_var(self) -> Self::Var {
        OwnedVar(self.into())
    }
}

/// Computed [`Size`].
pub type LayoutSize = webrender::api::units::LayoutSize;

/// 2D rect in [`Length`] units.
#[derive(Copy, Clone, Debug)]
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

    pub fn from_size<S: Into<Size>>(size: S) -> Self {
        Self::new(Point::zero(), size)
    }

    #[inline]
    pub fn zero() -> Self {
        Self::new(Point::zero(), Size::zero())
    }

    /// Rect that fills the available space.
    #[inline]
    pub fn fill() -> Self {
        Self::from_size(Size::fill())
    }

    #[inline]
    pub fn to_layout(&self, available_size: LayoutSize, ctx: &LayoutContext) -> LayoutRect {
        LayoutRect::new(self.origin.to_layout(available_size, ctx), self.size.to_layout(available_size, ctx))
    }
}
impl From<Size> for Rect {
    fn from(size: Size) -> Self {
        Self::from_size(size)
    }
}
impl From<Rect> for Size {
    fn from(rect: Rect) -> Self {
        rect.size
    }
}
impl From<Rect> for Point {
    fn from(rect: Rect) -> Self {
        rect.origin
    }
}
impl<O: Into<Point>, S: Into<Size>> From<(O, S)> for Rect {
    fn from(t: (O, S)) -> Self {
        Rect::new(t.0, t.1)
    }
}
impl<X, Y, W, H> From<(X, Y, W, H)> for Rect
where
    X: Into<Length>,
    Y: Into<Length>,
    W: Into<Length>,
    H: Into<Length>,
{
    fn from(t: (X, Y, W, H)) -> Self {
        Rect::new((t.0, t.1), (t.2, t.3))
    }
}
impl<O: Into<Point> + Clone, S: Into<Size> + Clone> IntoVar<Rect> for (O, S) {
    type Var = OwnedVar<Rect>;

    fn into_var(self) -> Self::Var {
        OwnedVar(self.into())
    }
}
impl<X, Y, W, H> IntoVar<Rect> for (X, Y, W, H)
where
    X: Into<Length> + Clone,
    Y: Into<Length> + Clone,
    W: Into<Length> + Clone,
    H: Into<Length> + Clone,
{
    type Var = OwnedVar<Rect>;

    fn into_var(self) -> Self::Var {
        OwnedVar(self.into())
    }
}

/// Computed [`Rect`].
pub type LayoutRect = webrender::api::units::LayoutRect;

/// 2D size offsets in [`Length`] units.
#[derive(Copy, Clone, Debug)]
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

    /// Top-Bottom and Left-Right equal.
    pub fn new_dimension<TB: Into<Length>, LR: Into<Length>>(top_bottom: TB, left_right: LR) -> Self {
        let top_bottom = top_bottom.into();
        let left_right = left_right.into();
        SideOffsets {
            top: top_bottom,
            bottom: top_bottom,
            left: left_right,
            right: left_right,
        }
    }

    /// All sides equal.
    pub fn new_all<T: Into<Length>>(all_sides: T) -> Self {
        let all_sides = all_sides.into();
        SideOffsets {
            top: all_sides,
            right: all_sides,
            bottom: all_sides,
            left: all_sides,
        }
    }

    #[inline]
    pub fn zero() -> Self {
        Self::new_all(Length::zero())
    }

    #[inline]
    pub fn to_layout(&self, available_size: LayoutSize, ctx: &LayoutContext) -> LayoutSideOffsets {
        let width = LayoutLength::new(available_size.width);
        let height = LayoutLength::new(available_size.height);
        LayoutSideOffsets::from_lengths(
            self.top.to_layout(height, ctx),
            self.right.to_layout(width, ctx),
            self.bottom.to_layout(height, ctx),
            self.left.to_layout(width, ctx),
        )
    }
}

/// all sides equal.
impl<A: Into<Length>> From<A> for SideOffsets {
    fn from(all: A) -> Self {
        SideOffsets::new_all(all)
    }
}

/// all sides equal.
impl<A: Into<Length> + Clone> IntoVar<SideOffsets> for A {
    type Var = OwnedVar<SideOffsets>;

    fn into_var(self) -> Self::Var {
        OwnedVar(SideOffsets::new_all(self))
    }
}

/// (top-bottom, left-right)
impl<TB: Into<Length>, LR: Into<Length>> From<(TB, LR)> for SideOffsets {
    fn from(t: (TB, LR)) -> Self {
        SideOffsets::new_dimension(t.0, t.1)
    }
}

/// (top-bottom, left-right)
impl<TB, LR> IntoVar<SideOffsets> for (TB, LR)
where
    TB: Into<Length> + Clone,
    LR: Into<Length> + Clone,
{
    type Var = OwnedVar<SideOffsets>;

    fn into_var(self) -> Self::Var {
        OwnedVar(self.into())
    }
}

/// (top, right, bottom, left)
impl<T, R, B, L> From<(T, R, B, L)> for SideOffsets
where
    T: Into<Length>,
    R: Into<Length>,
    B: Into<Length>,
    L: Into<Length>,
{
    fn from(t: (T, R, B, L)) -> Self {
        SideOffsets::new(t.0, t.1, t.2, t.3)
    }
}

/// (top, right, bottom, left)
impl<T, R, B, L> IntoVar<SideOffsets> for (T, R, B, L)
where
    T: Into<Length> + Clone,
    R: Into<Length> + Clone,
    B: Into<Length> + Clone,
    L: Into<Length> + Clone,
{
    type Var = OwnedVar<SideOffsets>;

    fn into_var(self) -> Self::Var {
        OwnedVar(self.into())
    }
}

/// Computed [`SideOffsets`].
pub type LayoutSideOffsets = webrender::api::units::LayoutSideOffsets;

// TODO

/// for uniform
impl IntoVar<LayoutSideOffsets> for f32 {
    type Var = OwnedVar<LayoutSideOffsets>;

    fn into_var(self) -> Self::Var {
        OwnedVar(LayoutSideOffsets::new_all_same(self))
    }
}

///for (top-bottom, left-right)
impl IntoVar<LayoutSideOffsets> for (f32, f32) {
    type Var = OwnedVar<LayoutSideOffsets>;

    fn into_var(self) -> Self::Var {
        OwnedVar(LayoutSideOffsets::new(self.0, self.1, self.0, self.1))
    }
}

///for (top, right, bottom, left)
impl IntoVar<LayoutSideOffsets> for (f32, f32, f32, f32) {
    type Var = OwnedVar<LayoutSideOffsets>;

    fn into_var(self) -> Self::Var {
        OwnedVar(LayoutSideOffsets::new(self.0, self.1, self.2, self.3))
    }
}

impl IntoVar<LayoutPoint> for (f32, f32) {
    type Var = OwnedVar<LayoutPoint>;

    fn into_var(self) -> Self::Var {
        let (x, y) = self;
        OwnedVar(LayoutPoint::new(x, y))
    }
}

impl IntoVar<LayoutSize> for (f32, f32) {
    type Var = OwnedVar<LayoutSize>;

    fn into_var(self) -> Self::Var {
        let (w, h) = self;
        OwnedVar(LayoutSize::new(w, h))
    }
}

impl IntoVar<LayoutRect> for (f32, f32, f32, f32) {
    type Var = OwnedVar<LayoutRect>;

    fn into_var(self) -> Self::Var {
        let (x, y, w, h) = self;
        OwnedVar(LayoutRect::new(LayoutPoint::new(x, y), LayoutSize::new(w, h)))
    }
}

/// A device pixel scale factor used for pixel alignment.
///
/// Types that can be aligned with this grid implement [`PixelGridExt`].
#[derive(Copy, Clone, Debug)]
pub struct PixelGrid {
    pub scale_factor: f32,
}
impl PixelGrid {
    #[inline]
    pub fn new(scale_factor: f32) -> Self {
        PixelGrid { scale_factor }
    }

    /// Aligns the layout value `n` using this algorithm:
    ///
    /// scaled `n` | op
    /// -----------|------------------------
    /// < 0.01     | floor (`0`)
    /// < 1.0      | ceil (`1` pixel)
    /// >= 1.0     | round to nearest pixel
    #[inline]
    pub fn snap(self, layout_value: f32) -> f32 {
        let px = layout_value * self.scale_factor;
        if px < 0.01 {
            0.0
        } else if px < 1.0 {
            1.0 / self.scale_factor
        } else {
            px.round() / self.scale_factor
        }
    }

    /// Checks if the layout value is aligned with this grid.
    #[inline]
    pub fn is_aligned(self, layout_value: f32) -> bool {
        let scaled = layout_value * self.scale_factor;
        (scaled - scaled.round()).abs() < 0.0001
    }
}
impl Default for PixelGrid {
    /// `1.0` scale factor.
    #[inline]
    fn default() -> Self {
        PixelGrid::new(1.0)
    }
}
impl PartialEq for PixelGrid {
    fn eq(&self, other: &Self) -> bool {
        (self.scale_factor - other.scale_factor).abs() < 0.01
    }
}

/// Methods for types that can be aligned to a [`PixelGrid`].
pub trait PixelGridExt {
    /// Gets a copy of self that is aligned with the pixel grid.
    fn snap_to(self, grid: PixelGrid) -> Self;
    /// Checks if self is aligned with the pixel grid.
    fn is_aligned_to(self, grid: PixelGrid) -> bool;
}
impl PixelGridExt for LayoutPoint {
    #[inline]
    fn snap_to(self, grid: PixelGrid) -> Self {
        LayoutPoint::new(grid.snap(self.x), grid.snap(self.y))
    }
    #[inline]
    fn is_aligned_to(self, grid: PixelGrid) -> bool {
        grid.is_aligned(self.x) && grid.is_aligned(self.y)
    }
}
impl PixelGridExt for LayoutSize {
    #[inline]
    fn snap_to(self, grid: PixelGrid) -> Self {
        LayoutSize::new(grid.snap(self.width), grid.snap(self.height))
    }
    #[inline]
    fn is_aligned_to(self, grid: PixelGrid) -> bool {
        grid.is_aligned(self.width) && grid.is_aligned(self.height)
    }
}
impl PixelGridExt for LayoutRect {
    #[inline]
    fn snap_to(self, grid: PixelGrid) -> Self {
        LayoutRect::new(self.origin.snap_to(grid), self.size.snap_to(grid))
    }
    #[inline]
    fn is_aligned_to(self, grid: PixelGrid) -> bool {
        self.origin.is_aligned_to(grid) && self.size.is_aligned_to(grid)
    }
}
impl PixelGridExt for LayoutSideOffsets {
    #[inline]
    fn snap_to(self, grid: PixelGrid) -> Self {
        LayoutSideOffsets::new(
            grid.snap(self.top),
            grid.snap(self.right),
            grid.snap(self.bottom),
            grid.snap(self.left),
        )
    }
    #[inline]
    fn is_aligned_to(self, grid: PixelGrid) -> bool {
        grid.is_aligned(self.top) && grid.is_aligned(self.right) && grid.is_aligned(self.bottom) && grid.is_aligned(self.left)
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
