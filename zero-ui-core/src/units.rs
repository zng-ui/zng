//! Angle, factor, length, time, byte and resolution units.

#[doc(inline)]
pub use zero_ui_view_api::units::*;

mod alignment;
pub use alignment::*;

mod angle;
pub use angle::*;

mod constraints;
pub use constraints::*;

mod byte;
pub use byte::*;

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

mod text;
pub use text::*;

mod time;
pub use time::*;

mod transform;
pub use transform::*;

mod vector;
pub use vector::*;

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

            impl<$($N),+> crate::var::IntoVar<$For> for ($($N),+)
            where
            $($N: Into<Length> + Clone,)+
            {
                type Var = crate::var::LocalVar<$For>;

                $(#[$docs])*
                fn into_var(self) -> Self::Var {
                    crate::var::LocalVar(self.into())
                }
            }
        )+
    };
}
use impl_length_comp_conversions;

/// Comparable key that represents the absolute distance between two pixel points.
///
/// Computing the actual distance only for comparison is expensive, this key avoids the conversion to float and square-root operation.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DistanceKey(u64);
impl DistanceKey {
    /// Value that is always greater than any distance key.
    pub const NONE_MAX: DistanceKey = DistanceKey(u64::MAX);

    /// Value that is always smaller than any distance key.
    pub const NONE_MIN: DistanceKey = DistanceKey(0);

    /// Maximum distance.
    pub const MAX: DistanceKey = DistanceKey((Px::MAX.0 as u64).pow(2));

    /// Minimum distance.
    pub const MIN: DistanceKey = DistanceKey(1);

    /// New distance key computed from two points.
    pub fn from_points(a: PxPoint, b: PxPoint) -> Self {
        let pa = ((a.x - b.x).0.unsigned_abs() as u64).pow(2);
        let pb = ((a.y - b.y).0.unsigned_abs() as u64).pow(2);

        Self((pa + pb) + 1)
    }

    /// New distance key from already computed actual distance.
    ///
    /// Note that computing the actual distance is slower then using [`from_points`] to compute just the distance key.
    ///
    /// [`from_points`]: Self::from_points
    pub fn from_distance(d: Px) -> Self {
        let p = (d.0.unsigned_abs() as u64).pow(2);
        Self(p + 1)
    }

    /// If the key is the [`NONE_MAX`] or [`NONE_MIN`].
    ///
    /// [`NONE_MAX`]: Self::NONE_MAX
    /// [`NONE_MIN`]: Self::NONE_MIN
    pub fn is_none(self) -> bool {
        self == Self::NONE_MAX || self == Self::NONE_MIN
    }

    /// Completes the distance calculation.
    pub fn distance(self) -> Option<Px> {
        if self.is_none() {
            None
        } else {
            let p = self.0 - 1;
            let d = (p as f64).sqrt();

            Some(Px(d.round() as i32))
        }
    }
}

/// Orientation of two 2D items.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Orientation2D {
    /// Point is above the origin.
    Above,
    /// Point is to the right of the origin.
    Right,
    /// Point is below the origin.
    Below,
    /// Point is to the left of the origin.
    Left,
}
impl Orientation2D {
    /// Check if `point` is orientation from `origin`.
    ///
    /// Returns `true` if  the point is hit by a 45º frustum cast from origin in the direction defined by the orientation.
    pub fn point_is(self, origin: PxPoint, point: PxPoint) -> bool {
        let (a, b, c, d) = match self {
            Orientation2D::Above => (point.y, origin.y, point.x, origin.x),
            Orientation2D::Right => (origin.x, point.x, point.y, origin.y),
            Orientation2D::Below => (origin.y, point.y, point.x, origin.x),
            Orientation2D::Left => (point.x, origin.x, point.y, origin.y),
        };

        let mut is = false;

        // for 'Above' this is:
        // is above line?
        if a < b {
            // is to the right?
            if c > d {
                // is in the 45º 'frustum'
                // │?╱
                // │╱__
                is = c <= d + (b - a);
            } else {
                //  ╲?│
                // __╲│
                is = c >= d - (b - a);
            }
        }

        is
    }

