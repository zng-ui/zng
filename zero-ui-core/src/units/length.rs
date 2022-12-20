use super::{about_eq, Dip, DipToPx, Factor, FactorPercent, FactorUnits, Px, EPSILON, EPSILON_100};
use std::{fmt, mem, ops};

use crate::{context::Layout1dMetrics, impl_from_and_into_var};

/// 1D length units.
///
/// See [`LengthUnits`] for more details.
///
/// # Equality
///
/// Two lengths are equal if they are of the same variant and if:
///
/// * `Dip` and `px` lengths uses [`Dip`] and [`Px`] equality.
/// * `Relative`, `Em`, `RootEm` lengths use the [`Factor`] equality.
/// * Viewport lengths uses [`about_eq`] with `0.00001` epsilon.
#[derive(Clone)]
pub enum Length {
    /// The default (initial) value.
    ///
    /// This is usually `0.px()`, unless the property redefines it.
    Default,
    /// The exact length in device independent units.
    Dip(Dip),
    /// The exact length in device pixel units.
    Px(Px),
    /// The exact length in font points.
    Pt(f32),
    /// Relative to the fill length.
    Relative(Factor),
    /// Relative to the leftover fill length.
    Leftover(Factor),
    /// Relative to the font-size of the widget.
    Em(Factor),
    /// Relative to the font-size of the root widget.
    RootEm(Factor),
    /// Relative to the width of the nearest viewport ancestor.
    ViewportWidth(Factor),
    /// Relative to the height of the nearest viewport ancestor.
    ViewportHeight(Factor),
    /// Relative to the smallest of the nearest viewport ancestor's dimensions.
    ViewportMin(Factor),
    /// Relative to the smallest of the nearest viewport ancestor's dimensions.
    ViewportMax(Factor),

    /// The exact length in device independent units, defined using a `f32` value.
    ///
    /// This value will be rounded to the nearest pixel after layout,
    /// but it will be used as is in the evaluation of length expressions.
    DipF32(f32),
    /// The exact length in device pixel units, defined using a `f32` value.
    ///
    /// This value will be rounded to the nearest pixel after layout,
    /// but it will be used as is in the evaluation of length expressions.
    PxF32(f32),

    /// Expression.
    Expr(Box<LengthExpr>),
}
impl<L: Into<Length>> ops::Add<L> for Length {
    type Output = Length;

    fn add(self, rhs: L) -> Self::Output {
        use Length::*;

        match (self, rhs.into()) {
            (Dip(a), Dip(b)) => Dip(a + b),
            (Px(a), Px(b)) => Px(a + b),
            (Pt(a), Pt(b)) => Pt(a + b),
            (Relative(a), Relative(b)) => Relative(a + b),
            (Leftover(a), Leftover(b)) => Leftover(a + b),
            (Em(a), Em(b)) => Em(a + b),
            (RootEm(a), RootEm(b)) => RootEm(a + b),
            (ViewportWidth(a), ViewportWidth(b)) => ViewportWidth(a + b),
            (ViewportHeight(a), ViewportHeight(b)) => ViewportHeight(a + b),
            (ViewportMin(a), ViewportMin(b)) => ViewportMin(a + b),
            (ViewportMax(a), ViewportMax(b)) => ViewportMax(a + b),
            (PxF32(a), PxF32(b)) => PxF32(a + b),
            (DipF32(a), DipF32(b)) => DipF32(a + b),
            (Px(a), PxF32(b)) | (PxF32(b), Px(a)) => PxF32(a.0 as f32 + b),
            (Dip(a), DipF32(b)) | (DipF32(b), Dip(a)) => DipF32(a.to_f32() + b),
            (a, b) => Length::Expr(Box::new(LengthExpr::Add(a, b))),
        }
    }
}
impl<L: Into<Length>> ops::AddAssign<L> for Length {
    fn add_assign(&mut self, rhs: L) {
        let lhs = mem::take(self);
        *self = lhs + rhs.into();
    }
}
impl<L: Into<Length>> ops::Sub<L> for Length {
    type Output = Length;

