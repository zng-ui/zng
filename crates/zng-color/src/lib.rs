#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Color and gradient types, functions and macros, [`Rgba`], [`filter`], [`hex!`] and more.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]
#![recursion_limit = "256"]

use std::{fmt, ops, sync::Arc};
use zng_app_context::context_local;

use zng_layout::unit::{AngleDegree, EQ_GRANULARITY, EQ_GRANULARITY_100, Factor, FactorUnits, about_eq, about_eq_hash};
use zng_var::{
    IntoVar, Var, VarValue,
    animation::{Transition, Transitionable, easing::EasingStep},
    context_var, expr_var, impl_from_and_into_var,
    types::ContextualizedVar,
};

pub use zng_view_api::config::ColorScheme;

#[doc(hidden)]
pub use zng_color_proc_macros::hex_color;

pub use zng_layout::unit::{Rgba, RgbaComponent};

pub mod colors;
pub mod filter;
pub mod gradient;
pub mod web_colors;

mod mix;
pub use mix::*;

/// Hexadecimal color literal.
///
/// # Syntax
///
/// `[#|0x]RRGGBB[AA]` or `[#|0x]RGB[A]`.
///
/// An optional prefix `#` or `0x` is supported, after the prefix a hexadecimal integer literal is expected. The literal can be
/// separated using `_`. No integer type suffix is allowed.
///
/// The literal is a sequence of 3 or 4 bytes (red, green, blue and alpha). If the sequence is in pairs each pair is a byte `[00..=FF]`.
/// If the sequence is in single characters this is a shorthand that repeats the character for each byte, e.g. `#012F` equals `#001122FF`.
///
/// # Examples
///
/// ```
/// # use zng_color::hex;
/// let red = hex!(#FF0000);
/// let green = hex!(#00FF00);
/// let blue = hex!(#0000FF);
/// let red_half_transparent = hex!(#FF00007F);
///
/// assert_eq!(red, hex!(#F00));
/// assert_eq!(red, hex!(0xFF_00_00));
/// assert_eq!(red, hex!(FF_00_00));
/// ```
///
#[macro_export]
macro_rules! hex {
    ($($tt:tt)+) => {
        $crate::hex_color!{$crate, $($tt)*}
    };
}

fn lerp_rgba_linear(mut from: Rgba, to: Rgba, factor: Factor) -> Rgba {
    from.red = from.red.lerp(&to.red, factor);
    from.green = from.green.lerp(&to.green, factor);
    from.blue = from.blue.lerp(&to.blue, factor);
    from.alpha = from.alpha.lerp(&to.alpha, factor);
    from
}

/// Default implementation of lerp for [`Rgba`] in apps.
///
/// Implements [`lerp_space`] dependent transition.
///
/// Apps set this as the default implementation on init.
pub fn lerp_rgba(from: Rgba, to: Rgba, factor: Factor) -> Rgba {
    match lerp_space() {
        LerpSpace::HslaChromatic => Hsla::from(from).slerp_chromatic(to.into(), factor).into(),
        LerpSpace::Rgba => lerp_rgba_linear(from, to, factor),
        LerpSpace::Hsla => Hsla::from(from).slerp(to.into(), factor).into(),
        LerpSpace::HslaLinear => Hsla::from(from).lerp_hsla(to.into(), factor).into(),
    }
}

/// Pre-multiplied RGB + alpha.
///
/// Use from/into conversion to create.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct PreMulRgba {
    /// [`Rgba::red`] multiplied by `alpha`.
    pub red: f32,
    /// [`Rgba::green`] multiplied by `alpha`.
    pub green: f32,
    /// [`Rgba::blue`] multiplied by `alpha`.
    pub blue: f32,
    /// Alpha channel value, in the `[0.0..=1.0]` range.
    pub alpha: f32,
}
impl PartialEq for PreMulRgba {
    fn eq(&self, other: &Self) -> bool {
        about_eq(self.red, other.red, EQ_GRANULARITY)
            && about_eq(self.green, other.green, EQ_GRANULARITY)
            && about_eq(self.blue, other.blue, EQ_GRANULARITY)
            && about_eq(self.alpha, other.alpha, EQ_GRANULARITY)
    }
}
impl Eq for PreMulRgba { }
impl std::hash::Hash for PreMulRgba {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        about_eq_hash(self.red, EQ_GRANULARITY, state);
        about_eq_hash(self.green, EQ_GRANULARITY, state);
        about_eq_hash(self.blue, EQ_GRANULARITY, state);
        about_eq_hash(self.alpha, EQ_GRANULARITY, state);
    }
}

impl_from_and_into_var! {
    fn from(c: Rgba) -> PreMulRgba {
        PreMulRgba {
            red: c.red * c.alpha,
            green: c.green * c.alpha,
            blue: c.blue * c.alpha,
            alpha: c.alpha,
        }
    }

    fn from(c: PreMulRgba) -> Rgba {
        Rgba {
            red: c.red / c.alpha,
            green: c.green / c.alpha,
            blue: c.blue / c.alpha,
            alpha: c.alpha,
        }
    }

    fn from(c: Hsla) -> PreMulRgba {
        Rgba::from(c).into()
    }

    fn from(c: PreMulRgba) -> Hsla {
        Rgba::from(c).into()
    }

    fn from(c: Hsva) -> PreMulRgba {
        Rgba::from(c).into()
    }

    fn from(c: PreMulRgba) -> Hsva {
        Rgba::from(c).into()
    }
}

