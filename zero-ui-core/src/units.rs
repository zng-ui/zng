//! Angle, factor and length units.

use derive_more as dm;
use std::fmt::Write;
use std::{f32::consts::*, fmt, time::Duration};
use std::{mem, ops};
use webrender_api::units as wr;

use crate::context::LayoutMetrics;
use crate::var::{impl_from_and_into_var, IntoVar, OwnedVar};

/// Minimal difference between values in around the 0.0..=1.0 scale.
const EPSILON: f32 = 0.00001;
/// Minimal difference between values in around the 1.0..=100.0 scale.
const EPSILON_100: f32 = 0.001;

/// If a layout value indicates that any size is available during layout.
///
/// Returns `true` if [`UiNode::measure`] implementers must return a finite size that is the nodes ideal
/// size given infinite space. Nodes that only fill the available space and have no inherent size must collapse (zero size).
///
/// # Examples
///
/// If the node can decide on a size without a finite `available_size` it must return that size:
///
/// ```
/// # use zero_ui_core::{*, units::*, context::*};
/// # struct FillOrMaxNode { max_size: LayoutSize }
/// #[impl_ui_node(none)]
/// impl UiNode for FillOrMaxNode {
///     fn measure(&mut self, ctx: &mut LayoutContext, available_size: LayoutSize) -> LayoutSize {
///         let mut desired_size = available_size;
///         if is_layout_any_size(desired_size.width) {
///             desired_size.width = self.max_size.width;
///         }
///         if is_layout_any_size(desired_size.height) {
///             desired_size.height = self.max_size.height;
///         }
///         desired_size
///     }
/// }
/// ```
///
/// If the node cannot decide on a size it must collapse:
///
/// ```
/// # use zero_ui_core::{*, units::*, context::*};
/// # struct FillNode { }
/// #[impl_ui_node(none)]
/// impl UiNode for FillNode {
///     fn measure(&mut self, ctx: &mut LayoutContext, available_size: LayoutSize) -> LayoutSize {
///         let mut desired_size = available_size;
///         if is_layout_any_size(desired_size.width) {
///             desired_size.width = 0.0;
///         }
///         if is_layout_any_size(desired_size.height) {
///             desired_size.height = 0.0;
///         }
///         desired_size
///     }
/// }
/// ```
///
/// Note that both nodes don't delegate the measure to inner nodes, these *leaf* nodes need to check
/// the size, because returning a not-finite value is an error. It is ok to delegate the layout-any-size
/// value to an inner node, and that is what happens in the default delegating implementation of measure.
/// See [`UiNode::measure`] for more information about how to implement layout nodes.
///
/// [`UiNode::measure`]: crate::UiNode::measure
#[inline]
pub fn is_layout_any_size(value: f32) -> bool {
    value.is_infinite() && value.is_sign_positive()
}

/// Value that indicates that any size is available during layout.
///
/// Use [`is_layout_any_size`] to check if a layout value indicates this.
///
/// # Examples
///
/// A layout node can use this value to tell its children that they have any space available:
///
/// ```
/// # use zero_ui_core::{*, units::*, context::*};
/// # struct VerticalStackNode<C> { children: C }
/// #[impl_ui_node(children)]
/// impl<C: UiNodeList> UiNode for VerticalStackNode<C> {
///     fn measure(
///         &mut self,
///         ctx: &mut LayoutContext,
///         mut available_size: LayoutSize
///     ) -> LayoutSize {
///
///         // children can be any height
///         available_size.height = LAYOUT_ANY_SIZE;
///
///         let mut desired_size = LayoutSize::zero();
///         self.children.measure_all(
///             ctx,
///             |_, _| available_size,
///             |i, child_ds, _| {
///                 desired_size.width = desired_size.width.max(child_ds.width);
///
///                 // each child returns a finite height
///                 desired_size.height += child_ds.height;
///
///             },
///         );
///         desired_size
///     }
/// }
/// ```
///
/// The child nodes in the example return any finite height. See [`UiNode::measure`] and [`UiNode::arrange`]
/// for more information about how to implement layout panel nodes.
///
/// [`UiNode::measure`]: crate::UiNode::measure
/// [`UiNode::arrange`]: crate::UiNode::arrange
pub const LAYOUT_ANY_SIZE: f32 = f32::INFINITY;

/// [`f32`] equality used in [`units`](crate::units).
///
/// * [`NaN`](f32::is_nan) values are equal.
/// * [`INFINITY`](f32::INFINITY) values are equal.
/// * [`NEG_INFINITY`](f32::NEG_INFINITY) values are equal.
/// * Finite values are equal if the difference is less than `epsilon`.
pub fn about_eq(a: f32, b: f32, epsilon: f32) -> bool {
    if a.is_nan() {
        b.is_nan()
    } else if a.is_infinite() {
        b.is_infinite() && a.is_sign_positive() == b.is_sign_positive()
    } else {
        (a - b).abs() < epsilon
    }
}

/// Angle in radians.
///
/// See [`AngleUnits`] for more details.
///
/// # Equality
///
/// Equality is determined using [`about_eq`] with `0.00001` epsilon.
#[derive(Copy, Clone, dm::Add, dm::AddAssign, dm::Sub, dm::SubAssign, dm::Mul, dm::MulAssign, dm::Div, dm::DivAssign, dm::Neg)]
pub struct AngleRadian(pub f32);
impl AngleRadian {
    /// Radians in `[0.0 ..= TAU]`.
    #[inline]
    pub fn modulo(self) -> Self {
        AngleGradian::from(self).modulo().into()
    }
    /// Change type to [`LayoutAngle`].
    ///
    /// Note that layout angle is in radians so no computation happens.
    #[inline]
    pub fn to_layout(self) -> LayoutAngle {
        self.into()
    }
}
impl PartialEq for AngleRadian {
    fn eq(&self, other: &Self) -> bool {
        about_eq(self.0, other.0, EPSILON)
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
/// Equality is determined using [`about_eq`] with `0.001` epsilon.
#[derive(Copy, Clone, dm::Add, dm::AddAssign, dm::Sub, dm::SubAssign, dm::Mul, dm::MulAssign, dm::Div, dm::DivAssign, dm::Neg)]
pub struct AngleGradian(pub f32);
impl AngleGradian {
    /// Gradians in `[0.0 ..= 400.0]`.
    #[inline]
    pub fn modulo(self) -> Self {
        AngleGradian(self.0.rem_euclid(400.0))
    }
}
impl PartialEq for AngleGradian {
    fn eq(&self, other: &Self) -> bool {
        about_eq(self.0, other.0, EPSILON_100)
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
/// Equality is determined using [`about_eq`] with `0.001` epsilon.
#[derive(Copy, Clone, dm::Add, dm::AddAssign, dm::Sub, dm::SubAssign, dm::Mul, dm::MulAssign, dm::Div, dm::DivAssign, dm::Neg)]
pub struct AngleDegree(pub f32);
impl AngleDegree {
    /// Degrees in `[0.0 ..= 360.0]`.
    #[inline]
    pub fn modulo(self) -> Self {
        AngleDegree(self.0.rem_euclid(360.0))
    }
}
impl PartialEq for AngleDegree {
    fn eq(&self, other: &Self) -> bool {
        about_eq(self.0, other.0, EPSILON_100)
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
/// Equality is determined using [`about_eq`] with `0.00001` epsilon.
#[derive(Copy, Clone, dm::Add, dm::AddAssign, dm::Sub, dm::SubAssign, dm::Mul, dm::MulAssign, dm::Div, dm::DivAssign, dm::Neg)]
pub struct AngleTurn(pub f32);
impl AngleTurn {
    /// Turns in `[0.0 ..= 1.0]`.
    #[inline]
    pub fn modulo(self) -> Self {
        AngleTurn(self.0.rem_euclid(1.0))
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
        about_eq(self.0, other.0, EPSILON)
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
pub type LayoutAngle = webrender_api::euclid::Angle<f32>;
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
/// # use zero_ui_core::units::*;
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
///
/// # Equality
///
/// Equality is determined using [`about_eq`] with `0.001` epsilon.
#[derive(Copy, Clone, dm::Add, dm::AddAssign, dm::Sub, dm::SubAssign)]
pub struct FactorPercent(pub f32);
impl FactorPercent {
    /// Clamp factor to [0.0..=100.0] range.
    #[inline]
    pub fn clamp_range(self) -> Self {
        FactorPercent(self.0.max(0.0).min(100.0))
    }
}
impl PartialEq for FactorPercent {
    fn eq(&self, other: &Self) -> bool {
        about_eq(self.0, other.0, EPSILON_100)
    }
}
impl_from_and_into_var! {
    fn from(n: FactorNormal) -> FactorPercent {
        FactorPercent(n.0 * 100.0)
    }
}
impl fmt::Debug for FactorPercent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_tuple("FactorPercent").field(&self.0).finish()
        } else {
            write!(f, "{}.pct()", self.0)
        }
    }
}
impl fmt::Display for FactorPercent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}%", self.0)
    }
}

/// Normalized multiplication factor (0.0-1.0).
///
/// See [`FactorUnits`] for more details.
///
/// # Equality
///
/// Equality is determined using [`about_eq`] with `0.00001` epsilon.
#[derive(Copy, Clone, dm::Add, dm::AddAssign, dm::Sub, dm::SubAssign)]
pub struct FactorNormal(pub f32);
impl FactorNormal {
    /// Clamp factor to [0.0..=1.0] range.
    #[inline]
    pub fn clamp_range(self) -> Self {
        FactorNormal(self.0.max(0.0).min(1.0))
    }

    /// Returns the maximum of two factors.
    #[inline]
    pub fn max(self, other: impl Into<FactorNormal>) -> FactorNormal {
        FactorNormal(self.0.max(other.into().0))
    }

    /// Returns the minimum of two factors.
    #[inline]
    pub fn min(self, other: impl Into<FactorNormal>) -> FactorNormal {
        FactorNormal(self.0.min(other.into().0))
    }

    /// Returns `self` if `min <= self <= max`, returns `min` if `self < min` or returns `max` if `self > max`.
    #[inline]
    pub fn clamp(self, min: impl Into<FactorNormal>, max: impl Into<FactorNormal>) -> FactorNormal {
        self.min(max).max(min)
    }

    /// Computes the absolute value of self.
    #[inline]
    pub fn abs(self) -> FactorNormal {
        FactorNormal(self.0.abs())
    }
}
impl PartialEq for FactorNormal {
    fn eq(&self, other: &Self) -> bool {
        about_eq(self.0, other.0, EPSILON)
    }
}
impl ops::Mul for FactorNormal {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        FactorNormal(self.0 * rhs.0)
    }
}
impl ops::MulAssign for FactorNormal {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}
impl ops::Div for FactorNormal {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        FactorNormal(self.0 / rhs.0)
    }
}
impl ops::DivAssign for FactorNormal {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}
impl_from_and_into_var! {
    fn from(percent: FactorPercent) -> FactorNormal {
        FactorNormal(percent.0 / 100.0)
    }

    fn from(f: f32) -> FactorNormal {
        FactorNormal(f)
    }

    fn from(f: f64) -> FactorNormal {
        FactorNormal(f as f32)
    }

    /// | Input  | Output  |
    /// |--------|---------|
    /// |`true`  | `1.0`   |
    /// |`false` | `0.0`   |
    fn from(b: bool) -> FactorNormal {
        FactorNormal(if b { 1.0 } else { 0.0 })
    }
}
impl fmt::Debug for FactorNormal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_tuple("FactorNormal").field(&self.0).finish()
        } else {
            write!(f, "{}.normal()", self.0)
        }
    }
}
impl fmt::Display for FactorNormal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// Extension methods for initializing factor units.
///
/// This trait is implemented for [`f32`] and [`u32`] allowing initialization of factor unit types using the `<number>.<unit>()` syntax.
///
/// # Example
///
/// ```
/// # use zero_ui_core::units::*;
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
///
/// # Equality
///
/// Two lengths are equal if they are of the same variant and if:
///
/// * `Exact` lengths uses [`about_eq`] with `0.001` epsilon.
/// * `Relative`, `Em`, `RootEm` lengths use the [`FactorNormal`] equality.
/// * Viewport lengths uses [`about_eq`] with `0.00001` epsilon.
#[derive(Clone)]
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
    /// Unresolved expression.
    Expr(Box<LengthExpr>),
}
impl<L: Into<Length>> ops::Add<L> for Length {
    type Output = Length;