    fn sub(self, rhs: L) -> Self::Output {
        use Length::*;

        match (self, rhs.into()) {
            (Dip(a), Dip(b)) => Dip(a - b),
            (Px(a), Px(b)) => Px(a - b),
            (Pt(a), Pt(b)) => Pt(a - b),
            (Relative(a), Relative(b)) => Relative(a - b),
            (Leftover(a), Leftover(b)) => Leftover(a - b),
            (Em(a), Em(b)) => Em(a - b),
            (RootEm(a), RootEm(b)) => RootEm(a - b),
            (ViewportWidth(a), ViewportWidth(b)) => ViewportWidth(a - b),
            (ViewportHeight(a), ViewportHeight(b)) => ViewportHeight(a - b),
            (ViewportMin(a), ViewportMin(b)) => ViewportMin(a - b),
            (ViewportMax(a), ViewportMax(b)) => ViewportMax(a - b),
            (PxF32(a), PxF32(b)) => PxF32(a - b),
            (DipF32(a), DipF32(b)) => DipF32(a - b),
            (Px(a), PxF32(b)) => PxF32(a.0 as f32 - b),
            (PxF32(a), Px(b)) => PxF32(a - b.0 as f32),
            (Dip(a), DipF32(b)) => DipF32(a.to_f32() - b),
            (DipF32(a), Dip(b)) => DipF32(a - b.to_f32()),
            (a, b) => Length::Expr(Box::new(LengthExpr::Sub(a, b))),
        }
    }
}
impl<L: Into<Length>> ops::SubAssign<L> for Length {
    fn sub_assign(&mut self, rhs: L) {
        let lhs = mem::take(self);
        *self = lhs - rhs.into();
    }
}
impl<F: Into<Factor>> ops::Mul<F> for Length {
    type Output = Length;

    fn mul(self, rhs: F) -> Self::Output {
        use Length::*;
        let rhs = rhs.into();
        match self {
            Dip(e) => DipF32(e.to_f32() * rhs.0),
            Px(e) => PxF32(e.0 as f32 * rhs.0),
            Pt(e) => Pt(e * rhs.0),
            Relative(r) => Relative(r * rhs),
            Leftover(r) => Leftover(r * rhs),
            Em(e) => Em(e * rhs),
            RootEm(e) => RootEm(e * rhs),
            ViewportWidth(w) => ViewportWidth(w * rhs),
            ViewportHeight(h) => ViewportHeight(h * rhs),
            ViewportMin(m) => ViewportMin(m * rhs),
            ViewportMax(m) => ViewportMax(m * rhs),
            DipF32(e) => DipF32(e * rhs.0),
            PxF32(e) => PxF32(e * rhs.0),
            e => Expr(Box::new(LengthExpr::Mul(e, rhs))),
        }
    }
}
impl<F: Into<Factor>> ops::MulAssign<F> for Length {
    fn mul_assign(&mut self, rhs: F) {
        let lhs = mem::take(self);
        *self = lhs * rhs.into();
    }
}
impl<F: Into<Factor>> ops::Div<F> for Length {
    type Output = Length;