/// HSL + alpha.
///
/// # Equality
///
/// Equality is determined using [`about_eq`] with `0.001` granularity for [`hue`](Hsla::hue)
/// and `0.00001` granularity for the others.
///
/// [`about_eq`]: zng_layout::unit::about_eq
#[derive(Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct Hsla {
    /// Hue color angle in the `[0.0..=360.0]` range.
    pub hue: f32,
    /// Saturation amount in the `[0.0..=1.0]` range, zero is gray, one is full color.
    pub saturation: f32,
    /// Lightness amount in the `[0.0..=1.0]` range, zero is black, one is white.
    pub lightness: f32,
    /// Alpha channel in the `[0.0..=1.0]` range, zero is invisible, one is opaque.
    pub alpha: f32,
}
impl PartialEq for Hsla {
    fn eq(&self, other: &Self) -> bool {
        about_eq(self.hue, other.hue, EQ_GRANULARITY_100)
            && about_eq(self.saturation, other.saturation, EQ_GRANULARITY)
            && about_eq(self.lightness, other.lightness, EQ_GRANULARITY)
            && about_eq(self.alpha, other.alpha, EQ_GRANULARITY)
    }
}
impl Eq for Hsla { }
impl std::hash::Hash for Hsla {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        about_eq_hash(self.hue, EQ_GRANULARITY_100, state);
        about_eq_hash(self.saturation, EQ_GRANULARITY, state);
        about_eq_hash(self.lightness, EQ_GRANULARITY, state);
        about_eq_hash(self.alpha, EQ_GRANULARITY, state);
    }
}
impl Hsla {
    /// Sets the [`hue`](Self::hue) color angle.
    ///
    /// The value is normalized to be in the `[0.0..=360.0]` range, that is `362.deg()` becomes `2.0`.
    pub fn set_hue<H: Into<AngleDegree>>(&mut self, hue: H) {
        self.hue = hue.into().modulo().0
    }

    /// Sets the [`lightness`](Self::lightness) value.
    pub fn set_lightness<L: Into<Factor>>(&mut self, lightness: L) {
        self.lightness = lightness.into().0;
    }

    /// Sets the [`saturation`](Self::saturation) value.
    pub fn set_saturation<S: Into<Factor>>(&mut self, saturation: S) {
        self.saturation = saturation.into().0;
    }

    /// Sets the [`alpha`](Self::alpha) value.
    pub fn set_alpha<A: Into<Factor>>(&mut self, alpha: A) {
        self.alpha = alpha.into().0
    }

    /// Returns a copy of this color with a new `hue`.
    pub fn with_hue<H: Into<AngleDegree>>(mut self, hue: H) -> Self {
        self.set_hue(hue);
        self
    }

    /// Returns a copy of this color with a new `lightness`.
    pub fn with_lightness<L: Into<Factor>>(mut self, lightness: L) -> Self {
        self.set_lightness(lightness);
        self
    }

    /// Returns a copy of this color with a new `saturation`.
    pub fn with_saturation<S: Into<Factor>>(mut self, saturation: S) -> Self {
        self.set_saturation(saturation);
        self
    }

    /// Returns a copy of this color with a new `alpha`.
    pub fn with_alpha<A: Into<Factor>>(mut self, alpha: A) -> Self {
        self.set_alpha(alpha);
        self
    }

    fn lerp_sla(mut self, to: Hsla, factor: Factor) -> Self {
        self.saturation = self.saturation.lerp(&to.saturation, factor);
        self.lightness = self.lightness.lerp(&to.lightness, factor);
        self.alpha = self.alpha.lerp(&to.alpha, factor);
        self
    }

    /// Interpolate in the [`LerpSpace::Hsla`] mode.
    pub fn slerp(mut self, to: Self, factor: Factor) -> Self {
        self = self.lerp_sla(to, factor);
        self.hue = AngleDegree(self.hue).slerp(AngleDegree(to.hue), factor).0;
        self
    }

    /// If the saturation is more than zero.
    ///
    /// If `false` the color is achromatic, the hue value does not affect the color.
    pub fn is_chromatic(self) -> bool {
        self.saturation > 0.0001
    }

    /// Interpolate in the [`LerpSpace::HslaChromatic`] mode.
    pub fn slerp_chromatic(mut self, to: Self, factor: Factor) -> Self {
        if self.is_chromatic() && to.is_chromatic() {
            self.slerp(to, factor)
        } else {
            self = self.lerp_sla(to, factor);
            if to.is_chromatic() {
                self.hue = to.hue;
            }
            self
        }
    }

    fn lerp_hsla(mut self, to: Self, factor: Factor) -> Self {
        self = self.lerp_sla(to, factor);
        self.hue = self.hue.lerp(&to.hue, factor);
        self
    }

