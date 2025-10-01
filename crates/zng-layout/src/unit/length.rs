use super::{
    ByteLength, ByteUnits, Dip, DipToPx, EQ_GRANULARITY, EQ_GRANULARITY_100, Factor, FactorPercent, FactorUnits, LayoutAxis, Px, about_eq,
};
use std::{fmt, mem, ops};

use zng_unit::about_eq_hash;
use zng_var::{
    animation::{Transitionable, easing::EasingStep},
    impl_from_and_into_var,
};

use crate::{
    context::{LAYOUT, LayoutMask},
    unit::ParseCompositeError,
};

mod expr;
pub use expr::*;

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
/// * Viewport lengths uses [`about_eq`] with `0.00001` granularity.
#[derive(Clone, serde::Serialize, serde::Deserialize)] // TODO(breaking) non_exhaustive
pub enum Length {
    /// The default (initial) value.
    Default,
    /// The exact length in device independent units.
    Dip(Dip),
    /// The exact length in device pixel units.
    Px(Px),
    /// The exact length in font points.
    Pt(f32),
    /// Relative to the fill length.
    Factor(Factor),
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

        let rhs = rhs.into();

        if self.is_zero() == Some(true) {
            return rhs; // 0 + rhs
        } else if rhs.is_zero() == Some(true) {
            return self; // self + 0
        }