    fn div(self, rhs: F) -> Self::Output {
        use Length::*;

        let rhs = rhs.into();

        match self {
            Dip(e) => DipF32(e.to_f32() / rhs.0),
            Px(e) => PxF32(e.0 as f32 / rhs.0),
            Pt(e) => Pt(e / rhs.0),
            Relative(r) => Relative(r / rhs),
            Leftover(r) => Leftover(r / rhs),
            Em(e) => Em(e / rhs),
            RootEm(e) => RootEm(e / rhs),
            ViewportWidth(w) => ViewportWidth(w / rhs),
            ViewportHeight(h) => ViewportHeight(h / rhs),
            ViewportMin(m) => ViewportMin(m / rhs),
            ViewportMax(m) => ViewportMax(m / rhs),
            DipF32(e) => DipF32(e / rhs.0),
            PxF32(e) => PxF32(e / rhs.0),
            e => Expr(Box::new(LengthExpr::Mul(e, rhs))),
        }
    }
}
impl<F: Into<Factor>> ops::DivAssign<F> for Length {
    fn div_assign(&mut self, rhs: F) {
        let lhs = mem::take(self);
        *self = lhs / rhs.into();
    }
}
impl ops::Neg for Length {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            Length::Default => Length::Expr(Box::new(LengthExpr::Neg(Length::Default))),
            Length::Dip(e) => Length::Dip(-e),
            Length::Px(e) => Length::Px(-e),
            Length::Pt(e) => Length::Pt(-e),
            Length::Relative(e) => Length::Relative(-e),
            Length::Leftover(e) => Length::Leftover(-e),
            Length::Em(e) => Length::Em(-e),
            Length::RootEm(e) => Length::RootEm(-e),
            Length::ViewportWidth(e) => Length::ViewportWidth(-e),
            Length::ViewportHeight(e) => Length::ViewportHeight(-e),
            Length::ViewportMin(e) => Length::ViewportMin(-e),
            Length::ViewportMax(e) => Length::ViewportMax(-e),
            Length::DipF32(e) => Length::DipF32(-e),
            Length::PxF32(e) => Length::PxF32(-e),
            Length::Expr(e) => Length::Expr(Box::new(LengthExpr::Neg(Length::Expr(e)))),
        }
    }
}
impl Default for Length {
    /// `Length::Default`
    fn default() -> Self {
        Length::Default
    }
}
impl PartialEq for Length {
    fn eq(&self, other: &Self) -> bool {
        use Length::*;
        match (self, other) {
            (Default, Default) => true,

            (Dip(a), Dip(b)) => a == b,
            (Px(a), Px(b)) => a == b,
            (Pt(a), Pt(b)) => about_eq(*a, *b, EPSILON_100),

            (DipF32(a), DipF32(b)) | (PxF32(a), PxF32(b)) => about_eq(*a, *b, EPSILON_100),

            (Relative(a), Relative(b)) | (Em(a), Em(b)) | (RootEm(a), RootEm(b)) | (Leftover(a), Leftover(b)) => a == b,

            (ViewportWidth(a), ViewportWidth(b))
            | (ViewportHeight(a), ViewportHeight(b))
            | (ViewportMin(a), ViewportMin(b))
            | (ViewportMax(a), ViewportMax(b)) => about_eq(a.0, b.0, EPSILON),

            (Expr(a), Expr(b)) => a == b,

            (Dip(a), DipF32(b)) | (DipF32(b), Dip(a)) => about_eq(a.to_f32(), *b, EPSILON_100),
            (Px(a), PxF32(b)) | (PxF32(b), Px(a)) => about_eq(a.0 as f32, *b, EPSILON_100),

            _ => false,
        }
    }
}
impl fmt::Debug for Length {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Length::*;
        if f.alternate() {
            match self {
                Default => write!(f, "Length::Default"),
                Dip(e) => f.debug_tuple("Length::Dip").field(e).finish(),
                Px(e) => f.debug_tuple("Length::Px").field(e).finish(),
                Pt(e) => f.debug_tuple("Length::Pt").field(e).finish(),
                Relative(e) => f.debug_tuple("Length::Relative").field(e).finish(),
                Leftover(e) => f.debug_tuple("Length::Leftover").field(e).finish(),
                Em(e) => f.debug_tuple("Length::Em").field(e).finish(),
                RootEm(e) => f.debug_tuple("Length::RootEm").field(e).finish(),
                ViewportWidth(e) => f.debug_tuple("Length::ViewportWidth").field(e).finish(),
                ViewportHeight(e) => f.debug_tuple("Length::ViewportHeight").field(e).finish(),
                ViewportMin(e) => f.debug_tuple("Length::ViewportMin").field(e).finish(),
                ViewportMax(e) => f.debug_tuple("Length::ViewportMax").field(e).finish(),
                DipF32(e) => f.debug_tuple("Length::DipF32").field(e).finish(),
                PxF32(e) => f.debug_tuple("Length::PxF32").field(e).finish(),
                Expr(e) => f.debug_tuple("Length::Expr").field(e).finish(),
            }
        } else {
            match self {
                Default => write!(f, "Default"),
                Dip(e) => write!(f, "{}.dip()", e.to_f32()),
                Px(e) => write!(f, "{}.px()", e.0),
                Pt(e) => write!(f, "{e}.pt()"),
                Relative(e) => write!(f, "{}.pct()", e.0 * 100.0),
                Leftover(e) => write!(f, "{}.lft()", e.0),
                Em(e) => write!(f, "{}.em()", e.0),
                RootEm(e) => write!(f, "{}.rem()", e.0),
                ViewportWidth(e) => write!(f, "{e}.vw()"),
                ViewportHeight(e) => write!(f, "{e}.vh()"),
                ViewportMin(e) => write!(f, "{e}.vmin()"),
                ViewportMax(e) => write!(f, "{e}.vmax()"),
                DipF32(e) => write!(f, "{}.dip()", e),
                PxF32(e) => write!(f, "{}.px()", e),
                Expr(e) => write!(f, "{e}"),
            }
        }
    }
}
impl fmt::Display for Length {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Length::*;
        match self {
            Default => write!(f, "Default"),
            Dip(l) => write!(f, "{l}"),
            Px(l) => write!(f, "{l}px"),
            Pt(l) => write!(f, "{l}pt"),
            Relative(n) => write!(f, "{:.*}%", f.precision().unwrap_or(0), n.0 * 100.0),
            Leftover(l) => write!(f, "{l}lft"),
            Em(e) => write!(f, "{e}em"),
            RootEm(re) => write!(f, "{re}rem"),
            ViewportWidth(vw) => write!(f, "{vw}vw"),
            ViewportHeight(vh) => write!(f, "{vh}vh"),
            ViewportMin(vmin) => write!(f, "{vmin}vmin"),
            ViewportMax(vmax) => write!(f, "{vmax}vmax"),
            DipF32(l) => write!(f, "{l}dip"),
            PxF32(l) => write!(f, "{l}px"),
            Expr(e) => write!(f, "{e}"),
        }
    }
}
impl_from_and_into_var! {
    /// Conversion to [`Length::Relative`]
    fn from(percent: FactorPercent) -> Length {
        Length::Relative(percent.into())
    }

    /// Conversion to [`Length::Relative`]
    fn from(norm: Factor) -> Length {
        Length::Relative(norm)
    }

    /// Conversion to [`Length::DipF32`]
    fn from(f: f32) -> Length {
        Length::DipF32(f)
    }

    /// Conversion to [`Length::Dip`]
    fn from(i: i32) -> Length {
        Length::Dip(Dip::new(i))
    }

    /// Conversion to [`Length::Px`]
    fn from(l: Px) -> Length {
        Length::Px(l)
    }

    /// Conversion to [`Length::Dip`]
    fn from(l: Dip) -> Length {
        Length::Dip(l)
    }
}
impl Length {
    /// Length of exact zero.
    pub const fn zero() -> Length {
        Length::Px(Px(0))
    }