    fn lerp(self, to: Self, factor: Factor) -> Self {
        match lerp_space() {
            LerpSpace::HslaChromatic => self.slerp_chromatic(to, factor),
            LerpSpace::Rgba => lerp_rgba_linear(self.into(), to.into(), factor).into(),
            LerpSpace::Hsla => self.slerp(to, factor),
            LerpSpace::HslaLinear => self.lerp_hsla(to, factor),
        }
    }
}
impl fmt::Debug for Hsla {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("Hsla")
                .field("hue", &self.hue)
                .field("saturation", &self.saturation)
                .field("lightness", &self.lightness)
                .field("alpha", &self.alpha)
                .finish()
        } else {
            fn p(n: f32) -> f32 {
                clamp_normal(n) * 100.0
            }
            let a = p(self.alpha);
            let h = AngleDegree(self.hue).modulo().0.round();
            if (a - 100.0).abs() <= EQ_GRANULARITY {
                write!(f, "hsl({h}.deg(), {}.pct(), {}.pct())", p(self.saturation), p(self.lightness))
            } else {
                write!(
                    f,
                    "hsla({h}.deg(), {}.pct(), {}.pct(), {}.pct())",
                    p(self.saturation),
                    p(self.lightness),
                    a
                )
            }
        }
    }
}
impl fmt::Display for Hsla {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn p(n: f32) -> f32 {
            clamp_normal(n) * 100.0
        }
        let a = p(self.alpha);
        let h = AngleDegree(self.hue).modulo().0.round();
        if (a - 100.0).abs() <= EQ_GRANULARITY {
            write!(f, "hsl({h}º, {}%, {}%)", p(self.saturation), p(self.lightness))
        } else {
            write!(f, "hsla({h}º, {}%, {}%, {}%)", p(self.saturation), p(self.lightness), a)
        }
    }
}
impl Transitionable for Hsla {
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        self.lerp(*to, step)
    }
}

/// HSV + alpha
///
/// # Equality
///
/// Equality is determined using [`about_eq`] with `0.001` granularity for [`hue`](Hsva::hue)
/// and `0.00001` granularity for the others.
///
/// [`about_eq`]: zng_layout::unit::about_eq
#[derive(Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct Hsva {
    /// Hue color angle in the `[0.0..=360.0]` range.
    pub hue: f32,
    /// Saturation amount in the `[0.0..=1.0]` range, zero is gray, one is full color.
    pub saturation: f32,
    /// Brightness amount in the `[0.0..=1.0]` range, zero is black, one is white.
    pub value: f32,
    /// Alpha channel in the `[0.0..=1.0]` range, zero is invisible, one is opaque.
    pub alpha: f32,
}
impl PartialEq for Hsva {
    fn eq(&self, other: &Self) -> bool {
        about_eq(self.hue, other.hue, EQ_GRANULARITY_100)
            && about_eq(self.saturation, other.saturation, EQ_GRANULARITY)
            && about_eq(self.value, other.value, EQ_GRANULARITY)
            && about_eq(self.alpha, other.alpha, EQ_GRANULARITY)
    }
}
impl Eq for Hsva { }
impl std::hash::Hash for Hsva {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        about_eq_hash(self.hue, EQ_GRANULARITY_100, state);
        about_eq_hash(self.saturation, EQ_GRANULARITY, state);
        about_eq_hash(self.value, EQ_GRANULARITY, state);
        about_eq_hash(self.alpha, EQ_GRANULARITY, state);
    }
}
impl Hsva {
    /// Sets the [`hue`](Self::hue) color angle.
    ///
    /// The value is normalized to be in the `[0.0..=360.0]` range, that is `362.deg()` becomes `2.0`.
    pub fn set_hue<H: Into<AngleDegree>>(&mut self, hue: H) {
        self.hue = hue.into().modulo().0
    }

    /// Sets the [`value`](Self::value).
    pub fn set_value<L: Into<Factor>>(&mut self, value: L) {
        self.value = value.into().0;
    }

    /// Sets the [`saturation`](Self::saturation) value.
    pub fn set_saturation<L: Into<Factor>>(&mut self, saturation: L) {
        self.saturation = saturation.into().0;
    }

    /// Sets the [`alpha`](Self::alpha) value.
    pub fn set_alpha<A: Into<Factor>>(&mut self, alpha: A) {
        self.alpha = alpha.into().0
    }

    /// Returns a copy of this color with a new `hue`.
    pub fn with_hue<H: Into<AngleDegree>>(mut self, hue: H) -> Self {
        self.set_hue(hue);
        self
    }

    /// Returns a copy of this color with a new `value`.
    pub fn with_value<V: Into<Factor>>(mut self, value: V) -> Self {
        self.set_value(value);
        self
    }

    /// Returns a copy of this color with a new `saturation`.
    pub fn with_saturation<S: Into<Factor>>(mut self, saturation: S) -> Self {
        self.set_saturation(saturation);
        self
    }

