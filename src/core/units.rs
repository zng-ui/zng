//! Angle, factor and length units.

use derive_more as dm;
use std::{f32::consts::*, fmt, time::Duration};
use webrender::api::units as wr;

use super::context::LayoutContext;
use crate::core::var::{IntoVar, OwnedVar};

const TAU: f32 = 2.0 * PI;

/// Angle in radians.
///
/// See [`AngleUnits`] for more details.
#[derive(
    Debug,
    dm::Display,
    Copy,
    Clone,
    dm::Add,
    dm::AddAssign,
    dm::Sub,
    dm::SubAssign,
    dm::Mul,
    dm::MulAssign,
    dm::Div,
    dm::DivAssign,
    dm::Neg,
    PartialEq,
)]
#[display(fmt = "{} rad", self.0)]
pub struct AngleRadian(pub f32);
impl AngleRadian {
    /// Radians in `[0.0 ..= TAU]`.
    #[inline]
    pub fn modulo(self) -> Self {
        AngleGradian::from(self).modulo().into()
    }
    #[inline]
    pub fn to_layout(self) -> LayoutAngle {
        self.into()
    }
}
impl_from_and_into_var! {
    fn from(grad: AngleGradian) -> AngleRadian {
        AngleRadian(grad.0 * PI / 200.0)
    }

    fn from(deg: AngleDegree) -> AngleRadian {
        AngleRadian(deg.0.to_radians())
    }

    fn from(turn: AngleTurn) -> AngleRadian {
        AngleRadian(turn.0 * TAU)
    }
}

/// Angle in gradians.
///
/// See [`AngleUnits`] for more details.
#[derive(
    Debug,
    dm::Display,
    Copy,
    Clone,
    dm::Add,
    dm::AddAssign,
    dm::Sub,
    dm::SubAssign,
    dm::Mul,
    dm::MulAssign,
    dm::Div,
    dm::DivAssign,
    dm::Neg,
    PartialEq,
)]
#[display(fmt = "{} gon", self.0)]
pub struct AngleGradian(pub f32);
impl AngleGradian {
    /// Gradians in `[0.0 ..= 400.0]`.
    #[inline]
    pub fn modulo(self) -> Self {
        AngleGradian(self.0.rem_euclid(400.0))
    }
}
impl_from_and_into_var! {
    fn from(rad: AngleRadian) -> AngleGradian {
        AngleGradian(rad.0 * 200.0 / PI)
    }

    fn from(deg: AngleDegree) -> AngleGradian {
        AngleGradian(deg.0 * 10.0 / 9.0)
    }

    fn from(turn: AngleTurn) -> AngleGradian {
        AngleGradian(turn.0 * 400.0)
    }
}

/// Angle in degrees.
///
/// See [`AngleUnits`] for more details.
#[derive(
    Debug,
    dm::Display,
    Copy,
    Clone,
    dm::Add,
    dm::AddAssign,
    dm::Sub,
    dm::SubAssign,
    dm::Mul,
    dm::MulAssign,
    dm::Div,
    dm::DivAssign,
    dm::Neg,
    PartialEq,
)]
#[display(fmt = "{}º", self.0)]
pub struct AngleDegree(pub f32);
impl AngleDegree {
    /// Degrees in `[0.0 ..= 360.0]`.
    #[inline]
    pub fn modulo(self) -> Self {
        AngleDegree(self.0.rem_euclid(360.0))
    }
}
impl_from_and_into_var! {
    fn from(rad: AngleRadian) -> AngleDegree {
        AngleDegree(rad.0.to_degrees())
    }

    fn from(grad: AngleGradian) -> AngleDegree {
        AngleDegree(grad.0 * 9.0 / 10.0)
    }

    fn from(turn: AngleTurn) -> AngleDegree {
        AngleDegree(turn.0 * 360.0)
    }
}