    fn add(self, rhs: L) -> Self::Output {
        use Length::*;

        match (self, rhs.into()) {
            (Exact(a), Exact(b)) => Exact(a + b),
            (Relative(a), Relative(b)) => Relative(a + b),
            (Em(a), Em(b)) => Em(a + b),
            (RootEm(a), RootEm(b)) => RootEm(a + b),
            (ViewportWidth(a), ViewportWidth(b)) => ViewportWidth(a + b),
            (ViewportHeight(a), ViewportHeight(b)) => ViewportHeight(a + b),
            (ViewportMin(a), ViewportMin(b)) => ViewportMin(a + b),
            (ViewportMax(a), ViewportMax(b)) => ViewportMax(a + b),
            (a, b) => Length::Expr(Box::new(LengthExpr::Add(a, b))),
        }
    }
}
impl<L: Into<Length>> ops::AddAssign<L> for Length {
    fn add_assign(&mut self, rhs: L) {
        let lhs = mem::replace(self, Length::Exact(f32::NAN));
        *self = lhs + rhs.into();
    }
}
impl<L: Into<Length>> ops::Sub<L> for Length {
    type Output = Length;

    fn sub(self, rhs: L) -> Self::Output {
        use Length::*;

        match (self, rhs.into()) {
            (Exact(a), Exact(b)) => Exact(a - b),
            (Relative(a), Relative(b)) => Relative(a - b),
            (Em(a), Em(b)) => Em(a - b),
            (RootEm(a), RootEm(b)) => RootEm(a - b),
            (ViewportWidth(a), ViewportWidth(b)) => ViewportWidth(a - b),
            (ViewportHeight(a), ViewportHeight(b)) => ViewportHeight(a - b),
            (ViewportMin(a), ViewportMin(b)) => ViewportMin(a - b),
            (ViewportMax(a), ViewportMax(b)) => ViewportMax(a - b),
            (a, b) => Length::Expr(Box::new(LengthExpr::Sub(a, b))),
        }
    }
}
impl<L: Into<Length>> ops::SubAssign<L> for Length {
    fn sub_assign(&mut self, rhs: L) {
        let lhs = mem::replace(self, Length::Exact(f32::NAN));
        *self = lhs - rhs.into();
    }
}
impl<F: Into<FactorNormal>> ops::Mul<F> for Length {
    type Output = Length;

    fn mul(self, rhs: F) -> Self::Output {
        use Length::*;
        let rhs = rhs.into();
        match self {
            Exact(e) => Exact(e * rhs.0),
            Relative(r) => Relative(r * rhs),
            Em(e) => Em(e * rhs),
            RootEm(e) => RootEm(e * rhs),
            ViewportWidth(w) => ViewportWidth(w * rhs.0),
            ViewportHeight(h) => ViewportHeight(h * rhs.0),
            ViewportMin(m) => ViewportMin(m * rhs.0),
            ViewportMax(m) => ViewportMax(m * rhs.0),
            e => Expr(Box::new(LengthExpr::Mul(e, rhs))),
        }
    }
}
impl<F: Into<FactorNormal>> ops::MulAssign<F> for Length {
    fn mul_assign(&mut self, rhs: F) {
        let lhs = mem::replace(self, Length::Exact(f32::NAN));
        *self = lhs * rhs.into();
    }
}
impl<F: Into<FactorNormal>> ops::Div<F> for Length {
    type Output = Length;

    fn div(self, rhs: F) -> Self::Output {
        use Length::*;

        let rhs = rhs.into();

        match self {
            Exact(e) => Exact(e / rhs.0),
            Relative(r) => Relative(r / rhs),
            Em(e) => Em(e / rhs),
            RootEm(e) => RootEm(e / rhs),
            ViewportWidth(w) => ViewportWidth(w / rhs.0),
            ViewportHeight(h) => ViewportHeight(h / rhs.0),
            ViewportMin(m) => ViewportMin(m / rhs.0),
            ViewportMax(m) => ViewportMax(m / rhs.0),
            e => Expr(Box::new(LengthExpr::Mul(e, rhs))),
        }
    }
}
impl<F: Into<FactorNormal>> ops::DivAssign<F> for Length {
    fn div_assign(&mut self, rhs: F) {
        let lhs = mem::replace(self, Length::Exact(f32::NAN));
        *self = lhs / rhs.into();
    }
}
impl Default for Length {
    /// Exact `0`.
    fn default() -> Self {
        Length::Exact(0.0)
    }
}
impl PartialEq for Length {
    fn eq(&self, other: &Self) -> bool {
        use Length::*;
        match (self, other) {
            (Exact(a), Exact(b)) => about_eq(*a, *b, EPSILON_100),

            (Relative(a), Relative(b)) | (Em(a), Em(b)) | (RootEm(a), RootEm(b)) => a == b,

            (ViewportWidth(a), ViewportWidth(b))
            | (ViewportHeight(a), ViewportHeight(b))
            | (ViewportMin(a), ViewportMin(b))
            | (ViewportMax(a), ViewportMax(b)) => about_eq(*a, *b, EPSILON),

            (Expr(a), Expr(b)) => a == b,

            _ => false,
        }
    }
}
impl fmt::Debug for Length {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Length::*;
        if f.alternate() {
            match self {
                Exact(e) => f.debug_tuple("Length::Exact").field(e).finish(),
                Relative(e) => f.debug_tuple("Length::Relative").field(e).finish(),
                Em(e) => f.debug_tuple("Length::Em").field(e).finish(),
                RootEm(e) => f.debug_tuple("Length::RootEm").field(e).finish(),
                ViewportWidth(e) => f.debug_tuple("Length::ViewportWidth").field(e).finish(),
                ViewportHeight(e) => f.debug_tuple("Length::ViewportHeight").field(e).finish(),
                ViewportMin(e) => f.debug_tuple("Length::ViewportMin").field(e).finish(),
                ViewportMax(e) => f.debug_tuple("Length::ViewportMax").field(e).finish(),
                Expr(e) => f.debug_tuple("Length::Expr").field(e).finish(),
            }
        } else {
            match self {
                Exact(e) => write!(f, "{}", e),
                Relative(e) => write!(f, "{}.pct()", e.0 * 100.0),
                Em(e) => write!(f, "{}.em()", e.0),
                RootEm(e) => write!(f, "{}.rem()", e.0),
                ViewportWidth(e) => write!(f, "{}.vw()", e),
                ViewportHeight(e) => write!(f, "{}.vh()", e),
                ViewportMin(e) => write!(f, "{}.vmin()", e),
                ViewportMax(e) => write!(f, "{}.vmax()", e),
                Expr(e) => write!(f, "{}", e),
            }
        }
    }
}
impl fmt::Display for Length {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Length::*;
        match self {
            Exact(l) => fmt::Display::fmt(l, f),
            Relative(n) => write!(f, "{:.*}%", f.precision().unwrap_or(0), n.0 * 100.0),
            Em(e) => write!(f, "{}em", e),
            RootEm(re) => write!(f, "{}rem", re),
            ViewportWidth(vw) => write!(f, "{}vw", vw),
            ViewportHeight(vh) => write!(f, "{}vh", vh),
            ViewportMin(vmin) => write!(f, "{}vmin", vmin),
            ViewportMax(vmax) => write!(f, "{}vmax", vmax),
            Expr(e) => write!(f, "{}", e),
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

    /// Conversion to [`Length::Exact`]
    fn from(l: LayoutLength) -> Length {
        Length::Exact(l.get())
    }
}
impl Length {
    /// Length of exact zero.
    #[inline]
    pub const fn zero() -> Length {
        Length::Exact(0.0)
    }

