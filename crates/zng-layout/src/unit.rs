//! Angle, factor, length, time, byte and resolution units.

use std::fmt;

pub use zng_unit::*;

mod alignment;
pub use alignment::*;

mod constraints;
pub use constraints::*;

mod factor;
pub use factor::*;

mod grid;
pub use grid::*;

mod length;
pub use length::*;

mod line;
pub use line::*;

mod point;
pub use point::*;

mod rect;
pub use rect::*;

mod resolution;
pub use resolution::*;

mod side_offsets;
pub use side_offsets::*;

mod size;
pub use size::*;

mod transform;
pub use transform::*;

mod vector;
pub use vector::*;

use crate::context::LayoutMask;

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

            impl<$($N),+> zng_var::IntoVar<$For> for ($($N),+)
            where
            $($N: Into<Length> + Clone,)+
            {
                $(#[$docs])*
                fn into_var(self) -> zng_var::Var<$For> {
                    zng_var::const_var(self.into())
                }
            }
        )+
    };
}
use impl_length_comp_conversions;

/// Represents a two-dimensional value that can be converted to a pixel value in a [`LAYOUT`] context.
///
/// [`LAYOUT`]: crate::context::LAYOUT
pub trait Layout2d {
    /// Pixel type.
    type Px: Default;

    /// Compute the pixel value in the current [`LAYOUT`] context.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout(&self) -> Self::Px {
        self.layout_dft(Default::default())
    }

    /// Compute the pixel value in the current [`LAYOUT`] context with `default`.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout_dft(&self, default: Self::Px) -> Self::Px;

    /// Compute a [`LayoutMask`] that flags all contextual values that affect the result of [`layout`].
    ///
    /// [`layout`]: Self::layout
    fn affect_mask(&self) -> LayoutMask;
}

/// Represents a layout dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LayoutAxis {
    /// Horizontal.
    X,
    /// Vertical.
    Y,
    /// Depth.
    Z,
}

/// Represents a one-dimensional length value that can be converted to a pixel length in a [`LAYOUT`] context.
///
/// [`LAYOUT`]: crate::context::LAYOUT
pub trait Layout1d {
    /// Compute the pixel value in the current [`LAYOUT`] context.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout(&self, axis: LayoutAxis) -> Px {
        self.layout_dft(axis, Px(0))
    }

    /// Compute the pixel value in the current [`LAYOUT`] context with `default`.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout_dft(&self, axis: LayoutAxis, default: Px) -> Px;

    /// Compute the pixel value in the current [`LAYOUT`] context ***x*** axis.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout_x(&self) -> Px {
        self.layout(LayoutAxis::X)
    }

    /// Compute the pixel value in the current [`LAYOUT`] context ***y*** axis.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout_y(&self) -> Px {
        self.layout(LayoutAxis::Y)
    }

    /// Compute the pixel value in the current [`LAYOUT`] context ***z*** axis.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout_z(&self) -> Px {
        self.layout(LayoutAxis::Z)
    }

    /// Compute the pixel value in the current [`LAYOUT`] context ***x*** axis with `default`.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout_dft_x(&self, default: Px) -> Px {
        self.layout_dft(LayoutAxis::X, default)
    }

    /// Compute the pixel value in the current [`LAYOUT`] context ***y*** axis with `default`.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout_dft_y(&self, default: Px) -> Px {
        self.layout_dft(LayoutAxis::Y, default)
    }

    /// Compute the pixel value in the current [`LAYOUT`] context ***z*** axis with `default`.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout_dft_z(&self, default: Px) -> Px {
        self.layout_dft(LayoutAxis::Z, default)
    }

    /// Compute the pixel value in the current [`LAYOUT`] context.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout_f32(&self, axis: LayoutAxis) -> f32 {
        self.layout_f32_dft(axis, 0.0)
    }

    /// Compute the pixel value in the current [`LAYOUT`] context with `default`.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout_f32_dft(&self, axis: LayoutAxis, default: f32) -> f32;

    /// Compute the pixel value in the current [`LAYOUT`] context ***x*** axis.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout_f32_x(&self) -> f32 {
        self.layout_f32(LayoutAxis::X)
    }

    /// Compute the pixel value in the current [`LAYOUT`] context ***y*** axis.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout_f32_y(&self) -> f32 {
        self.layout_f32(LayoutAxis::Y)
    }

    /// Compute the pixel value in the current [`LAYOUT`] context ***z*** axis.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout_f32_z(&self) -> f32 {
        self.layout_f32(LayoutAxis::Z)
    }

    /// Compute the pixel value in the current [`LAYOUT`] context ***x*** axis with `default`.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout_f32_dft_x(&self, default: f32) -> f32 {
        self.layout_f32_dft(LayoutAxis::X, default)
    }

    /// Compute the pixel value in the current [`LAYOUT`] context ***y*** axis with `default`.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout_f32_dft_y(&self, default: f32) -> f32 {
        self.layout_f32_dft(LayoutAxis::Y, default)
    }

    /// Compute the pixel value in the current [`LAYOUT`] context ***z*** axis with `default`.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout_f32_dft_z(&self, default: f32) -> f32 {
        self.layout_f32_dft(LayoutAxis::Z, default)
    }

    /// Compute a [`LayoutMask`] that flags all contextual values that affect the result of [`layout`].
    ///
    /// [`layout`]: Self::layout
    fn affect_mask(&self) -> LayoutMask;
}