    /// Length that fills the available space.
    pub const fn fill() -> Length {
        Length::Relative(Factor(1.0))
    }

    /// Length that fills 50% of the available space.
    pub const fn half() -> Length {
        Length::Relative(Factor(0.5))
    }

    /// Returns a length that resolves to the maximum layout length between `self` and `other`.
    pub fn max(&self, other: impl Into<Length>) -> Length {
        use Length::*;
        match (self.clone(), other.into()) {
            (Default, Default) => Default,
            (Dip(a), Dip(b)) => Dip(a.max(b)),
            (Px(a), Px(b)) => Px(a.max(b)),
            (Pt(a), Pt(b)) => Pt(a.max(b)),
            (Relative(a), Relative(b)) => Relative(a.max(b)),
            (Leftover(a), Leftover(b)) => Leftover(a.max(b)),
            (Em(a), Em(b)) => Em(a.max(b)),
            (RootEm(a), RootEm(b)) => RootEm(a.max(b)),
            (ViewportWidth(a), ViewportWidth(b)) => ViewportWidth(a.max(b)),
            (ViewportHeight(a), ViewportHeight(b)) => ViewportHeight(a.max(b)),
            (ViewportMin(a), ViewportMin(b)) => ViewportMin(a.max(b)),
            (ViewportMax(a), ViewportMax(b)) => ViewportMax(a.max(b)),
            (DipF32(a), DipF32(b)) => DipF32(a.max(b)),
            (PxF32(a), PxF32(b)) => PxF32(a.max(b)),
            (DipF32(a), Dip(b)) | (Dip(b), DipF32(a)) => DipF32(a.max(b.to_f32())),
            (PxF32(a), Px(b)) | (Px(b), PxF32(a)) => PxF32(a.max(b.0 as f32)),
            (a, b) => Expr(Box::new(LengthExpr::Max(a, b))),
        }
    }

    /// Returns a length that resolves to the minimum layout length between `self` and `other`.
    pub fn min(&self, other: impl Into<Length>) -> Length {
        use Length::*;
        match (self.clone(), other.into()) {
            (Default, Default) => Default,
            (Dip(a), Dip(b)) => Dip(a.min(b)),
            (Px(a), Px(b)) => Px(a.min(b)),
            (Pt(a), Pt(b)) => Pt(a.min(b)),
            (Relative(a), Relative(b)) => Relative(a.min(b)),
            (Leftover(a), Leftover(b)) => Leftover(a.min(b)),
            (Em(a), Em(b)) => Em(a.min(b)),
            (RootEm(a), RootEm(b)) => RootEm(a.min(b)),
            (ViewportWidth(a), ViewportWidth(b)) => ViewportWidth(a.min(b)),
            (ViewportHeight(a), ViewportHeight(b)) => ViewportHeight(a.min(b)),
            (ViewportMin(a), ViewportMin(b)) => ViewportMin(a.min(b)),
            (ViewportMax(a), ViewportMax(b)) => ViewportMax(a.min(b)),
            (DipF32(a), DipF32(b)) => DipF32(a.min(b)),
            (PxF32(a), PxF32(b)) => PxF32(a.min(b)),
            (DipF32(a), Dip(b)) | (Dip(b), DipF32(a)) => DipF32(a.min(b.to_f32())),
            (PxF32(a), Px(b)) | (Px(b), PxF32(a)) => PxF32(a.min(b.0 as f32)),
            (a, b) => Expr(Box::new(LengthExpr::Min(a, b))),
        }
    }

    /// Returns a length that constrains the computed layout length between `min` and `max`.
    pub fn clamp(&self, min: impl Into<Length>, max: impl Into<Length>) -> Length {
        self.max(min).min(max)
    }

    /// Returns a length that computes the absolute layout length of `self`.
    pub fn abs(&self) -> Length {
        use Length::*;
        match self {
            Default => Expr(Box::new(LengthExpr::Abs(Length::Default))),
            Dip(e) => Dip(e.abs()),
            Px(e) => Px(e.abs()),
            Pt(e) => Pt(e.abs()),
            Relative(r) => Relative(r.abs()),
            Leftover(r) => Leftover(r.abs()),
            Em(e) => Em(e.abs()),
            RootEm(r) => RootEm(r.abs()),
            ViewportWidth(w) => ViewportWidth(w.abs()),
            ViewportHeight(h) => ViewportHeight(h.abs()),
            ViewportMin(m) => ViewportMin(m.abs()),
            ViewportMax(m) => ViewportMax(m.abs()),
            DipF32(e) => DipF32(e.abs()),
            PxF32(e) => PxF32(e.abs()),
            Expr(e) => Expr(Box::new(LengthExpr::Abs(Length::Expr(e.clone())))),
        }
    }