    /// Length that fills the available space.
    #[inline]
    pub const fn fill() -> Length {
        Length::Relative(FactorNormal(1.0))
    }

    /// Length that fills 50% of the available space.
    #[inline]
    pub const fn half() -> Length {
        Length::Relative(FactorNormal(0.5))
    }

    /// Exact length in font units.
    #[inline]
    pub fn pt(font_pt: f32) -> Length {
        // make this const when https://github.com/rust-lang/rust/issues/57241
        Length::Exact(font_pt * 96.0 / 72.0)
    }

    /// Returns a length that resolves to the maximum layout length between `self` and `other`.
    pub fn max(&self, other: impl Into<Length>) -> Length {
        use Length::*;
        match (self.clone(), other.into()) {
            (Exact(a), Exact(b)) => Exact(a.max(b)),
            (Relative(a), Relative(b)) => Relative(a.max(b)),
            (Em(a), Em(b)) => Em(a.max(b)),
            (RootEm(a), RootEm(b)) => RootEm(a.max(b)),
            (ViewportWidth(a), ViewportWidth(b)) => ViewportWidth(a.max(b)),
            (ViewportHeight(a), ViewportHeight(b)) => ViewportHeight(a.max(b)),
            (ViewportMin(a), ViewportMin(b)) => ViewportMin(a.max(b)),
            (ViewportMax(a), ViewportMax(b)) => ViewportMax(a.max(b)),
            (a, b) => Length::Expr(Box::new(LengthExpr::Max(a, b))),
        }
    }

    /// Returns a length that resolves to the minimum layout length between `self` and `other`.
    pub fn min(&self, other: impl Into<Length>) -> Length {
        use Length::*;
        match (self.clone(), other.into()) {
            (Exact(a), Exact(b)) => Exact(a.min(b)),
            (Relative(a), Relative(b)) => Relative(a.min(b)),
            (Em(a), Em(b)) => Em(a.min(b)),
            (RootEm(a), RootEm(b)) => RootEm(a.min(b)),
            (ViewportWidth(a), ViewportWidth(b)) => ViewportWidth(a.min(b)),
            (ViewportHeight(a), ViewportHeight(b)) => ViewportHeight(a.min(b)),
            (ViewportMin(a), ViewportMin(b)) => ViewportMin(a.min(b)),
            (ViewportMax(a), ViewportMax(b)) => ViewportMax(a.min(b)),
            (a, b) => Length::Expr(Box::new(LengthExpr::Min(a, b))),
        }
    }

    /// Returns a length that constrains the computed layout length between `min` and `max`.
    #[inline]
    pub fn clamp(&self, min: impl Into<Length>, max: impl Into<Length>) -> Length {
        self.max(min).min(max)
    }

    /// Returns a length that computes the absolute layout length of `self`.
    #[inline]
    pub fn abs(&self) -> Length {
        use Length::*;
        match self {
            Exact(e) => Exact(e.abs()),
            Relative(r) => Relative(r.abs()),
            Em(e) => Em(e.abs()),
            RootEm(r) => RootEm(r.abs()),
            ViewportWidth(w) => ViewportWidth(w.abs()),
            ViewportHeight(h) => ViewportHeight(h.abs()),
            ViewportMin(m) => ViewportMin(m.abs()),
            ViewportMax(m) => ViewportMax(m.abs()),
            Expr(e) => Expr(Box::new(LengthExpr::Abs(e.clone()))),
        }
    }

    /// Compute the length at a context.
    pub fn to_layout(&self, ctx: &LayoutMetrics, available_size: LayoutLength) -> LayoutLength {
        use Length::*;
        let l = match self {
            Exact(l) => *l,
            Relative(f) => available_size.get() * f.0,
            Em(f) => ctx.font_size * f.0,
            RootEm(f) => ctx.root_font_size * f.0,
            ViewportWidth(p) => p * ctx.viewport_size.width / 100.0,
            ViewportHeight(p) => p * ctx.viewport_size.height / 100.0,
            ViewportMin(p) => p * ctx.viewport_min() / 100.0,
            ViewportMax(p) => p * ctx.viewport_max() / 100.0,
            Expr(e) => e.to_layout(ctx, available_size).0,
        };
        LayoutLength::new(ctx.pixel_grid.snap(l))
    }

    /// If this length is zero in any finite layout context.
    ///
    /// If is [`Expr`] returns `false`.
    ///
    /// [`Expr`]: Length::Expr
    pub fn is_zero(&self) -> bool {
        use Length::*;
        match self {
            Exact(l) => l.abs() < EPSILON_100,
            Relative(f) => f.0.abs() < EPSILON,
            Em(f) => f.0.abs() < EPSILON,
            RootEm(f) => f.0.abs() < EPSILON,
            ViewportWidth(p) => p.abs() < EPSILON,
            ViewportHeight(p) => p.abs() < EPSILON,
            ViewportMin(p) => p.abs() < EPSILON,
            ViewportMax(p) => p.abs() < EPSILON,
            Expr(_) => false,
        }
    }

    /// If this length is `NaN` in any finite layout context.
    ///
    /// If is [`Expr`] returns `false`.
    ///
    /// [`Expr`]: Length::Expr
    pub fn is_nan(&self) -> bool {
        use Length::*;
        match self {
            Exact(l) => l.is_nan(),
            Relative(f) => f.0.is_nan(),
            Em(f) => f.0.is_nan(),
            RootEm(f) => f.0.is_nan(),
            ViewportWidth(p) => p.is_nan(),
            ViewportHeight(p) => p.is_nan(),
            ViewportMin(p) => p.is_nan(),
            ViewportMax(p) => p.is_nan(),
            Expr(_) => false,
        }
    }

    /// If this length is a finite number in any finite layout context.
    ///
    /// If is [`Expr`] returns `false`.
    ///
    /// [`Expr`]: Length::Expr
    pub fn is_finite(&self) -> bool {
        use Length::*;
        match self {
            Exact(e) => e.is_finite(),
            Relative(f) => f.0.is_finite(),
            Em(f) => f.0.is_finite(),
            RootEm(f) => f.0.is_finite(),
            ViewportWidth(p) => p.is_finite(),
            ViewportHeight(p) => p.is_finite(),
            ViewportMin(p) => p.is_finite(),
            ViewportMax(p) => p.is_finite(),
            Expr(_) => false,
        }
    }