    /// Returns a copy of this color with a new `alpha`.
    pub fn with_alpha<A: Into<Factor>>(mut self, alpha: A) -> Self {
        self.set_alpha(alpha);
        self
    }
}
impl fmt::Debug for Hsva {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("Hsla")
                .field("hue", &self.hue)
                .field("saturation", &self.saturation)
                .field("value", &self.value)
                .field("alpha", &self.alpha)
                .finish()
        } else {
            fn p(n: f32) -> f32 {
                clamp_normal(n) * 100.0
            }
            let a = p(self.alpha);
            let h = AngleDegree(self.hue).modulo().0.round();
            if (a - 100.0).abs() <= EQ_GRANULARITY {
                write!(f, "hsv({h}.deg(), {}.pct(), {}.pct())", p(self.saturation), p(self.value))
            } else {
                write!(
                    f,
                    "hsva({h}.deg(), {}.pct(), {}.pct(), {}.pct())",
                    p(self.saturation),
                    p(self.value),
                    a
                )
            }
        }
    }
}
impl fmt::Display for Hsva {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn p(n: f32) -> f32 {
            clamp_normal(n) * 100.0
        }
        let a = p(self.alpha);
        let h = AngleDegree(self.hue).modulo().0.round();
        if (a - 100.0).abs() <= EQ_GRANULARITY {
            write!(f, "hsv({h}º, {}%, {}%)", p(self.saturation), p(self.value))
        } else {
            write!(f, "hsva({h}º, {}%, {}%, {}%)", p(self.saturation), p(self.value), a)
        }
    }
}
impl_from_and_into_var! {
    fn from(hsla: Hsla) -> Hsva {
        let lightness = clamp_normal(hsla.lightness);
        let saturation = clamp_normal(hsla.saturation);

        let value = lightness + saturation * lightness.min(1.0 - lightness);
        let saturation = if value <= EQ_GRANULARITY {
            0.0
        } else {
            2.0 * (1.0 - lightness / value)
        };

        Hsva {
            hue: hsla.hue,
            saturation,
            value,
            alpha: hsla.alpha,
        }
    }

    fn from(hsva: Hsva) -> Hsla {
        let saturation = clamp_normal(hsva.saturation);
        let value = clamp_normal(hsva.value);

        let lightness = value * (1.0 - saturation / 2.0);
        let saturation = if lightness <= EQ_GRANULARITY || lightness >= 1.0 - EQ_GRANULARITY {
            0.0
        } else {
            2.0 * (1.0 * lightness / value)
        };

        Hsla {
            hue: hsva.hue,
            saturation,
            lightness,
            alpha: hsva.alpha,
        }
    }

    fn from(hsva: Hsva) -> Rgba {
        let hue = AngleDegree(hsva.hue).modulo().0;
        let saturation = clamp_normal(hsva.saturation);
        let value = clamp_normal(hsva.value);

        let c = value * saturation;
        let hue = hue / 60.0;
        let x = c * (1.0 - (hue.rem_euclid(2.0) - 1.0).abs());

        let (red, green, blue) = if hue <= 1.0 {
            (c, x, 0.0)
        } else if hue <= 2.0 {
            (x, c, 0.0)
        } else if hue <= 3.0 {
            (0.0, c, x)
        } else if hue <= 4.0 {
            (0.0, x, c)
        } else if hue <= 5.0 {
            (x, 0.0, c)
        } else if hue <= 6.0 {
            (c, 0.0, x)
        } else {
            (0.0, 0.0, 0.0)
        };

        let m = value - c;

        let f = |n: f32| ((n + m) * 255.0).round() / 255.0;

        Rgba {
            red: f(red),
            green: f(green),
            blue: f(blue),
            alpha: hsva.alpha,
        }
    }

    fn from(hsla: Hsla) -> Rgba {
        if hsla.saturation <= EQ_GRANULARITY {
            return rgba(hsla.lightness, hsla.lightness, hsla.lightness, hsla.alpha);
        }

        let hue = AngleDegree(hsla.hue).modulo().0;
        let saturation = clamp_normal(hsla.saturation);
        let lightness = clamp_normal(hsla.lightness);

        let c = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
        let hp = hue / 60.0;
        let x = c * (1.0 - ((hp % 2.0) - 1.0).abs());
        let (red, green, blue) = if hp <= 1.0 {
            (c, x, 0.0)
        } else if hp <= 2.0 {
            (x, c, 0.0)
        } else if hp <= 3.0 {
            (0.0, c, x)
        } else if hp <= 4.0 {
            (0.0, x, c)
        } else if hp <= 5.0 {
            (x, 0.0, c)
        } else if hp <= 6.0 {
            (c, 0.0, x)
        } else {
            (0.0, 0.0, 0.0)
        };
        let m = lightness - c * 0.5;

        let f = |i: f32| ((i + m) * 255.0).round() / 255.0;

        Rgba {
            red: f(red),
            green: f(green),
            blue: f(blue),
            alpha: hsla.alpha,
        }
    }
}
impl Transitionable for Hsva {
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        match lerp_space() {
            LerpSpace::HslaChromatic => Hsla::from(self).slerp_chromatic((*to).into(), step).into(),
            LerpSpace::Rgba => lerp_rgba_linear(self.into(), (*to).into(), step).into(),
            LerpSpace::Hsla => Hsla::from(self).slerp((*to).into(), step).into(),
            LerpSpace::HslaLinear => Hsla::from(self).lerp_hsla((*to).into(), step).into(),
        }
    }
}

macro_rules! cylindrical_color {
    ($rgba:ident -> ($min:ident, $max:ident, $delta:ident, $hue:ident)) => {
        fn sanitize(i: f32) -> f32 {
            clamp_normal((i * 255.0).round() / 255.0)
        }

        let r = sanitize($rgba.red);
        let g = sanitize($rgba.green);
        let b = sanitize($rgba.blue);

        let $min = r.min(g).min(b);
        let $max = r.max(g).max(b);

        fn about_eq(a: f32, b: f32) -> bool {
            (a - b) <= EQ_GRANULARITY
        }

        let $delta = $max - $min;

        let $hue = if $delta <= EQ_GRANULARITY {
            0.0
        } else {
            60.0 * if about_eq($max, r) {
                ((g - b) / $delta).rem_euclid(6.0)
            } else if about_eq($max, g) {
                (b - r) / $delta + 2.0
            } else {
                debug_assert!(about_eq($max, b));
                (r - g) / $delta + 4.0
            }
        };
    };
}