    /// Compute the length at a context.
    ///
    /// Note that the result is not clamped by the [constrains], they are only used to compute the `Relative` value.
    ///
    /// The `default_value` closure is evaluated for every [`Default`] value is request, it can be more then once for [`Expr`] lengths,
    /// the closure value must not be expensive to produce, we only use a closure to avoid flagging layout dependencies in values that
    /// are only used for [`Default`].
    ///
    /// [constrains]: Layout1dMetrics::constrains
    /// [`Default`]: Length::Default
    /// [`Relative`]: Length::Relative
    /// [`Expr`]: Length::Expr
    pub fn layout(&self, ctx: Layout1dMetrics, default_value: impl FnMut(Layout1dMetrics) -> Px) -> Px {
        #[cfg(dyn_closure)]
        let default_value: Box<dyn FnMut(Layout1dMetrics) -> Px> = Box::new(default_value);
        self.layout_impl(ctx, default_value)
    }
    fn layout_impl(&self, ctx: Layout1dMetrics, mut default_value: impl FnMut(Layout1dMetrics) -> Px) -> Px {
        use Length::*;
        match self {
            Default => default_value(ctx),
            Dip(l) => l.to_px(ctx.scale_factor().0),
            Px(l) => *l,
            Pt(l) => Self::pt_to_px(*l, ctx.scale_factor()),
            Relative(f) => ctx.constrains().fill() * f.0,
            Leftover(f) => {
                if let Some(l) = ctx.leftover_length() {
                    l
                } else {
                    ctx.constrains().fill() * f.0
                }
            }
            Em(f) => ctx.font_size() * f.0,
            RootEm(f) => ctx.root_font_size() * f.0,
            ViewportWidth(p) => ctx.viewport().width * *p,
            ViewportHeight(p) => ctx.viewport().height * *p,
            ViewportMin(p) => ctx.viewport_min() * *p,
            ViewportMax(p) => ctx.viewport_max() * *p,
            DipF32(l) => self::Px((l * ctx.scale_factor().0).round() as i32),
            PxF32(l) => self::Px(l.round() as i32),
            Expr(e) => e.layout(ctx, &mut default_value),
        }
    }

    /// Same operation as [`layout`] but without rounding to nearest pixel.
    ///
    /// [`layout`]: Self::layout
    pub fn layout_f32(&self, ctx: Layout1dMetrics, default_value: impl FnMut(Layout1dMetrics) -> f32) -> f32 {
        #[cfg(dyn_closure)]
        let default_value: Box<dyn FnMut(Layout1dMetrics) -> f32> = Box::new(default_value);
        self.layout_f32_impl(ctx, default_value)
    }

    fn layout_f32_impl(&self, ctx: Layout1dMetrics, mut default_value: impl FnMut(Layout1dMetrics) -> f32) -> f32 {
        use Length::*;
        match self {
            Default => default_value(ctx),
            Dip(l) => l.to_f32() * ctx.scale_factor().0,
            Px(l) => l.0 as f32,
            Pt(l) => Self::pt_to_px_f32(*l, ctx.scale_factor()),
            Relative(f) => ctx.constrains().fill().0 as f32 * f.0,
            Leftover(f) => {
                if let Some(l) = ctx.leftover_length() {
                    l.0 as f32
                } else {
                    ctx.constrains().fill().0 as f32 * f.0
                }
            }
            Em(f) => ctx.font_size().0 as f32 * f.0,
            RootEm(f) => ctx.root_font_size().0 as f32 * f.0,
            ViewportWidth(p) => ctx.viewport().width.0 as f32 * *p,
            ViewportHeight(p) => ctx.viewport().height.0 as f32 * *p,
            ViewportMin(p) => ctx.viewport_min().0 as f32 * *p,
            ViewportMax(p) => ctx.viewport_max().0 as f32 * *p,
            DipF32(l) => *l * ctx.scale_factor().0,
            PxF32(l) => *l,
            Expr(e) => e.layout_f32(ctx, &mut default_value),
        }
    }

    /// Compute a [`LayoutMask`] that flags all contextual values that affect the result of [`layout`].
    ///
    /// [`layout`]: Self::layout
    pub fn affect_mask(&self) -> LayoutMask {
        use Length::*;
        match self {
            Default => LayoutMask::DEFAULT_VALUE,
            Dip(_) => LayoutMask::SCALE_FACTOR,
            Px(_) => LayoutMask::NONE,
            Pt(_) => LayoutMask::SCALE_FACTOR,
            Relative(_) => LayoutMask::CONSTRAINS,
            Leftover(_) => LayoutMask::LEFTOVER,
            Em(_) => LayoutMask::FONT_SIZE,
            RootEm(_) => LayoutMask::ROOT_FONT_SIZE,
            ViewportWidth(_) => LayoutMask::VIEWPORT,
            ViewportHeight(_) => LayoutMask::VIEWPORT,
            ViewportMin(_) => LayoutMask::VIEWPORT,
            ViewportMax(_) => LayoutMask::VIEWPORT,
            DipF32(_) => LayoutMask::SCALE_FACTOR,
            PxF32(_) => LayoutMask::NONE,
            Expr(e) => e.affect_mask(),
        }
    }