    /// If this length is an infinite number in any finite layout context.
    ///
    /// If is [`Expr`] returns `false`.
    ///
    /// [`Expr`]: Length::Expr
    pub fn is_infinite(&self) -> bool {
        use Length::*;
        match self {
            Exact(e) => e.is_infinite(),
            Relative(f) => f.0.is_infinite(),
            Em(f) => f.0.is_infinite(),
            RootEm(f) => f.0.is_infinite(),
            ViewportWidth(p) => p.is_infinite(),
            ViewportHeight(p) => p.is_infinite(),
            ViewportMin(p) => p.is_infinite(),
            ViewportMax(p) => p.is_infinite(),
            Expr(_) => false,
        }
    }
}

/// Represents an unresolved [`Length`] expression.
#[derive(Clone, PartialEq)]
pub enum LengthExpr {
    /// Sums the both layout length.
    Add(Length, Length),
    /// Subtracts the first layout length from the second.
    Sub(Length, Length),
    /// Multiplies the layout length by the factor.
    Mul(Length, FactorNormal),
    /// Divide the layout length by the factor.
    Div(Length, FactorNormal),
    /// Maximum layout length.
    Max(Length, Length),
    /// Minimum layout length.
    Min(Length, Length),
    /// Computes the absolute layout length.
    Abs(Box<LengthExpr>),
}
impl LengthExpr {
    /// Evaluate the expression at a layout context.
    pub fn to_layout(&self, ctx: &LayoutMetrics, available_size: LayoutLength) -> LayoutLength {
        use LengthExpr::*;
        match self {
            Add(a, b) => a.to_layout(ctx, available_size) + b.to_layout(ctx, available_size),
            Sub(a, b) => a.to_layout(ctx, available_size) - b.to_layout(ctx, available_size),
            Mul(l, s) => l.to_layout(ctx, available_size) * s.0,
            Div(l, s) => l.to_layout(ctx, available_size) / s.0,
            Max(a, b) => {
                let a = a.to_layout(ctx, available_size).0;
                let b = b.to_layout(ctx, available_size).0;
                LayoutLength::new(a.max(b))
            }
            Min(a, b) => {
                let a = a.to_layout(ctx, available_size).0;
                let b = b.to_layout(ctx, available_size).0;
                LayoutLength::new(a.min(b))
            }
            Abs(e) => LayoutLength::new(e.to_layout(ctx, available_size).0.abs()),
        }
    }
}
impl fmt::Debug for LengthExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use LengthExpr::*;
        if f.alternate() {
            match self {
                Add(a, b) => f.debug_tuple("LengthExpr::Add").field(a).field(b).finish(),
                Sub(a, b) => f.debug_tuple("LengthExpr::Sub").field(a).field(b).finish(),
                Mul(l, s) => f.debug_tuple("LengthExpr::Mul").field(l).field(s).finish(),
                Div(l, s) => f.debug_tuple("LengthExpr::Div").field(l).field(s).finish(),
                Max(a, b) => f.debug_tuple("LengthExpr::Max").field(a).field(b).finish(),
                Min(a, b) => f.debug_tuple("LengthExpr::Min").field(a).field(b).finish(),
                Abs(e) => f.debug_tuple("LengthExpr::Abs").field(e).finish(),
            }
        } else {
            match self {
                Add(a, b) => write!(f, "({:?} + {:?})", a, b),
                Sub(a, b) => write!(f, "({:?} - {:?})", a, b),
                Mul(l, s) => write!(f, "({:?} * {:?}.pct())", l, s.0 * 100.0),
                Div(l, s) => write!(f, "({:?} / {:?}.pct())", l, s.0 * 100.0),
                Max(a, b) => write!(f, "max({:?}, {:?})", a, b),
                Min(a, b) => write!(f, "min({:?}, {:?})", a, b),
                Abs(e) => write!(f, "abs({:?})", e),
            }
        }
    }
}
impl fmt::Display for LengthExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use LengthExpr::*;
        match self {
            Add(a, b) => write!(f, "({} + {})", a, b),
            Sub(a, b) => write!(f, "({} - {})", a, b),
            Mul(l, s) => write!(f, "({} * {}%)", l, s.0 * 100.0),
            Div(l, s) => write!(f, "({} / {}%)", l, s.0 * 100.0),
            Max(a, b) => write!(f, "max({}, {})", a, b),
            Min(a, b) => write!(f, "min({}, {})", a, b),
            Abs(e) => write!(f, "abs({})", e),
        }
    }
}

/// Computed [`Length`].
pub type LayoutLength = webrender_api::euclid::Length<f32, wr::LayoutPixel>;

/// Convert a [`LayoutLength`] to font units.
#[inline]
pub fn layout_to_pt(length: LayoutLength) -> f32 {
    length.get() * 72.0 / 96.0
}

/// Convert font units to a [`LayoutLength`].
#[inline]
pub fn pt_to_layout(pt: f32) -> LayoutLength {
    LayoutLength::new(pt * 96.0 / 72.0) // TODO verify this formula
}

/// Extension methods for initializing [`Length`] units.
///
/// This trait is implemented for [`f32`] and [`u32`] allowing initialization of length units using the `<number>.<unit>()` syntax.
///
/// # Example
///
/// ```
/// # use zero_ui_core::units::*;
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
#[derive(Clone, PartialEq)]
pub struct Point {
    /// *x* offset in length units.
    pub x: Length,
    /// *y* offset in length units.
    pub y: Length,
}
impl fmt::Debug for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("Point").field("x", &self.x).field("y", &self.y).finish()
        } else {
            write!(f, "({:?}, {:?})", self.x, self.y)
        }
    }
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
    /// New x, y from any [`Length`] unit.
    pub fn new<X: Into<Length>, Y: Into<Length>>(x: X, y: Y) -> Self {
        Point { x: x.into(), y: y.into() }
    }

    /// ***x:*** [`Length::zero`], ***y:*** [`Length::zero`].
    #[inline]
    pub fn zero() -> Self {
        Self::new(Length::zero(), Length::zero())
    }

    /// Point at the top-middle of the available space.
    ///
    /// ***x:*** [`Length::half`], ***y:*** [`Length::zero`]
    #[inline]
    pub fn top() -> Self {
        Self::new(Length::half(), Length::zero())
    }

    /// Point at the bottom-middle of the available space.
    ///
    /// ***x:*** [`Length::half`], ***y:*** [`Length::fill`]
    #[inline]
    pub fn bottom() -> Self {
        Self::new(Length::half(), Length::fill())
    }

    /// Point at the middle-left of the available space.
    ///
    /// ***x:*** [`Length::zero`], ***y:*** [`Length::half`]
    #[inline]
    pub fn left() -> Self {
        Self::new(Length::zero(), Length::half())
    }

    /// Point at the middle-right of the available space.
    ///
    /// ***x:*** [`Length::fill`], ***y:*** [`Length::half`]
    #[inline]
    pub fn right() -> Self {
        Self::new(Length::fill(), Length::half())
    }

    /// Point at the top-left of the available space.
    ///
    /// ***x:*** [`Length::zero`], ***y:*** [`Length::zero`]
    #[inline]
    pub fn top_left() -> Self {
        Self::zero()
    }

    /// Point at the top-right of the available space.
    ///
    /// ***x:*** [`Length::fill`], ***y:*** [`Length::zero`]
    #[inline]
    pub fn top_right() -> Self {
        Self::new(Length::fill(), Length::zero())
    }

    /// Point at the bottom-left of the available space.
    ///
    /// ***x:*** [`Length::zero`], ***y:*** [`Length::fill`]
    #[inline]
    pub fn bottom_left() -> Self {
        Self::new(Length::zero(), Length::fill())
    }

    /// Point at the bottom-right of the available space.
    ///
    /// ***x:*** [`Length::fill`], ***y:*** [`Length::fill`]
    #[inline]
    pub fn bottom_right() -> Self {
        Self::new(Length::fill(), Length::fill())
    }

    /// Swap `x` and `y`.
    #[inline]
    pub fn yx(self) -> Self {
        Point { y: self.x, x: self.y }
    }

    /// Returns `(x, y)`.
    #[inline]
    pub fn into_tuple(self) -> (Length, Length) {
        (self.x, self.y)
    }

    /// Compute the point in a layout context.
    #[inline]
    pub fn to_layout(&self, ctx: &LayoutMetrics, available_size: LayoutSize) -> LayoutPoint {
        LayoutPoint::from_lengths(
            self.x.to_layout(ctx, LayoutLength::new(available_size.width)),
            self.y.to_layout(ctx, LayoutLength::new(available_size.height)),
        )
    }
}
impl_length_comp_conversions! {
    fn from(x: X, y: Y) -> Point {
        Point::new(x, y)
    }
}
impl_from_and_into_var! {
    fn from(p: LayoutPoint) -> Point {
        Point::new(p.x, p.y)
    }
}

/// Computed [`Point`].
pub type LayoutPoint = wr::LayoutPoint;

/// 2D size in [`Length`] units.
#[derive(Clone, PartialEq)]
pub struct Size {
    /// *width* in length units.
    pub width: Length,
    /// *height* in length units.
    pub height: Length,
}
impl fmt::Debug for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("Size")
                .field("width", &self.width)
                .field("height", &self.height)
                .finish()
        } else {
            write!(f, "({:?}, {:?})", self.width, self.height)
        }
    }
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
    /// New width, height from any [`Length`] unit.
    pub fn new<W: Into<Length>, H: Into<Length>>(width: W, height: H) -> Self {
        Size {
            width: width.into(),
            height: height.into(),
        }
    }

    /// ***width:*** [`Length::zero`], ***height:*** [`Length::zero`]
    #[inline]
    pub fn zero() -> Self {
        Self::new(Length::zero(), Length::zero())
    }

    /// Size that fills the available space.
    ///
    /// ***width:*** [`Length::fill`], ***height:*** [`Length::fill`]
    #[inline]
    pub fn fill() -> Self {
        Self::new(Length::fill(), Length::fill())
    }

    /// Returns `(width, height)`.
    #[inline]
    pub fn into_tuple(self) -> (Length, Length) {
        (self.width, self.height)
    }

    /// Compute the size in a layout context.
    #[inline]
    pub fn to_layout(&self, ctx: &LayoutMetrics, available_size: LayoutSize) -> LayoutSize {
        LayoutSize::from_lengths(
            self.width.to_layout(ctx, LayoutLength::new(available_size.width)),
            self.height.to_layout(ctx, LayoutLength::new(available_size.height)),
        )
    }
}
impl_length_comp_conversions! {
    fn from(width: W, height: H) -> Size {
        Size::new(width, height)
    }
}
impl_from_and_into_var! {
    fn from(size: LayoutSize) -> Size {
        Size::new(size.width, size.height)
    }
}

/// Computed [`Size`].
pub type LayoutSize = wr::LayoutSize;