impl_from_and_into_var! {
    fn from(rgba: Rgba) -> Hsva {
        cylindrical_color!(rgba -> (min, max, delta, hue));

        let saturation = if max <= EQ_GRANULARITY { 0.0 } else { delta / max };

        let value = max;

        Hsva {
            hue,
            saturation,
            value,
            alpha: rgba.alpha,
        }
    }

    fn from(rgba: Rgba) -> Hsla {
        cylindrical_color!(rgba -> (min, max, delta, hue));

        let lightness = (max + min) / 2.0;

        let saturation = if delta <= EQ_GRANULARITY {
            0.0
        } else {
            delta / (1.0 - (2.0 * lightness - 1.0).abs())
        };

        Hsla {
            hue,
            lightness,
            saturation,
            alpha: rgba.alpha,
        }
    }
}

// Util
fn clamp_normal(i: f32) -> f32 {
    i.clamp(0.0, 1.0)
}

/// RGB color, opaque, alpha is set to `1.0`.
///
/// # Arguments
///
/// The arguments can either be [`f32`] in the `0.0..=1.0` range or
/// [`u8`] in the `0..=255` range or a [percentage](zng_layout::unit::FactorPercent).
///
/// # Examples
///
/// ```
/// use zng_color::rgb;
///
/// let red = rgb(1.0, 0.0, 0.0);
/// let green = rgb(0, 255, 0);
/// ```
pub fn rgb<C: Into<RgbaComponent>>(red: C, green: C, blue: C) -> Rgba {
    rgba(red, green, blue, 1.0)
}

/// RGBA color.
///
/// # Arguments
///
/// The arguments can either be `f32` in the `0.0..=1.0` range or
/// `u8` in the `0..=255` range or a [percentage](zng_layout::unit::FactorPercent).
///
/// The rgb arguments must be of the same type, the alpha argument can be of a different type.
///
/// # Examples
///
/// ```
/// use zng_color::rgba;
///
/// let half_red = rgba(255, 0, 0, 0.5);
/// let green = rgba(0.0, 1.0, 0.0, 1.0);
/// let transparent = rgba(0, 0, 0, 0);
/// ```
pub fn rgba<C: Into<RgbaComponent>, A: Into<RgbaComponent>>(red: C, green: C, blue: C, alpha: A) -> Rgba {
    Rgba {
        red: red.into().0,
        green: green.into().0,
        blue: blue.into().0,
        alpha: alpha.into().0,
    }
}

/// HSL color, opaque, alpha is set to `1.0`.
///
/// # Arguments
///
/// The first argument `hue` can be any [angle unit]. The other two arguments can be [`f32`] in the `0.0..=1.0`
/// range or a [percentage](zng_layout::unit::FactorPercent).
///
/// The `saturation` and `lightness` arguments must be of the same type.
///
/// # Examples
///
/// ```
/// use zng_color::hsl;
/// use zng_layout::unit::*;
///
/// let red = hsl(0.deg(), 100.pct(), 50.pct());
/// let green = hsl(115.deg(), 1.0, 0.5);
/// ```
///
/// [angle unit]: trait@zng_layout::unit::AngleUnits
pub fn hsl<H: Into<AngleDegree>, N: Into<Factor>>(hue: H, saturation: N, lightness: N) -> Hsla {
    hsla(hue, saturation, lightness, 1.0)
}

/// HSLA color.
///
/// # Arguments
///
/// The first argument `hue` can be any [angle unit]. The other two arguments can be [`f32`] in the `0.0..=1.0`
/// range or a [percentage](zng_layout::unit::FactorPercent).
///
/// The `saturation` and `lightness` arguments must be of the same type.
///
/// # Examples
///
/// ```
/// use zng_color::hsla;
/// use zng_layout::unit::*;
///
/// let red = hsla(0.deg(), 100.pct(), 50.pct(), 1.0);
/// let green = hsla(115.deg(), 1.0, 0.5, 100.pct());
/// let transparent = hsla(0.deg(), 1.0, 0.5, 0.0);
/// ```
///
/// [angle unit]: trait@zng_layout::unit::AngleUnits
pub fn hsla<H: Into<AngleDegree>, N: Into<Factor>, A: Into<Factor>>(hue: H, saturation: N, lightness: N, alpha: A) -> Hsla {
    Hsla {
        hue: hue.into().0,
        saturation: saturation.into().0,
        lightness: lightness.into().0,
        alpha: alpha.into().0,
    }
}

/// HSV color, opaque, alpha is set to `1.0`.
///
/// # Arguments
///
/// The first argument `hue` can be any [angle unit]. The other two arguments can be [`f32`] in the `0.0..=1.0`
/// range or a [percentage](zng_layout::unit::FactorPercent).
///
/// The `saturation` and `value` arguments must be of the same type.
///
/// # Examples
///
/// ```
/// use zng_color::hsv;
/// use zng_layout::unit::*;
///
/// let red = hsv(0.deg(), 100.pct(), 50.pct());
/// let green = hsv(115.deg(), 1.0, 0.5);
/// ```
///
/// [angle unit]: trait@zng_layout::unit::AngleUnits
pub fn hsv<H: Into<AngleDegree>, N: Into<Factor>>(hue: H, saturation: N, value: N) -> Hsva {
    hsva(hue, saturation, value, 1.0)
}