/// Angle in turns (complete rotations).
///
/// See [`AngleUnits`] for more details.
#[derive(
    Debug,
    dm::Display,
    Copy,
    Clone,
    dm::Add,
    dm::AddAssign,
    dm::Sub,
    dm::SubAssign,
    dm::Mul,
    dm::MulAssign,
    dm::Div,
    dm::DivAssign,
    dm::Neg,
    PartialEq,
)]
#[display(fmt = "{} tr", self.0)]
pub struct AngleTurn(pub f32);
impl AngleTurn {
    /// Turns in `[0.0 ..= 1.0]`.
    #[inline]
    pub fn modulo(self) -> Self {
        AngleTurn(self.0.rem_euclid(1.0))
    }
}
impl_from_and_into_var! {
    fn from(rad: AngleRadian) -> AngleTurn {
        AngleTurn(rad.0 / TAU)
    }

    fn from(grad: AngleGradian) -> AngleTurn {
        AngleTurn(grad.0 / 400.0)
    }

    fn from(deg: AngleDegree) -> AngleTurn {
        AngleTurn(deg.0 / 360.0)
    }
}

/// Radian angle type used by webrender.
pub type LayoutAngle = euclid::Angle<f32>;
impl From<AngleRadian> for LayoutAngle {
    fn from(rad: AngleRadian) -> Self {
        LayoutAngle::radians(rad.0)
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
impl_from_and_into_var! {
    fn from(n: FactorNormal) -> FactorPercent {
        FactorPercent(n.0 * 100.0)
    }
}

/// Normalized multiplication factor (0.0-1.0).
///
/// See [`FactorUnits`] for more details.
#[derive(Debug, dm::Display, Copy, Clone, dm::Add, dm::AddAssign, dm::Sub, dm::SubAssign, PartialEq)]
pub struct FactorNormal(pub f32);
impl FactorNormal {
    /// Clamp factor to [0.0..=1.0] range.
    #[inline]
    pub fn clamp_range(self) -> Self {
        FactorNormal(self.0.max(0.0).min(1.0))
    }
}
impl_from_and_into_var! {
    fn from(percent: FactorPercent) -> FactorNormal {
        FactorNormal(percent.0 / 100.0)
    }

    fn from(f: f32) -> FactorNormal {
        FactorNormal(f)
    }

    /// | Input  | Output  |
    /// |--------|---------|
    /// |`true`  | `1.0`   |
    /// |`false` | `0.0`   |
    fn from(b: bool) -> FactorNormal {
        FactorNormal(if b { 1.0 } else { 0.0 })
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
impl FactorUnits for i32 {
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
#[derive(Copy, Clone, Debug, PartialEq)]
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
impl fmt::Display for Length {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Length::Exact(l) => fmt::Display::fmt(l, f),
            Length::Relative(n) => write!(f, "{:.*}%", f.precision().unwrap_or(0), n.0 * 100.0),
            Length::Em(e) => write!(f, "{}em", e),
            Length::RootEm(re) => write!(f, "{}rem", re),
            Length::ViewportWidth(vw) => write!(f, "{}vw", vw),
            Length::ViewportHeight(vh) => write!(f, "{}vh", vh),
            Length::ViewportMin(vmin) => write!(f, "{}vmin", vmin),
            Length::ViewportMax(vmax) => write!(f, "{}vmax", vmax),
        }
    }
}
impl_from_and_into_var! {
    /// Conversion to [`Length::Relative`]
    fn from(percent: FactorPercent) -> Length {
        Length::Relative(percent.into())
    }

    /// Conversion to [`Length::Relative`]
    fn from(norm: FactorNormal) -> Length {
        Length::Relative(norm)
    }

    /// Conversion to [`Length::Exact`]
    fn from(f: f32) -> Length {
        Length::Exact(f)
    }

    /// Conversion to [`Length::Exact`]
    fn from(i: i32) -> Length {
        Length::Exact(i as f32)
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

    /// Exact length in font units.
    #[inline]
    pub fn pt(font_pt: f32) -> Length {
        // make this const when https://github.com/rust-lang/rust/issues/57241
        Length::Exact(font_pt * 96.0 / 72.0)
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
pub type LayoutLength = euclid::Length<f32, wr::LayoutPixel>;

/// Convert a [`LayoutLength`] to font units.
#[inline]
pub fn layout_length_to_pt(length: LayoutLength) -> f32 {
    length.get() * 72.0 / 96.0
}

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
    /// Exact size in font units.
    ///
    /// Returns [`Length::Exact`].
    fn pt(self) -> Length;

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
    fn pt(self) -> Length {
        Length::pt(self)
    }
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
impl LengthUnits for i32 {
    #[inline]
    fn pt(self) -> Length {
        Length::pt(self as f32)
    }
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

/// Implement From<{tuple of Into<Length>}> and IntoVar for Length compound types.
macro_rules! impl_length_comp_conversions {
    ($(
        $(#[$docs:meta])*
        fn from($($n:ident : $N:ident),+) -> $For:ty {
            $convert:expr
        }
    )+) => {
        $(
            impl<$($N),+> From<($($N),+)> for $For
            where
                $($N: Into<Length>,)+
            {
                $(#[$docs])*
                fn from(($($n),+) : ($($N),+)) -> Self {
                    $convert
                }
            }

            impl<$($N),+> IntoVar<$For> for ($($N),+)
            where
            $($N: Into<Length> + Clone,)+
            {
                type Var = OwnedVar<$For>;

                $(#[$docs])*
                fn into_var(self) -> Self::Var {
                    OwnedVar(self.into())
                }
            }
        )+
    };
}

/// 2D point in [`Length`] units.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Point {
    pub x: Length,
    pub y: Length,
}
impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(p) = f.precision() {
            write!(f, "({:.p$}, {:.p$})", self.x, self.y, p = p)
        } else {
            write!(f, "({}, {})", self.x, self.y)
        }
    }
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
impl_length_comp_conversions! {
    fn from(x: X, y: Y) -> Point {
        Point::new(x, y)
    }
}

/// Computed [`Point`].
pub type LayoutPoint = wr::LayoutPoint;

/// 2D size in [`Length`] units.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Size {
    pub width: Length,
    pub height: Length,
}
impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(p) = f.precision() {
            write!(f, "{:.p$} × {:.p$}", self.width, self.height, p = p)
        } else {
            write!(f, "{} × {}", self.width, self.height)
        }
    }
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
impl_length_comp_conversions! {
    fn from(width: W, height: H) -> Size {
        Size::new(width, height)
    }
}

/// Computed [`Size`].
pub type LayoutSize = wr::LayoutSize;

/// 2D rect in [`Length`] units.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}
impl fmt::Display for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(p) = f.precision() {
            write!(f, "{:.p$} {:.p$}", self.origin, self.size, p = p)
        } else {
            write!(f, "{} {}", self.origin, self.size)
        }
    }
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
impl<O: Into<Point> + Clone, S: Into<Size> + Clone> IntoVar<Rect> for (O, S) {
    type Var = OwnedVar<Rect>;

    fn into_var(self) -> Self::Var {
        OwnedVar(self.into())
    }
}

impl_length_comp_conversions! {
    fn from(x: X, y: Y, width: W, height: H) -> Rect {
        Rect::new((x, y), (width, height))
    }
}

/// Computed [`Rect`].
pub type LayoutRect = wr::LayoutRect;

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

impl_from_and_into_var! {
    /// All sides equal.
    fn from(all: Length) -> SideOffsets {
        SideOffsets::new_all(all)
    }

    /// All sides equal relative length.
    fn from(percent: FactorPercent) -> SideOffsets {
        SideOffsets::new_all(percent)
    }
    /// All sides equal relative length.
    fn from(norm: FactorNormal) -> SideOffsets {
        SideOffsets::new_all(norm)
    }

    /// All sides equal exact length.
    fn from(f: f32) -> SideOffsets {
        SideOffsets::new_all(f)
    }
    /// All sides equal exact length.
    fn from(i: i32) -> SideOffsets {
        SideOffsets::new_all(i)
    }
}

impl_length_comp_conversions! {
    /// (top-bottom, left-right)
    fn from(top_bottom: TB, left_right: LR) -> SideOffsets {
        SideOffsets::new_dimension(top_bottom,left_right)
    }

    /// (top, right, bottom, left)
    fn from(top: T, right: R, bottom: B, left: L) -> SideOffsets {
        SideOffsets::new(top, right, bottom, left)
    }
}

/// Computed [`SideOffsets`].
pub type LayoutSideOffsets = wr::LayoutSideOffsets;

/// `x` and `y` alignment.
///
/// The numbers indicate how much to the right and bottom the content is moved within
/// a larger available space.
#[derive(PartialEq, Clone, Copy)]
pub struct Alignment {
    pub x: FactorNormal,
    pub y: FactorNormal,
}
impl<X: Into<FactorNormal>, Y: Into<FactorNormal>> From<(X, Y)> for Alignment {
    fn from((x, y): (X, Y)) -> Self {
        Alignment { x: x.into(), y: y.into() }
    }
}
impl<X: Into<FactorNormal> + Clone, Y: Into<FactorNormal> + Clone> IntoVar<Alignment> for (X, Y) {
    type Var = OwnedVar<Alignment>;

    fn into_var(self) -> Self::Var {
        OwnedVar(self.into())
    }
}
macro_rules! named_aligns {
    ( $($NAME:ident = ($x:expr, $y:expr);)+ ) => {named_aligns!{$(
        [stringify!(($x, $y))] $NAME = ($x, $y);
    )+}};

    ( $([$doc:expr] $NAME:ident = ($x:expr, $y:expr);)+ ) => {$(
        #[doc=$doc]
        pub const $NAME: Alignment = Alignment { x: FactorNormal($x), y: FactorNormal($y) };

    )+};
}
impl Alignment {
    named_aligns! {
        TOP_LEFT = (0.0, 0.0);
        TOP_CENTER = (0.0, 0.5);
        TOP_RIGHT = (0.0, 1.0);

        CENTER_LEFT = (0.0, 0.5);
        CENTER = (0.5, 0.5);
        CENTER_RIGHT = (1.0, 0.5);

        BOTTOM_LEFT = (0.0, 1.0);
        BOTTOM_CENTER = (0.5, 1.0);
        BOTTOM_RIGHT = (1.0, 1.0);
    }
}
macro_rules! debug_display_align {
    (  $($NAME:ident),+ $(,)?) => {
        impl fmt::Display for Alignment {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let a = *self;
                $(if a == Alignment::$NAME { write!(f, "{}", stringify!($NAME)) }) else +
                else {
                    write!(f, "({}%, {}%)", a.x.0 * 100.0, a.y.0 * 100.0)
                }
            }
        }
        impl fmt::Debug for Alignment {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let a = *self;
                $(if a == Alignment::$NAME { write!(f, "Alignment::{}", stringify!($NAME)) }) else +
                else {
                    f.debug_struct("Alignment").field("x", &a.x).field("y", &a.y).finish()
                }
            }
        }
    };
}
debug_display_align! {
   TOP_LEFT,
   TOP_CENTER,
   TOP_RIGHT,

   CENTER_LEFT,
   CENTER,
   CENTER_RIGHT,

   BOTTOM_LEFT,
   BOTTOM_CENTER,
   BOTTOM_RIGHT,
}
impl From<Alignment> for Point {
    /// To relative length x and y.
    fn from(a: Alignment) -> Self {
        Point {
            x: a.x.into(),
            y: a.y.into(),
        }
    }
}
impl IntoVar<Point> for Alignment {
    type Var = OwnedVar<Point>;

    /// To relative length x and y.
    fn into_var(self) -> Self::Var {
        OwnedVar(self.into())
    }
}

/// Text line height.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LineHeight {
    /// Default height from the font data.
    ///
    /// The final value is computed from the font metrics: `ascent - descent + line_gap`. This
    /// is usually similar to `1.2.em()`.
    Font,
    /// Height in [`Length`] units.
    ///
    /// Relative lengths are computed to the font size.
    Length(Length),
}
impl Default for LineHeight {
    /// [`LineHeight::Font`]
    fn default() -> Self {
        LineHeight::Font
    }
}
impl_from_and_into_var! {
    fn from(length: Length) -> LineHeight {
        LineHeight::Length(length)
    }

    /// Percentage of font size.
    fn from(percent: FactorPercent) -> LineHeight {
        LineHeight::Length(percent.into())
    }
    /// Relative to font size.
    fn from(norm: FactorNormal) -> LineHeight {
        LineHeight::Length(norm.into())
    }

    /// Exact size in layout pixels.
    fn from(f: f32) -> LineHeight {
        LineHeight::Length(f.into())
    }
    /// Exact size in layout pixels.
    fn from(i: i32) -> LineHeight {
        LineHeight::Length(i.into())
    }
}

/// Extra spacing added in between text letters.
///
/// Letter spacing is computed using the font data, this unit represents
/// extra space added to the computed spacing.
///
/// A "letter" is a character glyph cluster, e.g.: `a`, `â`, `1`, `-`, `漢`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LetterSpacing {
    /// Letter spacing can be tweaked when justification is enabled.
    Auto,
    /// Extra space in [`Length`] units.
    ///
    /// Relative lengths are computed from the affected glyph "advance",
    /// that is, how much "width" the next letter will take.
    ///
    /// This variant disables automatic adjustments for justification.
    Length(Length),
}
impl Default for LetterSpacing {
    /// [`LetterSpacing::Auto`]
    fn default() -> Self {
        LetterSpacing::Auto
    }
}
impl_from_and_into_var! {
    fn from(length: Length) -> LetterSpacing {
        LetterSpacing::Length(length)
    }

    /// Percentage of font size.
    fn from(percent: FactorPercent) -> LetterSpacing {
        LetterSpacing::Length(percent.into())
    }
    /// Relative to font size.
    fn from(norm: FactorNormal) -> LetterSpacing {
        LetterSpacing::Length(norm.into())
    }

    /// Exact size in layout pixels.
    fn from(f: f32) -> LetterSpacing {
        LetterSpacing::Length(f.into())
    }
    /// Exact size in layout pixels.
    fn from(i: i32) -> LetterSpacing {
        LetterSpacing::Length(i.into())
    }
}

/// Extra spacing added to the Unicode `U+0020 SPACE` character.
///
/// Word spacing is done using the space character "advance" as defined in the font,
/// this unit represents extra spacing added to that default spacing.
///
/// A "word" is the sequence of characters in between space characters. This extra
/// spacing is applied per space character not per word, if there are three spaces between words
/// the extra spacing is applied thrice. Usually the number of spaces between words is collapsed to one,
/// see [`white_space`](crate::properties::text_theme::white_space) for more details.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum WordSpacing {
    /// Word spacing can be tweaked when justification is enabled.
    Auto,
    /// Extra space in [`Length`] units.
    ///
    /// Relative lengths are computed from the default space advance.
    ///
    /// This variant disables automatic adjustments for justification.
    Length(Length),
}
impl Default for WordSpacing {
    /// [`WordSpacing::Auto`]
    fn default() -> Self {
        WordSpacing::Auto
    }
}
impl_from_and_into_var! {
    fn from(length: Length) -> WordSpacing {
        WordSpacing::Length(length)
    }

    /// Percentage of space advance (width).
    fn from(percent: FactorPercent) -> WordSpacing {
        WordSpacing::Length(percent.into())
    }
    /// Relative to the space advance (width).
    fn from(norm: FactorNormal) -> WordSpacing {
        WordSpacing::Length(norm.into())
    }

    /// Exact space in layout pixels.
    fn from(f: f32) -> WordSpacing {
        WordSpacing::Length(f.into())
    }
    /// Exact space in layout pixels.
    fn from(i: i32) -> WordSpacing {
        WordSpacing::Length(i.into())
    }
}