/// Ellipse in [`Length`] units.
///
/// This is very similar to [`Size`] but allows initializing from a single [`Length`].
#[derive(Clone, PartialEq)]
pub struct Ellipse {
    /// *width* in length units.
    pub width: Length,
    /// *height* in length units.
    pub height: Length,
}
impl fmt::Debug for Ellipse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("Ellipse")
                .field("width", &self.width)
                .field("height", &self.height)
                .finish()
        } else if self.maybe_circle() {
            write!(f, "{:?}", self.width)
        } else {
            write!(f, "({:?}, {:?})", self.width, self.height)
        }
    }
}
impl fmt::Display for Ellipse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.maybe_circle() {
            if let Some(p) = f.precision() {
                write!(f, "{:.p$}", self.width, p = p)
            } else {
                write!(f, "{}", self.width)
            }
        } else if let Some(p) = f.precision() {
            write!(f, "{:.p$} × {:.p$}", self.width, self.height, p = p)
        } else {
            write!(f, "{} × {}", self.width, self.height)
        }
    }
}
impl Ellipse {
    /// New width, height from any [`Length`] unit.
    pub fn new<W: Into<Length>, H: Into<Length>>(width: W, height: H) -> Self {
        Ellipse {
            width: width.into(),
            height: height.into(),
        }
    }

    /// New width and height from the same [`Length`].
    pub fn new_all<L: Into<Length>>(width_and_height: L) -> Self {
        let l = width_and_height.into();
        Ellipse {
            width: l.clone(),
            height: l,
        }
    }

    /// ***width:*** [`Length::zero`], ***height:*** [`Length::zero`]
    #[inline]
    pub fn zero() -> Self {
        Self::new_all(Length::zero())
    }

    /// Size that fills the available space.
    ///
    /// ***width:*** [`Length::fill`], ***height:*** [`Length::fill`]
    #[inline]
    pub fn fill() -> Self {
        Self::new_all(Length::fill())
    }

    /// Returns `(width, height)`.
    #[inline]
    pub fn into_tuple(self) -> (Length, Length) {
        (self.width, self.height)
    }

    /// Compute the size in a layout context.
    #[inline]
    pub fn to_layout(&self, ctx: &LayoutMetrics, available_size: LayoutSize) -> LayoutEllipse {
        LayoutEllipse::from_lengths(
            self.width.to_layout(ctx, LayoutLength::new(available_size.width)),
            self.height.to_layout(ctx, LayoutLength::new(available_size.height)),
        )
    }

    /// If the [`width`](Self::width) and [`height`](Self::height) are equal.
    ///
    /// Note that if the values are relative may still not be a perfect circle.
    #[inline]
    pub fn maybe_circle(&self) -> bool {
        self.width == self.height
    }
}
impl_from_and_into_var! {
    /// New circular.
    fn from(all: Length) -> Ellipse {
        Ellipse::new_all(all)
    }

    /// New circular relative length.
    fn from(percent: FactorPercent) -> Ellipse {
        Ellipse::new_all(percent)
    }
    /// New circular relative length.
    fn from(norm: FactorNormal) -> Ellipse {
        Ellipse::new_all(norm)
    }

    /// New circular exact length.
    fn from(f: f32) -> Ellipse {
        Ellipse::new_all(f)
    }
    /// New circular exact length.
    fn from(i: i32) -> Ellipse {
        Ellipse::new_all(i)
    }

    /// New from [`LayoutEllipse`].
    fn from(ellipse: LayoutEllipse) -> Ellipse {
        Ellipse::new(ellipse.width, ellipse.height)
    }
}

/// Computed [`Ellipse`].
pub type LayoutEllipse = LayoutSize;

/// Spacing in-between grid cells in [`Length`] units.
#[derive(Clone, PartialEq)]
pub struct GridSpacing {
    /// Spacing in-between columns, in length units.
    pub column: Length,
    /// Spacing in-between rows, in length units.
    pub row: Length,
}
impl GridSpacing {
    /// New column, row from any [`Length`] unit..
    pub fn new<C: Into<Length>, R: Into<Length>>(column: C, row: R) -> Self {
        GridSpacing {
            column: column.into(),
            row: row.into(),
        }
    }

    /// Same spacing for both columns and rows.
    pub fn new_all<S: Into<Length>>(same: S) -> Self {
        let same = same.into();
        GridSpacing {
            column: same.clone(),
            row: same,
        }
    }

    /// Compute the spacing in a layout context.
    #[inline]
    pub fn to_layout(&self, ctx: &LayoutMetrics, available_size: LayoutSize) -> LayoutGridSpacing {
        LayoutGridSpacing {
            column: self.column.to_layout(ctx, LayoutLength::new(available_size.width)).get(),
            row: self.row.to_layout(ctx, LayoutLength::new(available_size.height)).get(),
        }
    }
}
impl_length_comp_conversions! {
    fn from(column: C, row: R) -> GridSpacing {
        GridSpacing::new(column, row)
    }
}
impl_from_and_into_var! {
    /// Same spacing for both columns and rows.
    fn from(all: Length) -> GridSpacing {
        GridSpacing::new_all(all)
    }

    /// Column and row equal relative length.
    fn from(percent: FactorPercent) -> GridSpacing {
        GridSpacing::new_all(percent)
    }
    /// Column and row equal relative length.
    fn from(norm: FactorNormal) -> GridSpacing {
        GridSpacing::new_all(norm)
    }

    /// Column and row equal exact length.
    fn from(f: f32) -> GridSpacing {
        GridSpacing::new_all(f)
    }
    /// Column and row equal exact length.
    fn from(i: i32) -> GridSpacing {
        GridSpacing::new_all(i)
    }

    /// Column and row in exact length.
    fn from(spacing: LayoutGridSpacing) -> GridSpacing {
        GridSpacing::new(spacing.column, spacing.row)
    }
}
impl fmt::Debug for GridSpacing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("GridSpacing")
                .field("column", &self.column)
                .field("row", &self.row)
                .finish()
        } else if self.column == self.row {
            write!(f, "{:?}", self.column)
        } else {
            write!(f, "({:?}, {:?})", self.column, self.row)
        }
    }
}

/// Computed [`GridSpacing`].
#[derive(Clone, Copy, Debug)]
pub struct LayoutGridSpacing {
    /// Spacing in-between columns, in layout pixels.
    pub column: f32,
    /// Spacing in-between rows, in layout pixels.
    pub row: f32,
}

/// 2D rect in [`Length`] units.
#[derive(Clone, PartialEq)]
pub struct Rect {
    /// Top-left origin of the rectangle in length units.
    pub origin: Point,
    /// Size of the rectangle in length units.
    pub size: Size,
}
impl fmt::Debug for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("Rect")
                .field("origin", &self.origin)
                .field("size", &self.size)
                .finish()
        } else {
            write!(f, "{:?}.at{:?}", self.origin, self.size)
        }
    }
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
    /// New rectangle defined by an origin point (top-left) and a size, both in any type that converts to
    /// [`Point`] and [`Size`].
    ///
    /// Also see [`RectFromTuplesBuilder`] for another way of initializing a rectangle value.
    pub fn new<O: Into<Point>, S: Into<Size>>(origin: O, size: S) -> Self {
        Rect {
            origin: origin.into(),
            size: size.into(),
        }
    }

    /// New rectangle at origin [zero](Point::zero). The size is in any [`Length`] unit.
    pub fn from_size<S: Into<Size>>(size: S) -> Self {
        Self::new(Point::zero(), size)
    }

    /// New rectangle at origin [zero](Point::zero) and size [zero](Size::zero).
    #[inline]
    pub fn zero() -> Self {
        Self::new(Point::zero(), Size::zero())
    }

    /// Rect that fills the available space.
    #[inline]
    pub fn fill() -> Self {
        Self::from_size(Size::fill())
    }

    /// Compute the rectangle in a layout context.
    #[inline]
    pub fn to_layout(&self, ctx: &LayoutMetrics, available_size: LayoutSize) -> LayoutRect {
        LayoutRect::new(self.origin.to_layout(ctx, available_size), self.size.to_layout(ctx, available_size))
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
impl_from_and_into_var! {
    /// New in exact length.
    fn from(rect: LayoutRect) -> Rect {
        Rect::new(rect.origin, rect.size)
    }
}

/// Computed [`Rect`].
pub type LayoutRect = wr::LayoutRect;

/// 2D line in [`Length`] units.
#[derive(Clone, PartialEq)]
pub struct Line {
    /// Start point in length units.
    pub start: Point,
    /// End point in length units.
    pub end: Point,
}
impl fmt::Debug for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("Line").field("start", &self.start).field("end", &self.end).finish()
        } else {
            write!(f, "{:?}.to{:?}", self.start, self.end)
        }
    }
}
impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(p) = f.precision() {
            write!(f, "{:.p$} to {:.p$}", self.start, self.end, p = p)
        } else {
            write!(f, "{} to {}", self.start, self.end)
        }
    }
}
impl Line {
    /// New line defined by two points of any type that converts to [`Point`].
    ///
    /// Also see [`LineFromTuplesBuilder`] for another way of initializing a line value.
    pub fn new<S: Into<Point>, E: Into<Point>>(start: S, end: E) -> Self {
        Line {
            start: start.into(),
            end: end.into(),
        }
    }

    /// Line from [zero](Point::zero) to [zero](Point::zero).
    #[inline]
    pub fn zero() -> Line {
        Line {
            start: Point::zero(),
            end: Point::zero(),
        }
    }