/// HSVA color.
///
/// # Arguments
///
/// The first argument `hue` can be any [angle unit]. The other two arguments can be [`f32`] in the `0.0..=1.0`
/// range or a [percentage](zng_layout::unit::FactorPercent).
///
/// The `saturation` and `value` arguments must be of the same type.
///
/// # Examples
///
/// ```
/// use zng_color::hsva;
/// use zng_layout::unit::*;
///
/// let red = hsva(0.deg(), 100.pct(), 50.pct(), 1.0);
/// let green = hsva(115.deg(), 1.0, 0.5, 100.pct());
/// let transparent = hsva(0.deg(), 1.0, 0.5, 0.0);
/// ```
///
/// [angle unit]: trait@zng_layout::unit::AngleUnits
pub fn hsva<H: Into<AngleDegree>, N: Into<Factor>, A: Into<Factor>>(hue: H, saturation: N, value: N, alpha: A) -> Hsva {
    Hsva {
        hue: hue.into().0,
        saturation: saturation.into().0,
        value: value.into().0,
        alpha: alpha.into().0,
    }
}

context_var! {
    /// Defines the preferred color scheme in a context.
    pub static COLOR_SCHEME_VAR: ColorScheme = ColorScheme::default();
}

/// RGBA color pair.
///
/// # Arguments
///
/// The arguments can be any color type that converts to [`Rgba`]. The first color
/// is used in [`ColorScheme::Dark`] contexts, the second color is used in [`ColorScheme::Light`] contexts.
///
/// Note that [`LightDark`] converts `IntoVar<Rgba>` with a contextual var that selects the color, so you
/// can just set color properties directly with a color pair .
pub fn light_dark(light: impl Into<Rgba>, dark: impl Into<Rgba>) -> LightDark {
    LightDark {
        light: light.into(),
        dark: dark.into(),
    }
}

/// Represents a dark and light *color*.
///
///
/// Note that this struct converts `IntoVar<Rgba>` with a contextual var that selects the color, so you
/// can just set color properties directly with a color pair.
#[derive(Debug, Clone, Copy, PartialEq, Hash, serde::Serialize, serde::Deserialize, Transitionable)]
pub struct LightDark {
    /// Color used when [`ColorScheme::Dark`].
    pub dark: Rgba,
    /// Color used when [`ColorScheme::Light`].
    pub light: Rgba,
}
impl_from_and_into_var! {
    /// From `(light, dark)` tuple.
    fn from<L: Into<Rgba>, D: Into<Rgba>>((light, dark): (L, D)) -> LightDark {
        LightDark {
            light: light.into(),
            dark: dark.into(),
        }
    }

    /// From same color to both.
    fn from(color: Rgba) -> LightDark {
        LightDark { dark: color, light: color }
    }

    /// From same color to both.
    fn from(color: Hsva) -> LightDark {
        Rgba::from(color).into()
    }

    /// From same color to both.
    fn from(color: Hsla) -> LightDark {
        Rgba::from(color).into()
    }

    fn from(color: LightDark) -> Option<LightDark>;
}
impl IntoVar<Rgba> for LightDark {
    type Var = ContextualizedVar<Rgba>;

    fn into_var(self) -> Self::Var {
        COLOR_SCHEME_VAR.map(move |s| match s {
            ColorScheme::Light => self.light,
            ColorScheme::Dark => self.dark,
            _ => self.light,
        })
    }
}
impl LightDark {
    /// New from light, dark colors.
    pub fn new(light: impl Into<Rgba>, dark: impl Into<Rgba>) -> Self {
        Self {
            light: light.into(),
            dark: dark.into(),
        }
    }

    /// Overlay WHITE/BLACK to the dark/light color depending on the `factor`, negative factor inverts overlay.
    pub fn shade_fct(mut self, factor: impl Into<Factor>) -> Self {
        let mut factor = factor.into();
        let (dark_overlay, light_overlay) = if factor > 0.fct() {
            (colors::WHITE, colors::BLACK)
        } else {
            factor = factor.abs();
            (colors::BLACK, colors::WHITE)
        };
        self.dark = dark_overlay.with_alpha(factor).mix_normal(self.dark);
        self.light = light_overlay.with_alpha(factor).mix_normal(self.light);
        self
    }

    /// Shade at 8% increments.
    ///
    /// Common usage: 1=hovered, 2=pressed.
    pub fn shade(self, shade: i8) -> Self {
        self.shade_fct(shade as f32 * 0.08)
    }

    /// Gets a contextual `Rgba` var that selects the color for the context scheme.
    ///
    /// Also see [`LightDarkVarExt`] for mapping from vars.
    pub fn rgba(self) -> ContextualizedVar<Rgba> {
        IntoVar::<Rgba>::into_var(self)
    }

    /// Gets a contextual `Rgba` var that selects the color for the context scheme and `map` it.
    pub fn rgba_map<T: VarValue>(self, mut map: impl FnMut(Rgba) -> T + Send + 'static) -> impl Var<T> {
        COLOR_SCHEME_VAR.map(move |s| match s {
            ColorScheme::Light => map(self.light),
            ColorScheme::Dark => map(self.dark),
            _ => map(self.light),
        })
    }

    /// Gets a contextual `Rgba` var that selects the color for the context scheme and converts it to `T`.
    pub fn rgba_into<T: VarValue + From<Rgba>>(self) -> impl Var<T> {
        self.rgba_map(T::from)
    }
}
impl ops::Index<ColorScheme> for LightDark {
    type Output = Rgba;

    fn index(&self, index: ColorScheme) -> &Self::Output {
        match index {
            ColorScheme::Light => &self.light,
            ColorScheme::Dark => &self.dark,
            _ => &self.light,
        }
    }
}
impl ops::IndexMut<ColorScheme> for LightDark {
    fn index_mut(&mut self, index: ColorScheme) -> &mut Self::Output {
        match index {
            ColorScheme::Light => &mut self.light,
            ColorScheme::Dark => &mut self.dark,
            _ => &mut self.light,
        }
    }
}