        match (self, rhs) {
            (Dip(a), Dip(b)) => Dip(a + b),
            (Px(a), Px(b)) => Px(a + b),
            (Pt(a), Pt(b)) => Pt(a + b),
            (Factor(a), Factor(b)) => Factor(a + b),
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
            (a, b) => LengthExpr::Add(a, b).to_length_checked(),
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

        let rhs = rhs.into();

        if rhs.is_zero() == Some(true) {
            return self; // self - 0
        } else if self.is_zero() == Some(true) {
            return -rhs; // 0 - rhs
        }

        match (self, rhs) {
            (Dip(a), Dip(b)) => Dip(a - b),
            (Px(a), Px(b)) => Px(a - b),
            (Pt(a), Pt(b)) => Pt(a - b),
            (Factor(a), Factor(b)) => Factor(a - b),
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
            (a, b) => LengthExpr::Sub(a, b).to_length_checked(),
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

        if self.is_zero() == Some(true) || rhs == 1.fct() {
            return self; // 0 * fct || len * 1.0
        } else if rhs == 0.fct() {
            return Self::zero(); // len * 0.0
        }

        match self {
            Dip(e) => DipF32(e.to_f32() * rhs.0),
            Px(e) => PxF32(e.0 as f32 * rhs.0),
            Pt(e) => Pt(e * rhs.0),
            Factor(r) => Factor(r * rhs),
            Leftover(r) => Leftover(r * rhs),
            Em(e) => Em(e * rhs),
            RootEm(e) => RootEm(e * rhs),
            ViewportWidth(w) => ViewportWidth(w * rhs),
            ViewportHeight(h) => ViewportHeight(h * rhs),
            ViewportMin(m) => ViewportMin(m * rhs),
            ViewportMax(m) => ViewportMax(m * rhs),
            DipF32(e) => DipF32(e * rhs.0),
            PxF32(e) => PxF32(e * rhs.0),
            e => LengthExpr::Mul(e, rhs).to_length_checked(),
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

        if self.is_zero() == Some(true) && rhs != 0.fct() {
            return self; // 0 / fct
        }

        match self {
            Dip(e) => DipF32(e.to_f32() / rhs.0),
            Px(e) => PxF32(e.0 as f32 / rhs.0),
            Pt(e) => Pt(e / rhs.0),
            Factor(r) => Factor(r / rhs),
            Leftover(r) => Leftover(r / rhs),
            Em(e) => Em(e / rhs),
            RootEm(e) => RootEm(e / rhs),
            ViewportWidth(w) => ViewportWidth(w / rhs),
            ViewportHeight(h) => ViewportHeight(h / rhs),
            ViewportMin(m) => ViewportMin(m / rhs),
            ViewportMax(m) => ViewportMax(m / rhs),
            DipF32(e) => DipF32(e / rhs.0),
            PxF32(e) => PxF32(e / rhs.0),
            e => LengthExpr::Div(e, rhs).to_length_checked(),
        }
    }
}
impl<F: Into<Factor>> ops::DivAssign<F> for Length {
    fn div_assign(&mut self, rhs: F) {
        let lhs = mem::take(self);
        *self = lhs / rhs.into();
    }
}
impl Transitionable for Length {
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        use Length::*;

        if step == 0.fct() {
            return self;
        }
        if step == 1.fct() {
            return to.clone();
        }

        match (self, to) {
            (Dip(a), Dip(b)) => Dip(a.lerp(b, step)),
            (Px(a), Px(b)) => Px(a.lerp(b, step)),
            (Pt(a), Pt(b)) => Pt(a.lerp(b, step)),
            (Factor(a), Factor(b)) => Factor(a.lerp(b, step)),
            (Leftover(a), Leftover(b)) => Leftover(a.lerp(b, step)),
            (Em(a), Em(b)) => Em(a.lerp(b, step)),
            (RootEm(a), RootEm(b)) => RootEm(a.lerp(b, step)),
            (ViewportWidth(a), ViewportWidth(b)) => ViewportWidth(a.lerp(b, step)),
            (ViewportHeight(a), ViewportHeight(b)) => ViewportHeight(a.lerp(b, step)),
            (ViewportMin(a), ViewportMin(b)) => ViewportMin(a.lerp(b, step)),
            (ViewportMax(a), ViewportMax(b)) => ViewportMax(a.lerp(b, step)),
            (PxF32(a), PxF32(b)) => PxF32(a.lerp(b, step)),
            (DipF32(a), DipF32(b)) => DipF32(a.lerp(b, step)),
            (Px(a), PxF32(b)) => PxF32((a.0 as f32).lerp(b, step)),
            (PxF32(a), Px(b)) => PxF32(a.lerp(&(b.0 as f32), step)),
            (Dip(a), DipF32(b)) => DipF32(a.to_f32().lerp(b, step)),
            (DipF32(a), Dip(b)) => DipF32(a.lerp(&b.to_f32(), step)),
            (a, b) => LengthExpr::Lerp(a, b.clone(), step).to_length_checked(),
        }
    }
}
impl ops::Neg for Length {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            Length::Default => LengthExpr::Neg(Length::Default).to_length_checked(),
            Length::Dip(e) => Length::Dip(-e),
            Length::Px(e) => Length::Px(-e),
            Length::Pt(e) => Length::Pt(-e),
            Length::Factor(e) => Length::Factor(-e),
            Length::Leftover(e) => Length::Leftover(-e),
            Length::Em(e) => Length::Em(-e),
            Length::RootEm(e) => Length::RootEm(-e),
            Length::ViewportWidth(e) => Length::ViewportWidth(-e),
            Length::ViewportHeight(e) => Length::ViewportHeight(-e),
            Length::ViewportMin(e) => Length::ViewportMin(-e),
            Length::ViewportMax(e) => Length::ViewportMax(-e),
            Length::DipF32(e) => Length::DipF32(-e),
            Length::PxF32(e) => Length::PxF32(-e),
            Length::Expr(e) => LengthExpr::Neg(Length::Expr(e)).to_length_checked(),
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
            (Pt(a), Pt(b)) => about_eq(*a, *b, EQ_GRANULARITY_100),

            (DipF32(a), DipF32(b)) | (PxF32(a), PxF32(b)) => about_eq(*a, *b, EQ_GRANULARITY_100),

            (Factor(a), Factor(b))
            | (Em(a), Em(b))
            | (RootEm(a), RootEm(b))
            | (Leftover(a), Leftover(b))
            | (ViewportWidth(a), ViewportWidth(b))
            | (ViewportHeight(a), ViewportHeight(b))
            | (ViewportMin(a), ViewportMin(b))
            | (ViewportMax(a), ViewportMax(b)) => a == b,