    /// Line that fills the available length from [bottom](Point::bottom) to [top](Point::top).
    #[inline]
    pub fn to_top() -> Line {
        Line {
            start: Point::bottom(),
            end: Point::top(),
        }
    }

    /// Line that traces the length from [top](Point::top) to [bottom](Point::bottom).
    #[inline]
    pub fn to_bottom() -> Line {
        Line {
            start: Point::top(),
            end: Point::bottom(),
        }
    }

    /// Line that traces the length from [left](Point::left) to [right](Point::right).
    #[inline]
    pub fn to_right() -> Line {
        Line {
            start: Point::left(),
            end: Point::right(),
        }
    }

    /// Line that traces the length from [right](Point::right) to [left](Point::left).
    #[inline]
    pub fn to_left() -> Line {
        Line {
            start: Point::right(),
            end: Point::left(),
        }
    }

    /// Line that traces the length from [bottom-right](Point::bottom_right) to [top-left](Point::top_left).
    #[inline]
    pub fn to_top_left() -> Line {
        Line {
            start: Point::bottom_right(),
            end: Point::top_left(),
        }
    }

    /// Line that traces the length from [bottom-left](Point::bottom_left) to [top-right](Point::top_right).
    #[inline]
    pub fn to_top_right() -> Line {
        Line {
            start: Point::bottom_left(),
            end: Point::top_right(),
        }
    }

    /// Line that traces the length from [top-right](Point::top_right) to [bottom-left](Point::bottom_left).
    #[inline]
    pub fn to_bottom_left() -> Line {
        Line {
            start: Point::top_right(),
            end: Point::bottom_left(),
        }
    }

    /// Line that traces the length from [top-left](Point::top_left) to [bottom-right](Point::bottom_right).
    #[inline]
    pub fn to_bottom_right() -> Line {
        Line {
            start: Point::top_left(),
            end: Point::bottom_right(),
        }
    }

    /// Compute the line in a layout context.
    #[inline]
    pub fn to_layout(&self, ctx: &LayoutMetrics, available_size: LayoutSize) -> LayoutLine {
        LayoutLine {
            start: self.start.to_layout(ctx, available_size),
            end: self.end.to_layout(ctx, available_size),
        }
    }
}
impl_from_and_into_var! {
    /// From exact lengths.
    fn from(line: LayoutLine) -> Line {
        Line::new(line.start, line.end)
    }
}

/// Computed [`Line`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutLine {
    /// Start point in layout units.
    pub start: LayoutPoint,
    /// End point in layout units.
    pub end: LayoutPoint,
}
impl LayoutLine {
    /// New layout line defined by two layout points.
    #[inline]
    pub fn new(start: LayoutPoint, end: LayoutPoint) -> Self {
        LayoutLine { start, end }
    }

    /// Line from (0, 0) to (0, 0).
    #[inline]
    pub fn zero() -> LayoutLine {
        LayoutLine::new(LayoutPoint::zero(), LayoutPoint::zero())
    }

    /// Line length in layout units.
    #[inline]
    pub fn length(&self) -> LayoutLength {
        LayoutLength::new(self.start.distance_to(self.end))
    }

    /// Bounding box that fits the line points, in layout units.
    #[inline]
    pub fn bounds(&self) -> LayoutRect {
        LayoutRect::from_points(&[self.start, self.end])
    }
}

/// Build a [`Line`] using the syntax `(x1, y1).to(x2, y2)`.
///
/// # Example
///
/// ```
/// # use zero_ui_core::units::*;
/// let line = (10, 20).to(100, 120);
/// assert_eq!(Line::new(Point::new(10, 20), Point::new(100, 120)), line);
/// ```
pub trait LineFromTuplesBuilder {
    /// New [`Line`] from `self` as a start point to `x2, y2` end point.
    fn to<X2: Into<Length>, Y2: Into<Length>>(self, x2: X2, y2: Y2) -> Line;
}
impl<X1: Into<Length>, Y1: Into<Length>> LineFromTuplesBuilder for (X1, Y1) {
    fn to<X2: Into<Length>, Y2: Into<Length>>(self, x2: X2, y2: Y2) -> Line {
        Line::new(self, (x2, y2))
    }
}

/// Build a [`Rect`] using the syntax `(width, height).at(x, y)`.
///
/// # Example
///
/// ```
/// # use zero_ui_core::units::*;
/// let rect = (800, 600).at(10, 20);
/// assert_eq!(Rect::new(Point::new(10, 20), Size::new(800, 600)), rect);
/// ```
pub trait RectFromTuplesBuilder {
    /// New [`Rect`] from `self` as the size placed at the `x, y` origin.
    fn at<X: Into<Length>, Y: Into<Length>>(self, x: X, y: Y) -> Rect;
}
impl<W: Into<Length>, H: Into<Length>> RectFromTuplesBuilder for (W, H) {
    fn at<X: Into<Length>, Y: Into<Length>>(self, x: X, y: Y) -> Rect {
        Rect::new((x, y), self)
    }
}

/// 2D size offsets in [`Length`] units.
///
/// This unit defines spacing around all four sides of a box, a widget margin can be defined using a value of this type.
#[derive(Clone, PartialEq)]
pub struct SideOffsets {
    /// Spacing above, in length units.
    pub top: Length,
    /// Spacing to the right, in length units.
    pub right: Length,
    /// Spacing bellow, in length units.
    pub bottom: Length,
    /// Spacing to the left ,in length units.
    pub left: Length,
}
impl fmt::Debug for SideOffsets {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("SideOffsets")
                .field("top", &self.top)
                .field("right", &self.right)
                .field("bottom", &self.bottom)
                .field("left", &self.left)
                .finish()
        } else if self.all_eq() {
            write!(f, "{:?}", self.top)
        } else if self.dimensions_eq() {
            write!(f, "({:?}, {:?})", self.top, self.left)
        } else {
            write!(f, "({:?}, {:?}, {:?}, {:?})", self.top, self.right, self.bottom, self.left)
        }
    }
}
impl SideOffsets {
    /// New top, right, bottom left offsets. From any [`Length`] type.
    pub fn new<T: Into<Length>, R: Into<Length>, B: Into<Length>, L: Into<Length>>(top: T, right: R, bottom: B, left: L) -> Self {
        SideOffsets {
            top: top.into(),
            right: right.into(),
            bottom: bottom.into(),
            left: left.into(),
        }
    }

    /// Top-bottom and left-right equal. From any [`Length`] type.
    pub fn new_dimension<TB: Into<Length>, LR: Into<Length>>(top_bottom: TB, left_right: LR) -> Self {
        let top_bottom = top_bottom.into();
        let left_right = left_right.into();
        SideOffsets {
            top: top_bottom.clone(),
            bottom: top_bottom,
            left: left_right.clone(),
            right: left_right,
        }
    }

    /// All sides equal. From any [`Length`] type.
    pub fn new_all<T: Into<Length>>(all_sides: T) -> Self {
        let all_sides = all_sides.into();
        SideOffsets {
            top: all_sides.clone(),
            right: all_sides.clone(),
            bottom: all_sides.clone(),
            left: all_sides,
        }
    }

    /// All sides [zero](Length::zero).
    #[inline]
    pub fn zero() -> Self {
        Self::new_all(Length::zero())
    }

    /// If all sides are equal.
    #[inline]
    pub fn all_eq(&self) -> bool {
        self.top == self.bottom && self.top == self.left && self.top == self.right
    }

    /// If top and bottom are equal; and left and right are equal.
    #[inline]
    pub fn dimensions_eq(&self) -> bool {
        self.top == self.bottom && self.left == self.right
    }