/// Extension methods for `impl Var<LightDark>`.
pub trait LightDarkVarExt {
    /// Gets a contextualized var that maps to [`LightDark::rgba`].
    fn rgba(&self) -> impl Var<Rgba>;
    /// Gets a contextualized var that maps to [`LightDark::rgba`] and `map`.
    fn rgba_map<T: VarValue>(&self, map: impl FnMut(Rgba) -> T + Send + 'static) -> impl Var<T>;
    /// Gets a contextualized var that maps to [`LightDark::rgba`] converted into `T`.
    fn rgba_into<T: VarValue + From<Rgba>>(&self) -> impl Var<T>;

    /// Gets a contextualized var that maps using `map` and then to [`LightDark::rgba`].
    fn map_rgba(&self, map: impl FnMut(LightDark) -> LightDark + Send + 'static) -> impl Var<Rgba>;
    /// Gets a contextualized var that maps using `map` and then into `T`.
    fn map_rgba_into<T: VarValue + From<Rgba>>(&self, map: impl FnMut(LightDark) -> LightDark + Send + 'static) -> impl Var<T>;

    /// Gets a contextualized var that maps to [`LightDark::shade_fct`] and then to [`LightDark::rgba`].
    fn shade_fct(&self, fct: impl Into<Factor>) -> impl Var<Rgba>;
    /// Gets a contextualized var that maps to [`LightDark::shade_fct`] and then to [`LightDark::rgba`] and then into `T`.
    fn shade_fct_into<T: VarValue + From<Rgba>>(&self, fct: impl Into<Factor>) -> impl Var<T>;

    /// Gets a contextualized var that maps to [`LightDark::shade`] and then to [`LightDark::rgba`].   
    ///
    /// * +1 - Hovered.
    /// * +2 - Pressed.
    fn shade(&self, shade: i8) -> impl Var<Rgba>;
    /// Gets a contextualized var that maps to [`LightDark::shade`] and then to [`LightDark::rgba`] and then into `T`.
    fn shade_into<T: VarValue + From<Rgba>>(&self, shade: i8) -> impl Var<T>;
}
impl<V: Var<LightDark>> LightDarkVarExt for V {
    fn rgba(&self) -> impl Var<Rgba> {
        expr_var! {
            let c = #{self.clone()};
            match *#{COLOR_SCHEME_VAR} {
                ColorScheme::Light => c.light,
                ColorScheme::Dark => c.dark,
                _ => c.light,
            }
        }
    }

    fn rgba_map<T: VarValue>(&self, mut map: impl FnMut(Rgba) -> T + Send + 'static) -> impl Var<T> {
        expr_var! {
            let c = #{self.clone()};
            match *#{COLOR_SCHEME_VAR} {
                ColorScheme::Light => map(c.light),
                ColorScheme::Dark => map(c.dark),
                _ => map(c.light),
            }
        }
    }

    fn rgba_into<T: VarValue + From<Rgba>>(&self) -> impl Var<T> {
        self.rgba_map(Into::into)
    }

    fn map_rgba(&self, mut map: impl FnMut(LightDark) -> LightDark + Send + 'static) -> impl Var<Rgba> {
        expr_var! {
            let c = map(*#{self.clone()});
            match *#{COLOR_SCHEME_VAR} {
                ColorScheme::Light => c.light,
                ColorScheme::Dark => c.dark,
                _ => c.light,
            }
        }
    }

    fn map_rgba_into<T: VarValue + From<Rgba>>(&self, mut map: impl FnMut(LightDark) -> LightDark + Send + 'static) -> impl Var<T> {
        expr_var! {
            let c = map(*#{self.clone()});
            match *#{COLOR_SCHEME_VAR} {
                ColorScheme::Light => T::from(c.light),
                ColorScheme::Dark => T::from(c.dark),
                _ => T::from(c.light),
            }
        }
    }

    fn shade_fct(&self, fct: impl Into<Factor>) -> impl Var<Rgba> {
        let fct = fct.into();
        expr_var! {
            let c = #{self.clone()}.shade_fct(fct);
            match *#{COLOR_SCHEME_VAR} {
                ColorScheme::Light => c.light,
                ColorScheme::Dark => c.dark,
                _ => c.light,
            }
        }
    }

    fn shade(&self, shade: i8) -> impl Var<Rgba> {
        expr_var! {
            let c = #{self.clone()}.shade(shade);
            match *#{COLOR_SCHEME_VAR} {
                ColorScheme::Light => c.light,
                ColorScheme::Dark => c.dark,
                _ => c.light,
            }
        }
    }

    fn shade_fct_into<T: VarValue + From<Rgba>>(&self, fct: impl Into<Factor>) -> impl Var<T> {
        let fct = fct.into();
        expr_var! {
            let c = #{self.clone()}.shade_fct(fct);
            match *#{COLOR_SCHEME_VAR} {
                ColorScheme::Light => T::from(c.light),
                ColorScheme::Dark => T::from(c.dark),
                _ => T::from(c.light),
            }
        }
    }

    fn shade_into<T: VarValue + From<Rgba>>(&self, shade: i8) -> impl Var<T> {
        expr_var! {
            let c = #{self.clone()}.shade(shade);
            match *#{COLOR_SCHEME_VAR} {
                ColorScheme::Light => T::from(c.light),
                ColorScheme::Dark => T::from(c.dark),
                _ => T::from(c.light),
            }
        }
    }
}