/// An error which can be returned when parsing an type composed of integers.
#[derive(Debug)]
#[non_exhaustive]
pub enum ParseFloatCompositeError {
    /// Float component parse error.
    Component(std::num::ParseFloatError),
    /// Missing color component.
    MissingComponent,
    /// Extra color component.
    ExtraComponent,
    /// Unexpected char.
    UnknownFormat,
}
impl fmt::Display for ParseFloatCompositeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseFloatCompositeError::Component(e) => write!(f, "error parsing component, {e}"),
            ParseFloatCompositeError::MissingComponent => write!(f, "missing component"),
            ParseFloatCompositeError::ExtraComponent => write!(f, "extra component"),
            ParseFloatCompositeError::UnknownFormat => write!(f, "unknown format"),
        }
    }
}
impl std::error::Error for ParseFloatCompositeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let ParseFloatCompositeError::Component(e) = self {
            Some(e)
        } else {
            None
        }
    }
}
impl From<std::num::ParseFloatError> for ParseFloatCompositeError {
    fn from(value: std::num::ParseFloatError) -> Self {
        ParseFloatCompositeError::Component(value)
    }
}

/// An error which can be returned when parsing an type composed of integers.
#[derive(Debug)]
#[non_exhaustive]
pub enum ParseCompositeError {
    /// Float component parse error.
    FloatComponent(std::num::ParseFloatError),
    /// Integer component parse error.
    IntComponent(std::num::ParseIntError),
    /// Missing color component.
    MissingComponent,
    /// Extra color component.
    ExtraComponent,
    /// Unexpected char.
    UnknownFormat,
}
impl fmt::Display for ParseCompositeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseCompositeError::FloatComponent(e) => write!(f, "error parsing component, {e}"),
            ParseCompositeError::IntComponent(e) => write!(f, "error parsing component, {e}"),
            ParseCompositeError::MissingComponent => write!(f, "missing component"),
            ParseCompositeError::ExtraComponent => write!(f, "extra component"),
            ParseCompositeError::UnknownFormat => write!(f, "unknown format"),
        }
    }
}
impl std::error::Error for ParseCompositeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let ParseCompositeError::FloatComponent(e) = self {
            Some(e)
        } else if let ParseCompositeError::IntComponent(e) = self {
            Some(e)
        } else {
            None
        }
    }
}
impl From<std::num::ParseFloatError> for ParseCompositeError {
    fn from(value: std::num::ParseFloatError) -> Self {
        ParseCompositeError::FloatComponent(value)
    }
}
impl From<std::num::ParseIntError> for ParseCompositeError {
    fn from(value: std::num::ParseIntError) -> Self {
        ParseCompositeError::IntComponent(value)
    }
}
impl From<ParseFloatCompositeError> for ParseCompositeError {
    fn from(value: ParseFloatCompositeError) -> Self {
        match value {
            ParseFloatCompositeError::Component(e) => ParseCompositeError::FloatComponent(e),
            ParseFloatCompositeError::MissingComponent => ParseCompositeError::MissingComponent,
            ParseFloatCompositeError::ExtraComponent => ParseCompositeError::ExtraComponent,
            ParseFloatCompositeError::UnknownFormat => ParseCompositeError::UnknownFormat,
        }
    }
}
impl From<ParseIntCompositeError> for ParseCompositeError {
    fn from(value: ParseIntCompositeError) -> Self {
        match value {
            ParseIntCompositeError::Component(e) => ParseCompositeError::IntComponent(e),
            ParseIntCompositeError::MissingComponent => ParseCompositeError::MissingComponent,
            ParseIntCompositeError::ExtraComponent => ParseCompositeError::ExtraComponent,
            ParseIntCompositeError::UnknownFormat => ParseCompositeError::UnknownFormat,
            _ => unreachable!(),
        }
    }
}

pub(crate) struct LengthCompositeParser<'a> {
    sep: &'a [char],
    s: &'a str,
}
impl<'a> LengthCompositeParser<'a> {
    pub(crate) fn new(s: &'a str) -> Result<LengthCompositeParser<'a>, ParseCompositeError> {
        Self::new_sep(s, &[','])
    }
    pub(crate) fn new_sep(s: &'a str, sep: &'a [char]) -> Result<LengthCompositeParser<'a>, ParseCompositeError> {
        if let Some(s) = s.strip_prefix('(') {
            if let Some(s) = s.strip_suffix(')') {
                return Ok(Self { s, sep });
            } else {
                return Err(ParseCompositeError::MissingComponent);
            }
        }
        Ok(Self { s, sep })
    }