    /// Compute the offsets in a layout context.
    #[inline]
    pub fn to_layout(&self, ctx: &LayoutMetrics, available_size: LayoutSize) -> LayoutSideOffsets {
        let width = LayoutLength::new(available_size.width);
        let height = LayoutLength::new(available_size.height);
        LayoutSideOffsets::from_lengths(
            self.top.to_layout(ctx, height),
            self.right.to_layout(ctx, width),
            self.bottom.to_layout(ctx, height),
            self.left.to_layout(ctx, width),
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

    /// From exact lengths.
    fn from(offsets: LayoutSideOffsets) -> SideOffsets {
        SideOffsets::new(offsets.top, offsets.right, offsets.bottom, offsets.left)
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
/// The values indicate how much to the right and bottom the content is moved within
/// a larger available space. An `x` value of `0.0` means the content left border touches
/// the container left border, a value of `1.0` means the content right border touches the
/// container right border.
///
/// There is a constant for each of the usual alignment values, the alignment is defined as two factors like this
/// primarily for animating transition between alignments.
///
/// Values outside of the `[0.0..=1.0]` range places the content outside of the container bounds. A **non-finite
/// value** means the content stretches to fill the container bounds.
#[derive(Clone, Copy)]
pub struct Alignment {
    /// *x* alignment in a `[0.0..=1.0]` range.
    pub x: FactorNormal,
    /// *y* alignment in a `[0.0..=1.0]` range.
    pub y: FactorNormal,
}
impl PartialEq for Alignment {
    fn eq(&self, other: &Self) -> bool {
        self.fill_width() == other.fill_width() && self.fill_height() == other.fill_height() && self.x == other.x && self.y == other.y
    }
}
impl Alignment {
    /// Returns `true` if [`x`] is a special value that indicates the content width must be the container width.
    ///
    /// [`x`]: Alignment::x
    pub fn fill_width(self) -> bool {
        !self.x.0.is_finite()
    }

    /// Returns `true` if [`y`] is a special value that indicates the content height must be the container height.
    ///
    /// [`y`]: Alignment::y
    pub fn fill_height(self) -> bool {
        !self.y.0.is_finite()
    }
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

    ( $([$doc:expr] $NAME:ident = ($x:expr, $y:expr);)+ ) => {
        $(
        #[doc=$doc]
        pub const $NAME: Alignment = Alignment { x: FactorNormal($x), y: FactorNormal($y) };
        )+

        /// Returns the alignment `const` name if `self` is equal to one of then.
        pub fn name(self) -> Option<&'static str> {
            $(
                if self == Self::$NAME {
                    Some(stringify!($NAME))
                }
            )else+
            else {
                None
            }
        }
    };
}
impl fmt::Debug for Alignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = self.name() {
            if f.alternate() {
                write!(f, "Alignment::{}", name)
            } else {
                f.write_str(name)
            }
        } else {
            f.debug_struct("Alignment").field("x", &self.x).field("y", &self.y).finish()
        }
    }
}
impl fmt::Display for Alignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = self.name() {
            f.write_str(name)
        } else {
            f.write_char('(')?;
            if self.fill_width() {
                f.write_str("<fill>")?;
            } else {
                write!(f, "{}", FactorPercent::from(self.x))?;
            }
            f.write_str(", ")?;
            if self.fill_height() {
                f.write_str("<fill>")?;
            } else {
                write!(f, "{}", FactorPercent::from(self.x))?;
            }
            f.write_char(')')
        }
    }
}
impl Alignment {
    named_aligns! {
        LEFT_TOP = (0.0, 0.0);
        LEFT_BOTTOM = (0.0, 1.0);
        LEFT_FILL = (0.0, f32::NAN);

        RIGHT_TOP = (1.0, 0.0);
        RIGHT_BOTTOM = (1.0, 1.0);
        RIGHT_FILL = (1.0, f32::NAN);

        FILL_TOP = (f32::NAN, 0.0);
        FILL_BOTTOM = (f32::NAN, 1.0);

        LEFT = (0.0, 0.5);
        RIGHT = (1.0, 0.5);
        TOP = (0.5, 0.0);
        BOTTOM = (0.5, 1.0);

        CENTER = (0.5, 0.5);
        FILL = (f32::NAN, f32::NAN);
    }
}
impl_from_and_into_var! {
     /// To relative length x and y.
    fn from(alignment: Alignment) -> Point {
        Point {
            x: alignment.x.into(),
            y: alignment.y.into(),
        }
    }
}
impl Alignment {
    /// Compute a content rectangle given this alignment, the content size and the available size.
    ///
    /// To implement alignment, the `content_size` should be measured and recorded in [`UiNode::measure`]
    /// and then this method called in the [`UiNode::arrange`] with the final container size to get the
    /// content rectangle that must be recorded and used in [`UiNode::render`] to size and position the content
    /// in the space of the container.
    ///
    /// [`UiNode::measure`]: crate::UiNode::measure
    /// [`UiNode::arrange`]: crate::UiNode::arrange
    /// [`UiNode::render`]: crate::UiNode::render
    pub fn solve(self, content_size: LayoutSize, container_size: LayoutSize) -> LayoutRect {
        let mut r = LayoutRect::zero();

        if self.fill_width() {
            r.size.width = container_size.width;
        } else {
            r.size.width = container_size.width.min(content_size.width);
            r.origin.x = (container_size.width - r.size.width) * self.x.0;
        }
        if self.fill_height() {
            r.size.height = container_size.height;
        } else {
            r.size.height = container_size.height.min(content_size.height);
            r.origin.y = (container_size.height - r.size.height) * self.y.0;
        }

        r
    }
}

/// Text line height.
#[derive(Clone, PartialEq)]
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
impl fmt::Debug for LineHeight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "LineHeight::")?;
        }
        match self {
            LineHeight::Font => write!(f, "Font"),
            LineHeight::Length(l) => f.debug_tuple("Length").field(l).finish(),
        }
    }
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
#[derive(Clone, PartialEq)]
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
impl fmt::Debug for LetterSpacing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "LetterSpacing::")?;
        }
        match self {
            LetterSpacing::Auto => write!(f, "Auto"),
            LetterSpacing::Length(l) => f.debug_tuple("Length").field(l).finish(),
        }
    }
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
/// A "word" is the sequence of characters in-between space characters. This extra
/// spacing is applied per space character not per word, if there are three spaces between words
/// the extra spacing is applied thrice. Usually the number of spaces between words is collapsed to one,
/// see [`WhiteSpace`](crate::text::WhiteSpace).
#[derive(Clone, PartialEq)]
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
impl fmt::Debug for WordSpacing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "WordSpacing")?;
        }
        match self {
            WordSpacing::Auto => write!(f, "Auto"),
            WordSpacing::Length(l) => f.debug_tuple("Length").field(l).finish(),
        }
    }
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

/// Extra spacing in-between paragraphs.
///
/// The initial paragraph space is `line_height + line_spacing * 2`, this extra spacing is added to that.
///
/// A "paragraph" is a sequence of lines in-between blank lines (empty or spaces only). This extra space is applied per blank line
/// not per paragraph, if there are three blank lines between paragraphs the extra spacing is applied trice.
pub type ParagraphSpacing = Length;

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
    /// The scale factor that defines the pixel grid.
    pub scale_factor: f32,
}
impl PixelGrid {
    /// New from a scale factor. Layout pixels are adjusted so that when the renderer calculates device pixels
    /// no *half pixels* are generated.
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
        let aligned = self.snap(layout_value);
        (aligned - layout_value).abs() < 0.0001
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
        (self.scale_factor - other.scale_factor).abs() < EPSILON_100
    }
}

/// Methods for types that can be aligned to a [`PixelGrid`].
pub trait PixelGridExt {
    /// Gets a copy of self that is aligned with the pixel grid.
    fn snap_to(self, grid: PixelGrid) -> Self;
    /// Checks if self is aligned with the pixel grid.
    #[allow(clippy::wrong_self_convention)] // trait implemented for layout types that are all copy.
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
impl PixelGridExt for LayoutLine {
    fn snap_to(self, grid: PixelGrid) -> Self {
        LayoutLine::new(self.start.snap_to(grid), self.end.snap_to(grid))
    }