/// Length of a `TAB` space.
///
/// Relative lengths are computed from the normal space character "advance" plus the [`WordSpacing`].
/// So a `400%` length is 4 spaces.
pub type TabLength = Length;

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

        if px > 0.0 {
            if px < 0.01 {
                0.0
            } else if px < 1.0 {
                1.0 / self.scale_factor
            } else {
                px.round() / self.scale_factor
            }
        } else if px > -0.01 {
            0.0
        } else if px > -1.0 {
            -1.0 / self.scale_factor
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

/// A transform builder type.
///
/// # Builder
///
/// The transform can be started by one of this functions, [`rotate`], [`translate`], [`scale`] and [`skew`]. More
/// transforms can be chained by calling the methods of this type.
///
/// # Example
///
/// ```
/// # use zero_ui::prelude::*;
/// let rotate_then_move = rotate(10.deg()).translate(50.0, 30.0);
/// ```
#[derive(Clone, Default, Debug)]
pub struct Transform {
    steps: Vec<TransformStep>,
}
#[derive(Clone, Debug)]
enum TransformStep {
    Computed(LayoutTransform),
    Translate(Length, Length),
}
impl Transform {
    /// No transform.
    #[inline]
    pub fn identity() -> Self {
        Self::default()
    }

    /// Appends the `other` transform.
    pub fn and(mut self, other: Transform) -> Self {
        let mut other_steps = other.steps.into_iter();
        if let Some(first) = other_steps.next() {
            match first {
                TransformStep::Computed(first) => self.push_transform(first),
                first => self.steps.push(first),
            }
            self.steps.extend(other_steps);
        }
        self
    }

    fn push_transform(&mut self, transform: LayoutTransform) {
        if let Some(TransformStep::Computed(last)) = self.steps.last_mut() {
            *last = last.post_transform(&transform);
        } else {
            self.steps.push(TransformStep::Computed(transform));
        }
    }

    /// Append a 2d rotation transform.
    pub fn rotate<A: Into<AngleRadian>>(mut self, angle: A) -> Self {
        self.push_transform(LayoutTransform::create_rotation(0.0, 0.0, -1.0, angle.into().to_layout()));
        self
    }

    /// Append a 2d translation transform.
    #[inline]
    pub fn translate<X: Into<Length>, Y: Into<Length>>(mut self, x: X, y: Y) -> Self {
        self.steps.push(TransformStep::Translate(x.into(), y.into()));
        self
    }
    /// Append a 2d translation transform in the X dimension.
    #[inline]
    pub fn translate_x(self, x: f32) -> Self {
        self.translate(x, 0.0)
    }
    /// Append a 2d translation transform in the Y dimension.
    #[inline]
    pub fn translate_y(self, y: f32) -> Self {
        self.translate(0.0, y)
    }

    /// Append a 2d skew transform.
    pub fn skew<X: Into<AngleRadian>, Y: Into<AngleRadian>>(mut self, x: X, y: Y) -> Self {
        self.push_transform(LayoutTransform::create_skew(x.into().to_layout(), y.into().to_layout()));
        self
    }
    /// Append a 2d skew transform in the X dimension.
    pub fn skew_x<X: Into<AngleRadian>>(self, x: X) -> Self {
        self.skew(x, 0.rad())
    }
    /// Append a 2d skew transform in the Y dimension.
    pub fn skew_y<Y: Into<AngleRadian>>(self, y: Y) -> Self {
        self.skew(0.rad(), y)
    }

    /// Append a 2d scale transform.
    pub fn scale_xy<X: Into<FactorNormal>, Y: Into<FactorNormal>>(mut self, x: X, y: Y) -> Self {
        self.push_transform(LayoutTransform::create_scale(x.into().0, y.into().0, 1.0));
        self
    }
    /// Append a 2d scale transform in the X dimension.
    pub fn scale_x<X: Into<FactorNormal>>(self, x: X) -> Self {
        self.scale_xy(x, 1.0)
    }
    /// Append a 2d scale transform in the Y dimension.
    pub fn scale_y<Y: Into<FactorNormal>>(self, y: Y) -> Self {
        self.scale_xy(1.0, y)
    }
    /// Append a 2d uniform scale transform.
    pub fn scale<S: Into<FactorNormal>>(self, scale: S) -> Self {
        let s = scale.into();
        self.scale_xy(s, s)
    }

    /// Compute a [`LayoutTransform`].
    #[inline]
    pub fn to_layout(&self, available_size: LayoutSize, ctx: &LayoutContext) -> LayoutTransform {
        let mut r = LayoutTransform::identity();
        for step in &self.steps {
            r = match step {
                TransformStep::Computed(m) => r.post_transform(m),
                TransformStep::Translate(x, y) => r.post_translate(euclid::vec3(
                    x.to_layout(LayoutLength::new(available_size.width), ctx).get(),
                    y.to_layout(LayoutLength::new(available_size.height), ctx).get(),
                    0.0,
                )),
            };
        }
        r
    }
}

/// Create a 2d rotation transform.
pub fn rotate<A: Into<AngleRadian>>(angle: A) -> Transform {
    Transform::default().rotate(angle)
}

/// Create a 2d translation transform.
pub fn translate<X: Into<Length>, Y: Into<Length>>(x: X, y: Y) -> Transform {
    Transform::default().translate(x, y)
}

/// Create a 2d translation transform in the X dimension.
pub fn translate_x<X: Into<Length>>(x: X) -> Transform {
    translate(x, 0.0)
}

/// Create a 2d translation transform in the Y dimension.
pub fn translate_y<Y: Into<Length>>(y: Y) -> Transform {
    translate(0.0, y)
}

/// Create a 2d skew transform.
pub fn skew<X: Into<AngleRadian>, Y: Into<AngleRadian>>(x: X, y: Y) -> Transform {
    Transform::default().skew(x, y)
}

/// Create a 2d skew transform in the X dimension.
pub fn skew_x<X: Into<AngleRadian>>(x: X) -> Transform {
    skew(x, 0.rad())
}

/// Create a 2d skew transform in the Y dimension.
pub fn skew_y<Y: Into<AngleRadian>>(y: Y) -> Transform {
    skew(0.rad(), y)
}

/// Create a 2d scale transform.
///
/// The same `scale` is applied to both dimensions.
pub fn scale<S: Into<FactorNormal>>(scale: S) -> Transform {
    let scale = scale.into();
    scale_xy(scale, scale)
}

/// Create a 2d scale transform on the X dimension.
pub fn scale_x<X: Into<FactorNormal>>(x: X) -> Transform {
    scale_xy(x, 1.0)
}

/// Create a 2d scale transform on the Y dimension.
pub fn scale_y<Y: Into<FactorNormal>>(y: Y) -> Transform {
    scale_xy(1.0, y)
}

/// Create a 2d scale transform.
pub fn scale_xy<X: Into<FactorNormal>, Y: Into<FactorNormal>>(x: X, y: Y) -> Transform {
    Transform::default().scale_xy(x, y)
}

/// Computed [`Transform`].
pub type LayoutTransform = wr::LayoutTransform;

/// Extension methods for initializing [`Duration`] values.
pub trait TimeUnits {
    /// Milliseconds.
    fn ms(self) -> Duration;
    /// Seconds.
    fn secs(self) -> Duration;
    /// Minutes.
    fn minutes(self) -> Duration;
}
impl TimeUnits for u64 {
    #[inline]
    fn ms(self) -> Duration {
        Duration::from_millis(self)
    }

    #[inline]
    fn secs(self) -> Duration {
        Duration::from_secs(self)
    }

    #[inline]
    fn minutes(self) -> Duration {
        Duration::from_secs(self / 60)
    }
}
impl TimeUnits for f32 {
    #[inline]
    fn ms(self) -> Duration {
        Duration::from_secs_f32(self / 60.0)
    }

    #[inline]
    fn secs(self) -> Duration {
        Duration::from_secs_f32(self)
    }

    #[inline]
    fn minutes(self) -> Duration {
        Duration::from_secs_f32(self * 60.0)
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
