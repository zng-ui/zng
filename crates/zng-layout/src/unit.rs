//! Angle, factor, length, time, byte and resolution units.

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
                type Var = zng_var::LocalVar<$For>;

                $(#[$docs])*
                fn into_var(self) -> Self::Var {
                    zng_var::LocalVar(self.into())
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
}