    fn is_aligned_to(self, grid: PixelGrid) -> bool {
        self.start.is_aligned_to(grid) && self.end.is_aligned_to(grid)
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
/// # use zero_ui_core::units::*;
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
    pub fn to_layout(&self, ctx: &LayoutMetrics, available_size: LayoutSize) -> LayoutTransform {
        let mut r = LayoutTransform::identity();
        for step in &self.steps {
            r = match step {
                TransformStep::Computed(m) => r.post_transform(m),
                TransformStep::Translate(x, y) => r.post_translate(webrender_api::euclid::vec3(
                    x.to_layout(ctx, LayoutLength::new(available_size.width)).get(),
                    y.to_layout(ctx, LayoutLength::new(available_size.height)).get(),
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

/// Extension methods for initializing [`ByteLength`] values.
pub trait ByteUnits {
    /// Bytes.
    ///
    /// See [`ByteLength`] for more details.
    fn bytes(self) -> ByteLength;
    /// Kibi-bytes.
    ///
    /// See [`ByteLength::from_kibi`] for more details.
    fn kibi_bytes(self) -> ByteLength;
    /// Kilo-bytes.
    ///
    /// See [`ByteLength::from_kilo`] for more details.
    fn kilo_bytes(self) -> ByteLength;

    /// Mebi-bytes.
    ///
    /// See [`ByteLength::from_mebi`] for more details.
    fn mebi_bytes(self) -> ByteLength;
    /// Mega-bytes.
    ///
    /// See [`ByteLength::from_mega`] for more details.
    fn mega_bytes(self) -> ByteLength;

    /// Gibi-bytes.
    ///
    /// See [`ByteLength::from_gibi`] for more details.
    fn gibi_bytes(self) -> ByteLength;
    /// Giga-bytes.
    ///
    /// See [`ByteLength::from_giga`] for more details.
    fn giga_bytes(self) -> ByteLength;

    /// Tebi-bytes.
    ///
    /// See [`ByteLength::from_tebi`] for more details.
    fn tebi_bytes(self) -> ByteLength;
    /// Tera-bytes.
    ///
    /// See [`ByteLength::from_tera`] for more details.
    fn tera_bytes(self) -> ByteLength;
}
impl ByteUnits for usize {
    #[inline]
    fn bytes(self) -> ByteLength {
        ByteLength(self)
    }

    #[inline]
    fn kibi_bytes(self) -> ByteLength {
        ByteLength::from_kibi(self)
    }

    #[inline]
    fn kilo_bytes(self) -> ByteLength {
        ByteLength::from_kilo(self)
    }

    #[inline]
    fn mebi_bytes(self) -> ByteLength {
        ByteLength::from_mebi(self)
    }

    #[inline]
    fn mega_bytes(self) -> ByteLength {
        ByteLength::from_mega(self)
    }

    #[inline]
    fn gibi_bytes(self) -> ByteLength {
        ByteLength::from_gibi(self)
    }

    #[inline]
    fn giga_bytes(self) -> ByteLength {
        ByteLength::from_giga(self)
    }

    #[inline]
    fn tebi_bytes(self) -> ByteLength {
        ByteLength::from_tebi(self)
    }

    #[inline]
    fn tera_bytes(self) -> ByteLength {
        ByteLength::from_tera(self)
    }
}

/// A length in bytes.
///
/// The value is stored in bytes, you can use associated functions to convert from other units or
/// you can use the [`ByteUnits`] extension methods to initialize from an integer literal.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Default,
    dm::Add,
    dm::AddAssign,
    dm::Sub,
    dm::SubAssign,
    dm::Mul,
    dm::MulAssign,
    dm::Div,
    dm::DivAssign,
)]
pub struct ByteLength(pub usize);
impl_from_and_into_var! {
    fn from(bytes: usize) -> ByteLength {
        ByteLength(bytes)
    }
}
impl ByteLength {
    /// Length in bytes.
    ///
    /// This is the same as `.0`.
    pub fn bytes(&self) -> usize {
        self.0
    }

    fn scaled(self, scale: f64) -> f64 {
        self.0 as f64 / scale
    }

    /// Length in kibi-bytes.
    pub fn kibis(self) -> f64 {
        self.scaled(1024.0)
    }

    /// Length in kilo-bytes.
    pub fn kilos(self) -> f64 {
        self.scaled(1000.0)
    }

    /// Length in mebi-bytes.
    pub fn mebis(self) -> f64 {
        self.scaled(1024.0f64.powi(2))
    }

    /// Length in mega-bytes.
    pub fn megas(self) -> f64 {
        self.scaled(1000.0f64.powi(2))
    }

    /// Length in gibi-bytes.
    pub fn gibis(self) -> f64 {
        self.scaled(1024.0f64.powi(3))
    }

    /// Length in giga-bytes.
    pub fn gigas(self) -> f64 {
        self.scaled(1000.0f64.powi(3))
    }

    /// Length in tebi-bytes.
    pub fn tebis(self) -> f64 {
        self.scaled(1024.0f64.powi(4))
    }

    /// Length in tera-bytes.
    pub fn teras(self) -> f64 {
        self.scaled(1000.0f64.powi(4))
    }
}

/// Constructors
impl ByteLength {
    /// From bytes.
    ///
    /// This is the same as `ByteLength(bytes)`.
    #[inline]
    pub fn from_byte(bytes: usize) -> Self {
        ByteLength(bytes)
    }

    fn new(value: usize, scale: usize) -> Self {
        ByteLength(value.saturating_mul(scale))
    }

    /// From kibi-bytes.
    ///
    /// 1 kibi-byte equals 1024 bytes.
    #[inline]
    pub fn from_kibi(kibi_bytes: usize) -> Self {
        Self::new(kibi_bytes, 1024)
    }

    /// From kilo-bytes.
    ///
    /// 1 kilo-byte equals 1000 bytes.
    #[inline]
    pub fn from_kilo(kibi_bytes: usize) -> Self {
        Self::new(kibi_bytes, 1000)
    }

    /// From mebi-bytes.
    ///
    /// 1 mebi-byte equals 1024² bytes.
    pub fn from_mebi(mebi_bytes: usize) -> Self {
        Self::new(mebi_bytes, 1024usize.pow(2))
    }

    /// From mega-bytes.
    ///
    /// 1 mega-byte equals 1000² bytes.
    pub fn from_mega(mebi_bytes: usize) -> Self {
        Self::new(mebi_bytes, 1000usize.pow(2))
    }

    /// From gibi-bytes.
    ///
    /// 1 gibi-byte equals 1024³ bytes.
    pub fn from_gibi(gibi_bytes: usize) -> Self {
        Self::new(gibi_bytes, 1024usize.pow(3))
    }

    /// From giga-bytes.
    ///
    /// 1 giga-byte equals 1000³ bytes.
    pub fn from_giga(giba_bytes: usize) -> Self {
        Self::new(giba_bytes, 1000usize.pow(3))
    }

    /// From tebi-bytes.
    ///
    /// 1 tebi-byte equals 1024^4 bytes.
    pub fn from_tebi(gibi_bytes: usize) -> Self {
        Self::new(gibi_bytes, 1024usize.pow(4))
    }

    /// From tera-bytes.
    ///
    /// 1 tera-byte equals 1000^4 bytes.
    pub fn from_tera(giba_bytes: usize) -> Self {
        Self::new(giba_bytes, 1000usize.pow(4))
    }
}

impl ByteLength {
    /// Compares and returns the maximum of two lengths.
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }

    /// Compares and returns the minimum of two lengths.
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }
}

impl fmt::Debug for ByteLength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_tuple("ByteLength").field(&self.0).finish()
        } else {
            write!(f, "ByteLength({})", self)
        }
    }
}
impl fmt::Display for ByteLength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            if self.0 >= 1024usize.pow(4) {
                write!(f, "{} tebi-bytes", self.tebis())
            } else if self.0 >= 1024usize.pow(3) {
                write!(f, "{} gibi-bytes", self.gibis())
            } else if self.0 >= 1024usize.pow(2) {
                write!(f, "{} mebi-bytes", self.mebis())
            } else if self.0 >= 1024 {
                write!(f, "{} kibi-bytes", self.kibis())
            } else {
                write!(f, "{} bytes", self.bytes())
            }
        } else if self.0 >= 1000usize.pow(4) {
            write!(f, "{} tera-bytes", self.teras())
        } else if self.0 >= 1000usize.pow(3) {
            write!(f, "{} giga-bytes", self.gigas())
        } else if self.0 >= 1000usize.pow(2) {
            write!(f, "{} mega-bytes", self.megas())
        } else if self.0 >= 1000 {
            write!(f, "{} kilo-bytes", self.kilos())
        } else {
            write!(f, "{} bytes", self.bytes())
        }
    }
}

/// Pixels-per-inch resolution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ppi(pub f32);
impl Ppi {
    /// Returns the minimum of the two resolutions.
    pub fn min(self, other: impl Into<Ppi>) -> Ppi {
        Ppi(self.0.min(other.into().0))
    }

    /// Returns the maximum of the two resolutions.
    pub fn max(self, other: impl Into<Ppi>) -> Ppi {
        Ppi(self.0.max(other.into().0))
    }
}
impl Default for Ppi {
    /// 96ppi.
    fn default() -> Self {
        Ppi(96.0)
    }
}

/// Pixels-per-meter resolution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ppm(pub f32);
impl Ppm {
    /// Returns the minimum of the two resolutions.
    pub fn min(self, other: impl Into<Ppm>) -> Ppm {
        Ppm(self.0.min(other.into().0))
    }

    /// Returns the maximum of the two resolutions.
    pub fn max(self, other: impl Into<Ppm>) -> Ppm {
        Ppm(self.0.max(other.into().0))
    }
}
impl Default for Ppm {
    /// 96ppi.
    fn default() -> Self {
        Ppi(96.0).into()
    }
}

/// Extension methods for initializing resolution units.
///
/// # Examples
///
/// ```
/// use zero_ui_core::units::*;
///
/// let ppm: Ppm = 96.dpi().into();
/// ```
pub trait ResolutionUnits {
    /// Pixels-per-inch.
    fn ppi(self) -> Ppi;
    /// Same as [`ppi`].
    ///
    /// [`ppi`]: ResolutionUnits::ppi.
    fn dpi(self) -> Ppi
    where
        Self: Sized,
    {
        self.ppi()
    }

    /// Pixels-per-meter.
    fn ppm(self) -> Ppm;
}
impl ResolutionUnits for u32 {
    fn ppi(self) -> Ppi {
        Ppi(self as f32)
    }

    fn ppm(self) -> Ppm {
        Ppm(self as f32)
    }
}
impl ResolutionUnits for f32 {
    fn ppi(self) -> Ppi {
        Ppi(self)
    }

    fn ppm(self) -> Ppm {
        Ppm(self)
    }
}

impl fmt::Display for Ppi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}ppi", self.0)
    }
}
impl fmt::Display for Ppm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}ppm", self.0)
    }
}
impl_from_and_into_var! {
    fn from(ppi: Ppi) -> Ppm {
        Ppm(ppi.0 * 39.3701)
    }

    fn from(ppm: Ppm) -> Ppi {
        Ppi(ppm.0 / 39.3701)
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

    #[test]
    pub fn length_expr_same_unit() {
        let a = Length::from(200);
        let b = Length::from(300);
        let c = a + b;

        assert_eq!(c, Length::Exact(500.0));
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
        let l = (Length::from(200) - 100.pct()).abs();
        let ctx = LayoutMetrics::new(LayoutSize::new(600.0, 400.0), 14.0);
        let l = l.to_layout(&ctx, LayoutLength::new(600.0));

        assert!(about_eq(l.0, (200.0f32 - 600.0).abs(), 0.0001));
    }

    #[test]
    pub fn length_expr_clamp() {
        let l = Length::from(100.pct()).clamp(100, 500);
        assert!(matches!(l, Length::Expr(_)));

        let metrics = LayoutMetrics::new(LayoutSize::zero(), 0.0);

        let r = l.to_layout(&metrics, LayoutLength::new(200.0));
        assert!(about_eq(r.0, 200.0, 0.0001));

        let r = l.to_layout(&metrics, LayoutLength::new(50.0));
        assert!(about_eq(r.0, 100.0, 0.0001));

        let r = l.to_layout(&metrics, LayoutLength::new(550.0));
        assert!(about_eq(r.0, 500.0, 0.0001));
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