    /// If this length is zero in any finite layout context.
    ///
    /// Returns `None` if the value depends on the input to [`layout`].
    ///
    /// [`Expr`]: Length::Expr
    /// [`layout`]: Length::layout
    pub fn is_zero(&self) -> Option<bool> {
        use Length::*;
        match self {
            Default => None,
            Dip(l) => Some(*l == self::Dip::new(0)),
            Px(l) => Some(*l == self::Px(0)),
            Pt(l) => Some(l.abs() < EPSILON),
            Relative(f) => Some(f.0.abs() < EPSILON),
            Leftover(f) => Some(f.0.abs() < EPSILON),
            Em(f) => Some(f.0.abs() < EPSILON),
            RootEm(f) => Some(f.0.abs() < EPSILON),
            ViewportWidth(p) => Some(p.0.abs() < EPSILON),
            ViewportHeight(p) => Some(p.0.abs() < EPSILON),
            ViewportMin(p) => Some(p.0.abs() < EPSILON),
            ViewportMax(p) => Some(p.0.abs() < EPSILON),
            DipF32(l) => Some(about_eq(*l, 0.0, EPSILON_100)),
            PxF32(l) => Some(about_eq(*l, 0.0, EPSILON_100)),
            Expr(_) => None,
        }
    }

    /// Convert a `pt` unit value to [`Px`] given a `scale_factor`.
    pub fn pt_to_px(pt: f32, scale_factor: Factor) -> Px {
        let px = Self::pt_to_px_f32(pt, scale_factor);
        Px(px.round() as i32)
    }

    /// Same operation as [`pt_to_px`] but without rounding to nearest pixel.
    ///
    /// [`pt_to_px`]: Self::pt_to_px
    pub fn pt_to_px_f32(pt: f32, scale_factor: Factor) -> f32 {
        pt * Self::PT_TO_DIP * scale_factor.0
    }

    /// Convert a [`Px`] unit value to a `Pt` value given a `scale_factor`.
    pub fn px_to_pt(px: Px, scale_factor: Factor) -> f32 {
        let dip = px.0 as f32 / scale_factor.0;
        dip / Self::PT_TO_DIP
    }

    /// If is [`Length::Default`].
    pub fn is_default(&self) -> bool {
        matches!(self, Length::Default)
    }

    /// Replaces `self` with `overwrite` if `self` is [`Default`].
    ///
    /// [`Default`]: Length::Default
    pub fn replace_default(&mut self, overwrite: &Length) {
        if self.is_default() {
            *self = overwrite.clone();
        }
    }

    /// Convert [`PxF32`] to [`Px`] and [`DipF32`] to [`Dip`].
    ///
    /// [`PxF32`]: Self::PxF32
    /// [`Px`]: Self::Px
    /// [`DipF32`]: Self::DipF32
    /// [`Dip`]: Self::Dip
    pub fn round_exact(&mut self) {
        match self {
            Length::PxF32(l) => *self = Length::Px(Px(l.round() as i32)),
            Length::DipF32(l) => *self = Length::Dip(Dip::new_f32(*l)),
            _ => {}
        }
    }

    /// 96.0 / 72.0
    const PT_TO_DIP: f32 = 96.0 / 72.0; // 1.3333..;
}

bitflags! {
    /// Mask of values that can affect the [`Length::layout`] operation.
    pub struct LayoutMask: u32 {
        /// Represents no value dependency or change.
        const NONE = 0;

        /// The `default_value`.
        const DEFAULT_VALUE = 1 << 31;
        /// The `constrains`.
        const CONSTRAINS = 1 << 30;

        /// The [`LayoutMetrics::font_size`].
        const FONT_SIZE = 1;
        /// The [`LayoutMetrics::root_font_size`].
        const ROOT_FONT_SIZE = 1 << 1;
        /// The [`LayoutMetrics::scale_factor`].
        const SCALE_FACTOR = 1 << 2;
        /// The [`LayoutMetrics::viewport`].
        const VIEWPORT = 1 << 3;
        /// The [`LayoutMetrics::screen_ppi`].
        const SCREEN_PPI = 1 << 4;
        /// The [`LayoutMetrics::inline_advance`].
        const INLINE_ADVANCE = 1 << 5;
        /// The [`LayoutMetrics::direction`].
        const DIRECTION = 1 << 6;
        /// The [`LayoutMetrics::leftover`] and [`LayoutMetrics::leftover_count`].
        const LEFTOVER = 1 << 7;
    }
}
impl Default for LayoutMask {
    /// `NONE`.
    fn default() -> Self {
        LayoutMask::NONE
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
    Mul(Length, Factor),
    /// Divide the layout length by the factor.
    Div(Length, Factor),
    /// Maximum layout length.
    Max(Length, Length),
    /// Minimum layout length.
    Min(Length, Length),
    /// Computes the absolute layout length.
    Abs(Length),
    /// Negate the layout length.
    Neg(Length),
}
impl LengthExpr {
    /// Evaluate the expression at a layout context.
    pub fn layout(&self, ctx: Layout1dMetrics, default_value: &mut dyn FnMut(Layout1dMetrics) -> Px) -> Px {
        let l = self.layout_f32(ctx, &mut |ctx| default_value(ctx).0 as f32);
        Px(l.round() as i32)
    }