    pub(crate) fn next(&mut self) -> Result<Length, ParseCompositeError> {
        let mut depth = 0;
        for (ci, c) in self.s.char_indices() {
            if depth == 0
                && let Some(sep) = self.sep.iter().find(|s| **s == c)
            {
                let l = &self.s[..ci];
                self.s = &self.s[ci + sep.len_utf8()..];
                return l.trim().parse();
            } else if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
            }
        }
        if self.s.is_empty() {
            Err(ParseCompositeError::MissingComponent)
        } else {
            let l = self.s;
            self.s = "";
            l.trim().parse()
        }
    }

    pub fn has_ended(&self) -> bool {
        self.s.is_empty()
    }

    pub(crate) fn expect_last(mut self) -> Result<Length, ParseCompositeError> {
        let c = self.next()?;
        if !self.has_ended() {
            Err(ParseCompositeError::ExtraComponent)
        } else {
            Ok(c)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{PI, TAU};

    use zng_app_context::{AppId, LocalContext};

    use crate::context::{LAYOUT, LayoutMetrics};

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

    #[test]
    pub fn length_expr_same_unit() {
        let a = Length::from(200);
        let b = Length::from(300);
        let c = a + b;

        assert_eq!(c, 500.dip());
    }

    #[test]
    pub fn length_expr_diff_units() {
        let a = Length::from(200);
        let b = Length::from(10.pct());
        let c = a + b;

        assert_eq!(c, Length::Expr(Box::new(LengthExpr::Add(200.into(), 10.pct().into()))))
    }

    #[test]
    pub fn length_expr_eval() {
        let _app = LocalContext::start_app(AppId::new_unique());

        let l = (Length::from(200) - 100.pct()).abs();
        let metrics = LayoutMetrics::new(1.fct(), PxSize::new(Px(600), Px(400)), Px(0));
        let l = LAYOUT.with_context(metrics, || l.layout_x());

        assert_eq!(l.0, (200i32 - 600i32).abs());
    }

    #[test]
    pub fn length_expr_clamp() {
        let _app = LocalContext::start_app(AppId::new_unique());

        let l = Length::from(100.pct()).clamp(100, 500);
        assert!(matches!(l, Length::Expr(_)));

        let metrics = LayoutMetrics::new(1.fct(), PxSize::new(Px(200), Px(50)), Px(0));
        LAYOUT.with_context(metrics, || {
            let r = l.layout_x();
            assert_eq!(r.0, 200);

            let r = l.layout_y();
            assert_eq!(r.0, 100);

            LAYOUT.with_constraints(LAYOUT.constraints().with_new_max_x(Px(550)), || {
                let r = l.layout_x();
                assert_eq!(r.0, 500);
            });
        });
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

    #[test]
    fn distance_bounds() {
        assert_eq!(DistanceKey::MAX.distance(), Some(Px::MAX));
        assert_eq!(DistanceKey::MIN.distance(), Some(Px(0)));
    }

    #[test]
    fn orientation_box_above() {
        let a = PxRect::from_size(PxSize::splat(Px(40)));
        let mut b = a;
        b.origin.y = -Px(82);
        let a = a.to_box2d();
        let b = b.to_box2d();

        assert!(Orientation2D::Above.box_is(a, b));
        assert!(!Orientation2D::Below.box_is(a, b));
        assert!(!Orientation2D::Left.box_is(a, b));
        assert!(!Orientation2D::Right.box_is(a, b));
    }

    #[test]
    fn orientation_box_below() {
        let a = PxRect::from_size(PxSize::splat(Px(40)));
        let mut b = a;
        b.origin.y = Px(42);
        let a = a.to_box2d();
        let b = b.to_box2d();

        assert!(!Orientation2D::Above.box_is(a, b));
        assert!(Orientation2D::Below.box_is(a, b));
        assert!(!Orientation2D::Left.box_is(a, b));
        assert!(!Orientation2D::Right.box_is(a, b));
    }

    #[test]
    fn orientation_box_left() {
        let a = PxRect::from_size(PxSize::splat(Px(40)));
        let mut b = a;
        b.origin.x = -Px(82);
        let a = a.to_box2d();
        let b = b.to_box2d();

        assert!(!Orientation2D::Above.box_is(a, b));
        assert!(!Orientation2D::Below.box_is(a, b));
        assert!(Orientation2D::Left.box_is(a, b));
        assert!(!Orientation2D::Right.box_is(a, b));
    }

    #[test]
    fn orientation_box_right() {
        let a = PxRect::from_size(PxSize::splat(Px(40)));
        let mut b = a;
        b.origin.x = Px(42);
        let a = a.to_box2d();
        let b = b.to_box2d();

        assert!(!Orientation2D::Above.box_is(a, b));
        assert!(!Orientation2D::Below.box_is(a, b));
        assert!(!Orientation2D::Left.box_is(a, b));
        assert!(Orientation2D::Right.box_is(a, b));
    }

    #[test]
    fn length_composite_parser_2() {
        let mut parser = LengthCompositeParser::new("(10%, 20%)").unwrap();
        assert_eq!(parser.next().unwrap(), Length::from(10.pct()));
        assert_eq!(parser.expect_last().unwrap(), Length::from(20.pct()));
    }

    #[test]
    fn length_composite_parser_1() {
        let parser = LengthCompositeParser::new("10px").unwrap();
        assert_eq!(parser.expect_last().unwrap(), 10.px());
    }
}