            (Expr(a), Expr(b)) => a == b,

            (Dip(a), DipF32(b)) | (DipF32(b), Dip(a)) => about_eq(a.to_f32(), *b, EQ_GRANULARITY_100),
            (Px(a), PxF32(b)) | (PxF32(b), Px(a)) => about_eq(a.0 as f32, *b, EQ_GRANULARITY_100),

            (a, b) => {
                debug_assert_ne!(std::mem::discriminant(a), std::mem::discriminant(b));
                false
            }
        }
    }
}
impl Eq for Length {}
impl std::hash::Hash for Length {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Length::Default => {}
            Length::Dip(dip) => dip.hash(state),
            Length::Px(px) => px.hash(state),
            Length::Factor(factor)
            | Length::Leftover(factor)
            | Length::Em(factor)
            | Length::RootEm(factor)
            | Length::ViewportWidth(factor)
            | Length::ViewportHeight(factor)
            | Length::ViewportMin(factor)
            | Length::ViewportMax(factor) => factor.hash(state),
            Length::DipF32(f) | Length::PxF32(f) | Length::Pt(f) => about_eq_hash(*f, EQ_GRANULARITY_100, state),
            Length::Expr(length_expr) => length_expr.hash(state),
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
                Factor(e) => f.debug_tuple("Length::Factor").field(e).finish(),
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
                Factor(e) => write!(f, "{}.pct()", e.0 * 100.0),
                Leftover(e) => write!(f, "{}.lft()", e.0),
                Em(e) => write!(f, "{}.em()", e.0),
                RootEm(e) => write!(f, "{}.rem()", e.0),
                ViewportWidth(e) => write!(f, "{e}.vw()"),
                ViewportHeight(e) => write!(f, "{e}.vh()"),
                ViewportMin(e) => write!(f, "{e}.vmin()"),
                ViewportMax(e) => write!(f, "{e}.vmax()"),
                DipF32(e) => write!(f, "{e}.dip()"),
                PxF32(e) => write!(f, "{e}.px()"),
                Expr(e) => write!(f, "{e}"),
            }
        }
    }
}
impl fmt::Display for Length {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Length::*;
        match self {
            Default => write!(f, "default"),
            Dip(l) => write!(f, "{l}"),
            Px(l) => write!(f, "{l}"),
            Pt(l) => write!(f, "{l}pt"),
            Factor(n) => write!(f, "{:.*}%", f.precision().unwrap_or(0), n.0 * 100.0),
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
impl std::str::FromStr for Length {
    type Err = ParseCompositeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "default" || s == "Default" {
            Ok(Self::Default)
        } else if let Some(dip) = s.strip_suffix("dip").or_else(|| s.strip_suffix(".dip()")) {
            if dip.contains('.') {
                Ok(Self::DipF32(dip.parse()?))
            } else {
                Ok(Self::Dip(Dip::new_f32(dip.parse()?)))
            }
        } else if let Some(px) = s.strip_suffix("px").or_else(|| s.strip_suffix(".px()")) {
            if px.contains('.') {
                Ok(Self::PxF32(px.parse()?))
            } else {
                Ok(Self::Px(Px(px.parse()?)))
            }
        } else if let Some(pt) = s.strip_suffix("pt").or_else(|| s.strip_suffix(".pt()")) {
            Ok(Self::Pt(pt.parse()?))
        } else if let Some(fct) = s.strip_suffix("fct").or_else(|| s.strip_suffix(".fct()")) {
            Ok(Self::Factor(Factor(fct.parse()?)))
        } else if let Some(fct) = s.strip_suffix("%").or_else(|| s.strip_suffix(".pct()")) {
            Ok(Self::Factor(FactorPercent(fct.parse()?).fct()))
        } else if let Some(lft) = s.strip_suffix("lft").or_else(|| s.strip_suffix(".lft()")) {
            Ok(Self::Leftover(Factor(lft.parse()?)))
        } else if let Some(em) = s.strip_suffix("em").or_else(|| s.strip_suffix(".em()")) {
            Ok(Self::Em(Factor(em.parse()?)))
        } else if let Some(root_em) = s.strip_suffix("rem").or_else(|| s.strip_suffix(".rem()")) {
            Ok(Self::RootEm(Factor(root_em.parse()?)))
        } else if let Some(vw) = s.strip_suffix("vw").or_else(|| s.strip_suffix(".vw()")) {
            Ok(Self::ViewportWidth(Factor(vw.parse()?)))
        } else if let Some(vh) = s.strip_suffix("vh").or_else(|| s.strip_suffix(".vh()")) {
            Ok(Self::ViewportHeight(Factor(vh.parse()?)))
        } else if let Some(v_min) = s.strip_suffix("vmin").or_else(|| s.strip_suffix(".vmin()")) {
            Ok(Self::ViewportMin(Factor(v_min.parse()?)))
        } else if let Some(v_max) = s.strip_suffix("vmax").or_else(|| s.strip_suffix(".vmax()")) {
            Ok(Self::ViewportMax(Factor(v_max.parse()?)))
        } else if let Ok(int) = s.parse::<i32>() {
            Ok(Self::Dip(Dip::new(int)))
        } else if let Ok(float) = s.parse::<f32>() {
            Ok(Self::DipF32(float))
        } else {
            Ok(Self::Expr(Box::new(s.parse()?)))
        }
    }
}
impl_from_and_into_var! {
    /// Conversion to [`Length::Factor`]
    fn from(percent: FactorPercent) -> Length {
        Length::Factor(percent.into())
    }

    /// Conversion to [`Length::Factor`]
    fn from(norm: Factor) -> Length {
        Length::Factor(norm)
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

    fn from(expr: LengthExpr) -> Length {
        Length::Expr(Box::new(expr))
    }
}
impl Length {
    /// Length of exact zero.
    pub const fn zero() -> Length {
        Length::Px(Px(0))
    }

    /// Length that fills the available space.
    pub const fn fill() -> Length {
        Length::Factor(Factor(1.0))
    }

    /// Length that fills 50% of the available space.
    pub const fn half() -> Length {
        Length::Factor(Factor(0.5))
    }

    /// Returns a length that resolves to the maximum layout length between `self` and `other`.
    pub fn max(&self, other: impl Into<Length>) -> Length {
        use Length::*;
        match (self.clone(), other.into()) {
            (Default, Default) => Default,
            (Dip(a), Dip(b)) => Dip(a.max(b)),
            (Px(a), Px(b)) => Px(a.max(b)),
            (Pt(a), Pt(b)) => Pt(a.max(b)),
            (Factor(a), Factor(b)) => Factor(a.max(b)),
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
            (a, b) => LengthExpr::Max(a, b).to_length_checked(),
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
            (Factor(a), Factor(b)) => Factor(a.min(b)),
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
            (a, b) => LengthExpr::Min(a, b).to_length_checked(),
        }
    }

    /// Returns a length that constraints the computed layout length between `min` and `max`.
    pub fn clamp(&self, min: impl Into<Length>, max: impl Into<Length>) -> Length {
        self.max(min).min(max)
    }

    /// Returns a length that computes the absolute layout length of `self`.
    pub fn abs(&self) -> Length {
        use Length::*;
        match self {
            Default => LengthExpr::Abs(Length::Default).to_length_checked(),
            Dip(e) => Dip(e.abs()),
            Px(e) => Px(e.abs()),
            Pt(e) => Pt(e.abs()),
            Factor(r) => Factor(r.abs()),
            Leftover(r) => Leftover(r.abs()),
            Em(e) => Em(e.abs()),
            RootEm(r) => RootEm(r.abs()),
            ViewportWidth(w) => ViewportWidth(w.abs()),
            ViewportHeight(h) => ViewportHeight(h.abs()),
            ViewportMin(m) => ViewportMin(m.abs()),
            ViewportMax(m) => ViewportMax(m.abs()),
            DipF32(e) => DipF32(e.abs()),
            PxF32(e) => PxF32(e.abs()),
            Expr(e) => LengthExpr::Abs(Length::Expr(e.clone())).to_length_checked(),
        }
    }

    /// If this length is zero in any finite layout context.
    ///
    /// Returns `None` if the value depends on the default value or is an expression.
    pub fn is_zero(&self) -> Option<bool> {
        use Length::*;
        match self {
            Default => None,
            Dip(l) => Some(*l == self::Dip::new(0)),
            Px(l) => Some(*l == self::Px(0)),
            Pt(l) => Some(about_eq(*l, 0.0, EQ_GRANULARITY)),
            Factor(f) | Leftover(f) | Em(f) | RootEm(f) | ViewportWidth(f) | ViewportHeight(f) | ViewportMin(f) | ViewportMax(f) => {
                Some(*f == 0.fct())
            }
            DipF32(l) => Some(about_eq(*l, 0.0, EQ_GRANULARITY_100)),
            PxF32(l) => Some(about_eq(*l, 0.0, EQ_GRANULARITY_100)),
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

    /// If is [`Default`].
    ///
    /// [`Default`]: Length::Default
    pub fn is_default(&self) -> bool {
        matches!(self, Length::Default)
    }

    /// If is [`Default`] or is [`Expr`] that contains defaults.
    ///
    /// [`Default`]: Length::Default
    /// [`Expr`]: Length::Expr
    pub fn has_default(&self) -> bool {
        match self {
            Length::Default => true,
            Length::Expr(e) => e.has_default(),
            _ => false,
        }
    }

    /// Replaces `self` with `overwrite` if `self` is [`Default`] or replace all defaults in [`Expr`].
    ///
    /// [`Default`]: Length::Default
    /// [`Expr`]: Length::Expr
    pub fn replace_default(&mut self, overwrite: &Length) {
        match self {
            Length::Default => *self = overwrite.clone(),
            Length::Expr(e) => e.replace_default(overwrite),
            _ => {}
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
            Length::Expr(e) => e.round_exact(),
            _ => {}
        }
    }

    /// Gets the total memory allocated by this length.
    ///
    /// This includes the sum of all nested [`Length::Expr`] heap memory.
    pub fn memory_used(&self) -> ByteLength {
        std::mem::size_of::<Length>().bytes() + self.heap_memory_used()
    }

    /// Sum total memory used in nested [`Length::Expr`] heap memory.
    pub fn heap_memory_used(&self) -> ByteLength {
        if let Length::Expr(e) = self { e.memory_used() } else { 0.bytes() }
    }

    /// 96.0 / 72.0
    const PT_TO_DIP: f32 = 96.0 / 72.0; // 1.3333..;
}
impl super::Layout1d for Length {
    fn layout_dft(&self, axis: LayoutAxis, default: Px) -> Px {
        use Length::*;
        match self {
            Default => default,
            Dip(l) => l.to_px(LAYOUT.scale_factor()),
            Px(l) => *l,
            Pt(l) => Self::pt_to_px(*l, LAYOUT.scale_factor()),
            Factor(f) => LAYOUT.constraints_for(axis).fill() * f.0,
            Leftover(f) => {
                if let Some(l) = LAYOUT.leftover_for(axis) {
                    l
                } else {
                    let fill = LAYOUT.constraints_for(axis).fill();
                    (fill * f.0).clamp(self::Px(0), fill)
                }
            }
            Em(f) => LAYOUT.font_size() * f.0,
            RootEm(f) => LAYOUT.root_font_size() * f.0,
            ViewportWidth(p) => LAYOUT.viewport().width * *p,
            ViewportHeight(p) => LAYOUT.viewport().height * *p,
            ViewportMin(p) => LAYOUT.viewport_min() * *p,
            ViewportMax(p) => LAYOUT.viewport_max() * *p,
            DipF32(l) => self::Px((l * LAYOUT.scale_factor().0).round() as i32),
            PxF32(l) => self::Px(l.round() as i32),
            Expr(e) => e.layout_dft(axis, default),
        }
    }

    fn layout_f32_dft(&self, axis: LayoutAxis, default: f32) -> f32 {
        use Length::*;
        match self {
            Default => default,
            Dip(l) => l.to_f32() * LAYOUT.scale_factor().0,
            Px(l) => l.0 as f32,
            Pt(l) => Self::pt_to_px_f32(*l, LAYOUT.scale_factor()),
            Factor(f) => LAYOUT.constraints_for(axis).fill().0 as f32 * f.0,
            Leftover(f) => {
                if let Some(l) = LAYOUT.leftover_for(axis) {
                    l.0 as f32
                } else {
                    let fill = LAYOUT.constraints_for(axis).fill().0 as f32;
                    (fill * f.0).clamp(0.0, fill)
                }
            }
            Em(f) => LAYOUT.font_size().0 as f32 * f.0,
            RootEm(f) => LAYOUT.root_font_size().0 as f32 * f.0,
            ViewportWidth(p) => LAYOUT.viewport().width.0 as f32 * *p,
            ViewportHeight(p) => LAYOUT.viewport().height.0 as f32 * *p,
            ViewportMin(p) => LAYOUT.viewport_min().0 as f32 * *p,
            ViewportMax(p) => LAYOUT.viewport_max().0 as f32 * *p,
            DipF32(l) => *l * LAYOUT.scale_factor().0,
            PxF32(l) => *l,
            Expr(e) => e.layout_f32_dft(axis, default),
        }
    }

    fn affect_mask(&self) -> LayoutMask {
        use Length::*;
        match self {
            Default => LayoutMask::DEFAULT_VALUE,
            Dip(_) => LayoutMask::SCALE_FACTOR,
            Px(_) => LayoutMask::empty(),
            Pt(_) => LayoutMask::SCALE_FACTOR,
            Factor(_) => LayoutMask::CONSTRAINTS,
            Leftover(_) => LayoutMask::LEFTOVER,
            Em(_) => LayoutMask::FONT_SIZE,
            RootEm(_) => LayoutMask::ROOT_FONT_SIZE,
            ViewportWidth(_) => LayoutMask::VIEWPORT,
            ViewportHeight(_) => LayoutMask::VIEWPORT,
            ViewportMin(_) => LayoutMask::VIEWPORT,
            ViewportMax(_) => LayoutMask::VIEWPORT,
            DipF32(_) => LayoutMask::SCALE_FACTOR,
            PxF32(_) => LayoutMask::empty(),
            Expr(e) => e.affect_mask(),
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
/// # use zng_layout::unit::*;
/// let font_size = 1.em();
/// let root_font_size = 1.rem();
/// let viewport_width = 100.vw();
/// let viewport_height = 100.vh();
/// let viewport_min = 100.vmin(); // min(width, height)
/// let viewport_max = 100.vmax(); // max(width, height)
///
/// // other length units not provided by `LengthUnits`:
///
/// let exact_size: Length = 500.into();
/// let relative_size: Length = 100.pct().into(); // FactorUnits
/// let relative_size: Length = 1.0.fct().into(); // FactorUnits
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

    /// Factor of the fill length.
    ///
    /// This is the same as [`FactorUnits::fct`], but produces a [`Length`] directly. This might be needed
    /// in places that don't automatically convert [`Factor`] to [`Length`].
    ///
    /// Returns [`Length::Factor`].
    fn fct_l(self) -> Length;

    /// Percentage of the fill length.
    ///
    /// This is the same as [`FactorUnits::pct`], but produces a [`Length`] directly. This might be needed
    /// in places that don't automatically convert [`FactorPercent`] to [`Length`].
    ///
    /// Returns [`Length::Factor`].
    fn pct_l(self) -> Length;

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
    /// like a single item having all the available fill space as leftover space.
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

    fn fct_l(self) -> Length {
        Length::Factor(self.fct())
    }

    fn pct_l(self) -> Length {
        Length::Factor(self.pct().fct())
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

    fn fct_l(self) -> Length {
        Length::Factor(self.fct())
    }

    fn pct_l(self) -> Length {
        Length::Factor(self.pct().fct())
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