    /// Same operation as [`layout`] but without rounding to nearest pixel.
    ///
    /// [`layout`]: Self::layout
    pub fn layout_f32(&self, ctx: Layout1dMetrics, mut default_value: &mut dyn FnMut(Layout1dMetrics) -> f32) -> f32 {
        use LengthExpr::*;
        match self {
            Add(a, b) => a.layout_f32(ctx, &mut default_value) + b.layout_f32(ctx, default_value),
            Sub(a, b) => a.layout_f32(ctx, &mut default_value) - b.layout_f32(ctx, default_value),
            Mul(l, s) => l.layout_f32(ctx, default_value) * s.0,
            Div(l, s) => l.layout_f32(ctx, default_value) / s.0,
            Max(a, b) => {
                let a = a.layout_f32(ctx, &mut default_value);
                let b = b.layout_f32(ctx, default_value);
                a.max(b)
            }
            Min(a, b) => {
                let a = a.layout_f32(ctx, &mut default_value);
                let b = b.layout_f32(ctx, default_value);
                a.min(b)
            }
            Abs(e) => e.layout_f32(ctx, default_value).abs(),
            Neg(e) => -e.layout_f32(ctx, default_value),
        }
    }

    /// Compute a [`LayoutMask`] that flags all contextual values that affect the result
    /// of [`layout`] called for this length.
    ///
    /// [`layout`]: Self::layout
    pub fn affect_mask(&self) -> LayoutMask {
        use LengthExpr::*;
        match self {
            Add(a, b) => a.affect_mask() | b.affect_mask(),
            Sub(a, b) => a.affect_mask() | b.affect_mask(),
            Mul(a, _) => a.affect_mask(),
            Div(a, _) => a.affect_mask(),
            Max(a, b) => a.affect_mask() | b.affect_mask(),
            Min(a, b) => a.affect_mask() | b.affect_mask(),
            Abs(a) => a.affect_mask(),
            Neg(a) => a.affect_mask(),
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
                Neg(e) => f.debug_tuple("LengthExpr::Neg").field(e).finish(),
            }
        } else {
            match self {
                Add(a, b) => write!(f, "({a:?} + {b:?})"),
                Sub(a, b) => write!(f, "({a:?} - {b:?})"),
                Mul(l, s) => write!(f, "({l:?} * {:?}.pct())", s.0 * 100.0),
                Div(l, s) => write!(f, "({l:?} / {:?}.pct())", s.0 * 100.0),
                Max(a, b) => write!(f, "max({a:?}, {b:?})"),
                Min(a, b) => write!(f, "min({a:?}, {b:?})"),
                Abs(e) => write!(f, "abs({e:?})"),
                Neg(e) => write!(f, "-({e:?})"),
            }
        }
    }
}
impl fmt::Display for LengthExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use LengthExpr::*;
        match self {
            Add(a, b) => write!(f, "({a} + {b})"),
            Sub(a, b) => write!(f, "({a} - {b})"),
            Mul(l, s) => write!(f, "({l} * {}%)", s.0 * 100.0),
            Div(l, s) => write!(f, "({l} / {}%)", s.0 * 100.0),
            Max(a, b) => write!(f, "max({a}, {b})"),
            Min(a, b) => write!(f, "min({a}, {b})"),
            Abs(e) => write!(f, "abs({e})"),
            Neg(e) => write!(f, "-({e})"),
        }
    }
}

/// Extension methods for initializing [`Length`] units.
///
/// This trait is implemented for [`f32`] and [`u32`] allowing initialization of length units using the `<number>.<unit>()` syntax.
///
/// # Examples
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
/// let relative_size: Length = 100.pct().into();// FactorUnits
/// let relative_size: Length = 1.0.fct().into();// FactorUnits
/// ```
pub trait LengthUnits {
    /// Exact size in device independent pixels.
    ///
    /// Returns [`Length::Dip`].
    fn dip(self) -> Length;

    /// Exact size in device pixels.
    ///
    /// Returns [`Length::Px`].
    fn px(self) -> Length;

    /// Exact size in font units.
    ///
    /// Returns [`Length::Pt`].
    fn pt(self) -> Length;