    /// Check if `b` is orientation from `origin`.
    ///
    /// Returns `true` if the box `b` collides with the box `origin` in the direction defined by orientation. Also
    /// returns `true` if the boxes already overlap.
    pub fn box_is(self, origin: PxBox, b: PxBox) -> bool {
        fn d_intersects(a_min: Px, a_max: Px, b_min: Px, b_max: Px) -> bool {
            a_min < b_max && a_max > b_min
        }
        match self {
            Orientation2D::Above => b.min.y <= origin.min.y && d_intersects(b.min.x, b.max.x, origin.min.x, origin.max.x),
            Orientation2D::Left => b.min.x <= origin.min.x && d_intersects(b.min.y, b.max.y, origin.min.y, origin.max.y),
            Orientation2D::Below => b.max.y >= origin.max.y && d_intersects(b.min.x, b.max.x, origin.min.x, origin.max.x),
            Orientation2D::Right => b.max.x >= origin.max.x && d_intersects(b.min.y, b.max.y, origin.min.y, origin.max.y),
        }
    }

    /// Iterator that yields quadrants for efficient search in a quad-tree, if a point is inside a quadrant and
    /// passes the [`Orientation2D::point_is`] check it is in the orientation, them if it is within the `max_distance` it is valid.
    pub fn search_bounds(self, origin: PxPoint, max_distance: Px, spatial_bounds: PxBox) -> impl Iterator<Item = PxBox> {
        let mut bounds = PxRect::new(origin, PxSize::splat(max_distance));
        match self {
            Orientation2D::Above => {
                bounds.origin.x -= max_distance / Px(2);
                bounds.origin.y -= max_distance;
            }
            Orientation2D::Right => bounds.origin.y -= max_distance / Px(2),
            Orientation2D::Below => bounds.origin.x -= max_distance / Px(2),
            Orientation2D::Left => {
                bounds.origin.y -= max_distance / Px(2);
                bounds.origin.x -= max_distance;
            }
        }

        // oriented search is a 45º square in the direction specified, so we grow and cut the search quadrant like
        // in the "nearest with bounds" algorithm, but then cut again to only the part that fully overlaps the 45º
        // square, points found are then matched with the `Orientation2D::is` method.

        let max_quad = spatial_bounds.intersection_unchecked(&bounds.to_box2d());
        let mut is_none = max_quad.is_empty();

        let mut source_quad = PxRect::new(origin - PxVector::splat(Px(64)), PxSize::splat(Px(128))).to_box2d();
        let mut search_quad = source_quad.intersection_unchecked(&max_quad);
        is_none |= search_quad.is_empty();

        let max_diameter = max_distance * Px(2);

        let mut is_first = true;

        std::iter::from_fn(move || {
            let source_width = source_quad.width();
            if is_none {
                None
            } else if is_first {
                is_first = false;
                Some(search_quad)
            } else if source_width >= max_diameter {
                is_none = true;
                None
            } else {
                source_quad = source_quad.inflate(source_width, source_width);
                let mut new_search = source_quad.intersection_unchecked(&max_quad);
                if new_search == source_quad || new_search.is_empty() {
                    is_none = true; // filled bounds
                    return None;
                }

                match self {
                    Orientation2D::Above => {
                        new_search.max.y = search_quad.min.y;
                    }
                    Orientation2D::Right => {
                        new_search.min.x = search_quad.max.x;
                    }
                    Orientation2D::Below => {
                        new_search.min.y = search_quad.max.y;
                    }
                    Orientation2D::Left => {
                        new_search.max.x = search_quad.min.x;
                    }
                }

                search_quad = new_search;

                Some(search_quad)
            }
        })
    }
}
impl crate::var::IntoVar<Option<Orientation2D>> for Orientation2D {
    type Var = crate::var::LocalVar<Option<Orientation2D>>;

    fn into_var(self) -> Self::Var {
        crate::var::LocalVar(Some(self))
    }
}
impl crate::var::IntoValue<Option<Orientation2D>> for Orientation2D {}

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

    use crate::{
        app::App,
        context::{LayoutMetrics, LAYOUT},
    };

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
        let _app = App::minimal().run_headless(false);

        let l = (Length::from(200) - 100.pct()).abs();
        let metrics = LayoutMetrics::new(1.fct(), PxSize::new(Px(600), Px(400)), Px(0));
        let l = LAYOUT.with_context(metrics, || l.layout_x());

        assert_eq!(l.0, (200i32 - 600i32).abs());
    }

    #[test]
    pub fn length_expr_clamp() {
        let _app = App::minimal().run_headless(false);

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