/// Defines the color space for color interpolation.
///
/// See [`with_lerp_space`] for more details.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LerpSpace {
    /// Linear interpolation in each RGBA component.
    Rgba,
    /// Spherical linear interpolation in Hue (shorter path), linear interpolation in SLA.
    Hsla,
    /// Linear interpolate SLA, spherical linear interpolation in Hue (short) if both colors are chromatic (S>0) or
    /// jumps to the chromatic hue from the start.
    #[default]
    HslaChromatic,
    /// Linear interpolation in each HSLA component.
    HslaLinear,
}

/// Gets the lerp space used for color interpolation.
///
/// Must be called only inside the [`with_lerp_space`] closure or in the lerp implementation of a variable animating
/// with [`rgba_sampler`] or [`hsla_linear_sampler`].
pub fn lerp_space() -> LerpSpace {
    LERP_SPACE.get_clone()
}

/// Calls `f` with [`lerp_space`] set to `space`.
///
/// See [`rgba_sampler`] and [`hsla_linear_sampler`] for a way to set the space in animations.
pub fn with_lerp_space<R>(space: LerpSpace, f: impl FnOnce() -> R) -> R {
    LERP_SPACE.with_context(&mut Some(Arc::new(space)), f)
}

/// Animation sampler that sets the [`lerp_space`] to [`LerpSpace::Rgba`].
///
/// Samplers can be set in animations using the [`Var::easing_with`] method.
///
/// [`Var::easing_with`]: zng_var::Var::easing_with
pub fn rgba_sampler<T: Transitionable>(t: &Transition<T>, step: EasingStep) -> T {
    with_lerp_space(LerpSpace::Rgba, || t.sample(step))
}

/// Animation sampler that sets the [`lerp_space`] to [`LerpSpace::Hsla`].
///
/// Note that this is already the default.
pub fn hsla_sampler<T: Transitionable>(t: &Transition<T>, step: EasingStep) -> T {
    with_lerp_space(LerpSpace::Hsla, || t.sample(step))
}

/// Animation sampler that sets the [`lerp_space`] to [`LerpSpace::HslaLinear`].
///
/// Samplers can be set in animations using the [`Var::easing_with`] method.
///
/// [`Var::easing_with`]: zng_var::Var::easing_with
pub fn hsla_linear_sampler<T: Transitionable>(t: &Transition<T>, step: EasingStep) -> T {
    with_lerp_space(LerpSpace::HslaLinear, || t.sample(step))
}

context_local! {
    static LERP_SPACE: LerpSpace = LerpSpace::default();
}

#[cfg(test)]
mod tests {
    use super::*;
    use zng_layout::unit::AngleUnits as _;

    #[test]
    fn hsl_red() {
        assert_eq!(Rgba::from(hsl(0.0.deg(), 100.pct(), 50.pct())), rgb(1.0, 0.0, 0.0))
    }

    #[test]
    fn hsl_color() {
        assert_eq!(Rgba::from(hsl(91.0.deg(), 1.0, 0.5)), rgb(123, 255, 0))
    }

    #[test]
    fn rgb_to_hsl() {
        let color = rgba(0, 100, 200, 0.2);
        let a = format!("{color:?}");
        let b = format!("{:?}", Rgba::from(Hsla::from(color)));
        assert_eq!(a, b)
    }

    #[test]
    fn rgb_to_hsv() {
        let color = rgba(0, 100, 200, 0.2);
        let a = format!("{color:?}");
        let b = format!("{:?}", Rgba::from(Hsva::from(color)));
        assert_eq!(a, b)
    }

    #[test]
    fn rgba_display() {
        macro_rules! test {
            ($($tt:tt)+) => {
                let expected = stringify!($($tt)+).replace(" ", "");
                let actual = hex!($($tt)+).to_string();
                assert_eq!(expected, actual);
            }
        }

        test!(#AABBCC);
        test!(#123456);
        test!(#000000);
        test!(#FFFFFF);

        test!(#AABBCCDD);
        test!(#12345678);
        test!(#00000000);
        test!(#FFFFFF00);
    }

    #[test]
    fn test_hex_color() {
        fn f(n: u8) -> f32 {
            n as f32 / 255.0
        }
        assert_eq!(Rgba::new(f(0x11), f(0x22), f(0x33), f(0x44)), hex!(0x11223344));

        assert_eq!(colors::BLACK, hex!(0x00_00_00_FF));
        assert_eq!(colors::WHITE, hex!(0xFF_FF_FF_FF));
        assert_eq!(colors::WHITE, hex!(0xFF_FF_FF));
        assert_eq!(colors::WHITE, hex!(0xFFFFFF));
        assert_eq!(colors::WHITE, hex!(#FFFFFF));
        assert_eq!(colors::WHITE, hex!(FFFFFF));
        assert_eq!(colors::WHITE, hex!(0xFFFF));
        assert_eq!(colors::BLACK, hex!(0x000));
        assert_eq!(colors::BLACK, hex!(#000));
        assert_eq!(colors::BLACK, hex!(000));
    }

    // #[test]
    // fn rgb_to_hsv_all() {
    //     for r in 0..=255 {
    //         println!("{r}");
    //         for g in 0..=255 {
    //             for b in 0..=255 {
    //                 let color = rgb(r, g, b);
    //                 let a = color.to_string();
    //                 let b = color.to_hsva().to_rgba().to_string();
    //                 assert_eq!(a, b)
    //             }
    //         }
    //     }
    // }
}