    /// Factor of the font-size of the widget.
    ///
    /// Returns [`Length::Em`].
    fn em(self) -> Length;

    /// Percentage of the font-size of the widget.
    ///
    /// Returns [`Length::Em`].
    fn em_pct(self) -> Length;

    /// Factor of the font-size of the root widget.
    ///
    /// Returns [`Length::RootEm`].
    fn rem(self) -> Length;

    /// Percentage of the font-size of the root widget.
    ///
    /// Returns [`Length::RootEm`].
    fn rem_pct(self) -> Length;

    /// Factor of the width of the nearest viewport ancestor.
    ///
    /// Returns [`Length::ViewportWidth`].
    fn vw(self) -> Length;

    /// Percentage of the width of the nearest viewport ancestor.
    ///
    /// Returns [`Length::ViewportWidth`].
    fn vw_pct(self) -> Length;

    /// Factor of the height of the nearest viewport ancestor.
    ///
    /// Returns [`Length::ViewportHeight`].
    fn vh(self) -> Length;

    /// Percentage of the height of the nearest viewport ancestor.
    ///
    /// Returns [`Length::ViewportHeight`].
    fn vh_pct(self) -> Length;

    /// Factor of the smallest of the nearest viewport's dimensions.
    ///
    /// Returns [`Length::ViewportMin`].
    fn vmin(self) -> Length;

    /// Percentage of the smallest of the nearest viewport's dimensions.
    ///
    /// Returns [`Length::ViewportMin`].
    fn vmin_pct(self) -> Length;

    /// Factor of the largest of the nearest viewport's dimensions.
    ///
    /// Returns [`Length::ViewportMax`].
    fn vmax(self) -> Length;

    /// Percentage of the largest of the nearest viewport's dimensions.
    ///
    /// Returns [`Length::ViewportMax`].
    fn vmax_pct(self) -> Length;

    /// Factor of the leftover layout space.
    ///
    /// Note that this unit must be supported by the parent panel widget and property, otherwise it evaluates
    /// like [`length::Relative`].
    ///
    /// Returns [`Length::Leftover`].
    fn lft(self) -> Length;
}
impl LengthUnits for f32 {
    fn dip(self) -> Length {
        Length::DipF32(self)
    }

    fn px(self) -> Length {
        Length::PxF32(self)
    }

    fn pt(self) -> Length {
        Length::Pt(self)
    }

    fn em(self) -> Length {
        Length::Em(self.into())
    }

    fn rem(self) -> Length {
        Length::RootEm(self.into())
    }

    fn vw(self) -> Length {
        Length::ViewportWidth(self.into())
    }

    fn vh(self) -> Length {
        Length::ViewportHeight(self.into())
    }

    fn vmin(self) -> Length {
        Length::ViewportMin(self.into())
    }

    fn vmax(self) -> Length {
        Length::ViewportMax(self.into())
    }

    fn em_pct(self) -> Length {
        Length::Em(self.pct().into())
    }

    fn rem_pct(self) -> Length {
        Length::RootEm(self.pct().into())
    }

    fn vw_pct(self) -> Length {
        Length::ViewportWidth(self.pct().into())
    }

    fn vh_pct(self) -> Length {
        Length::ViewportHeight(self.pct().into())
    }

    fn vmin_pct(self) -> Length {
        Length::ViewportMin(self.pct().into())
    }

    fn vmax_pct(self) -> Length {
        Length::ViewportMax(self.pct().into())
    }

    fn lft(self) -> Length {
        Length::Leftover(self.fct())
    }
}
impl LengthUnits for i32 {
    fn dip(self) -> Length {
        Length::Dip(Dip::new(self))
    }

    fn px(self) -> Length {
        Length::Px(Px(self))
    }

    fn pt(self) -> Length {
        Length::Pt(self as f32)
    }

    fn em(self) -> Length {
        Length::Em(self.fct())
    }

    fn rem(self) -> Length {
        Length::RootEm(self.fct())
    }

    fn vw(self) -> Length {
        Length::ViewportWidth(self.fct())
    }

    fn vh(self) -> Length {
        Length::ViewportHeight(self.fct())
    }

    fn vmin(self) -> Length {
        Length::ViewportMin(self.fct())
    }

    fn vmax(self) -> Length {
        Length::ViewportMax(self.fct())
    }

    fn em_pct(self) -> Length {
        Length::Em(self.pct().into())
    }

    fn rem_pct(self) -> Length {
        Length::RootEm(self.pct().into())
    }

    fn vw_pct(self) -> Length {
        Length::ViewportWidth(self.pct().into())
    }

    fn vh_pct(self) -> Length {
        Length::ViewportHeight(self.pct().into())
    }

    fn vmin_pct(self) -> Length {
        Length::ViewportMin(self.pct().into())
    }

    fn vmax_pct(self) -> Length {
        Length::ViewportMax(self.pct().into())
    }

    fn lft(self) -> Length {
        Length::Leftover(self.fct())
    }
}
