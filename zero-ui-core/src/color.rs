//! Color types, functions and macros, [`Rgba`], [`filters`], [`hex!`](crate::color::hex) and more.

use crate::{units::*, var::*, widget_instance::UiNode};
use std::{fmt, ops};

pub use crate::app::view_process::ColorScheme;

pub mod colors;
pub mod filters;
mod mix;
pub use mix::*;

///<span data-del-macro-root></span> Hexadecimal color literal.
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
/// # use zero_ui_core::color::hex;
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
        $crate::color::hex_color!{$crate, $($tt)*}
    };
}
#[doc(inline)]
pub use crate::hex;

#[doc(hidden)]
pub use zero_ui_proc_macros::hex_color;

/// Webrender RGBA.
pub type RenderColor = crate::render::webrender_api::ColorF;

/// Minimal difference between values in around the 0.0..=1.0 scale.
const EPSILON: f32 = 0.00001;
/// Minimal difference between values in around the 1.0..=100.0 scale.
const EPSILON_100: f32 = 0.001;

/// RGB + alpha.
///
/// # Equality
///
/// Equality is determined using [`about_eq`] with `0.00001` epsilon.
#[derive(Copy, Clone)]
pub struct Rgba {
    /// Red channel value, in the `[0.0..=1.0]` range.
    pub red: f32,
    /// Green channel value, in the `[0.0..=1.0]` range.
    pub green: f32,
    /// Blue channel value, in the `[0.0..=1.0]` range.
    pub blue: f32,
    /// Alpha channel value, in the `[0.0..=1.0]` range.
    pub alpha: f32,
}
impl PartialEq for Rgba {
    fn eq(&self, other: &Self) -> bool {
        about_eq(self.red, other.red, EPSILON)
            && about_eq(self.green, other.green, EPSILON)
            && about_eq(self.blue, other.blue, EPSILON)
            && about_eq(self.alpha, other.alpha, EPSILON)
    }
}
impl std::hash::Hash for Rgba {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        about_eq_hash(self.red, EPSILON, state);
        about_eq_hash(self.green, EPSILON, state);
        about_eq_hash(self.blue, EPSILON, state);
        about_eq_hash(self.alpha, EPSILON, state);
    }
}
impl Rgba {
    /// See [`rgba`] for a better constructor.
    pub fn new(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self { red, green, blue, alpha }
    }

    /// Set the [`red`](Rgba::red) component from any type that converts to [`RgbaComponent`].
    pub fn set_red<R: Into<RgbaComponent>>(&mut self, red: R) {
        self.red = red.into().0
    }

    /// Set the [`green`](Rgba::green) component from any type that converts to [`RgbaComponent`].
    pub fn set_green<G: Into<RgbaComponent>>(&mut self, green: G) {
        self.green = green.into().0
    }

    /// Set the [`blue`](Rgba::blue) component from any type that converts to [`RgbaComponent`].
    pub fn set_blue<B: Into<RgbaComponent>>(&mut self, blue: B) {
        self.blue = blue.into().0
    }

    /// Set the [`alpha`](Rgba::alpha) component from any type that converts to [`RgbaComponent`].
    pub fn set_alpha<A: Into<RgbaComponent>>(&mut self, alpha: A) {
        self.alpha = alpha.into().0
    }

    /// Returns a copy of the color with a new `red` value.
    pub fn with_red<R: Into<RgbaComponent>>(mut self, red: R) -> Self {
        self.set_red(red);
        self
    }

    /// Returns a copy of the color with a new `green` value.
    pub fn with_green<R: Into<RgbaComponent>>(mut self, green: R) -> Self {
        self.set_green(green);
        self
    }

    /// Returns a copy of the color with a new `blue` value.
    pub fn with_blue<B: Into<RgbaComponent>>(mut self, blue: B) -> Self {
        self.set_blue(blue);
        self
    }

    /// Returns a copy of the color with a new `alpha` value.
    pub fn with_alpha<A: Into<RgbaComponent>>(mut self, alpha: A) -> Self {
        self.set_alpha(alpha);
        self
    }

    /// Returns a copy of the color with the alpha set to `0`.
    pub fn transparent(self) -> Self {
        self.with_alpha(0.0)
    }

    /// Convert a copy of the color to [`Hsla`].
    pub fn to_hsla(self) -> Hsla {
        self.into()
    }

    /// Convert a copy of the color to [`Hsva`].
    pub fn to_hsva(self) -> Hsva {
        self.into()
    }

    /// Multiply channels by alpha.
    pub fn pre_mul(self) -> PreMulRgba {
        PreMulRgba {
            red: self.red * self.alpha,
            green: self.green * self.alpha,
            blue: self.blue * self.alpha,
            alpha: self.alpha,
        }
    }

    /// Adds the `amount` to the color *lightness*.
    ///
    /// This method converts to [`Hsla`] to lighten and then converts back to `Rgba`.
    ///
    /// # Examples
    ///
    /// Add `10%` of the current lightness to the `DARK_RED` color:
    ///
    /// ```
    /// # use zero_ui_core::color::*;
    /// # use zero_ui_core::units::*;
    /// colors::DARK_RED.lighten(10.pct())
    /// # ;
    /// ```
    pub fn lighten<A: Into<Factor>>(self, amount: A) -> Self {
        self.to_hsla().lighten(amount).to_rgba()
    }

    /// Subtracts the `amount` from the color *lightness*.
    ///
    /// This method converts to [`Hsla`] to darken and then converts back to `Rgba`.
    ///
    /// # Examples
    ///
    /// Removes `10%` of the current lightness from the `DARK_RED` color:
    ///
    /// ```
    /// # use zero_ui_core::color::*;
    /// # use zero_ui_core::units::*;
    /// colors::DARK_RED.darken(10.pct())
    /// # ;
    pub fn darken<A: Into<Factor>>(self, amount: A) -> Self {
        self.to_hsla().darken(amount).to_rgba()
    }

    /// Subtracts the `amount` from the color *saturation*.
    ///
    /// This method converts to [`Hsla`] to desaturate and then converts back to `Rgba`.
    ///
    /// # Examples
    ///
    /// Removes `10%` of the current saturation from the `RED` color:
    ///
    /// ```
    /// # use zero_ui_core::color::*;
    /// # use zero_ui_core::units::*;
    /// colors::RED.desaturate(10.pct())
    /// # ;
    pub fn desaturate<A: Into<Factor>>(self, amount: A) -> Self {
        self.to_hsla().desaturate(amount).to_rgba()
    }

    /// Returns a copy of this color with a new `lightness`.
    ///
    /// This method converts to [`Hsla`] to change the lightness and then converts back to `Rgba`.
    pub fn with_lightness<L: Into<Factor>>(self, lightness: L) -> Self {
        self.to_hsla().with_lightness(lightness).to_rgba()
    }

    /// Convert a copy to [R, G, B, A] bytes.
    pub fn to_bytes(self) -> [u8; 4] {
        [
            (self.red * 255.0) as u8,
            (self.green * 255.0) as u8,
            (self.blue * 255.0) as u8,
            (self.alpha * 255.0) as u8,
        ]
    }
}
impl fmt::Debug for Rgba {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("Rgba")
                .field("red", &self.red)
                .field("green", &self.green)
                .field("blue", &self.blue)
                .field("alpha", &self.alpha)
                .finish()
        } else {
            fn i(n: f32) -> u8 {
                (clamp_normal(n) * 255.0).round() as u8
            }
            let a = i(self.alpha);
            if a == 255 {
                write!(f, "rgb({}, {}, {})", i(self.red), i(self.green), i(self.blue))
            } else {
                write!(f, "rgba({}, {}, {}, {})", i(self.red), i(self.green), i(self.blue), a)
            }
        }
    }
}
impl fmt::Display for Rgba {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn i(n: f32) -> u32 {
            (clamp_normal(n) * 255.0).round() as u32
        }

        let mut rgb: u32 = 0;
        rgb |= i(self.red) << 16;
        rgb |= i(self.green) << 8;
        rgb |= i(self.blue);

        let a = i(self.alpha);
        if a == 255 {
            write!(f, "#{rgb:0>6X}")
        } else {
            let rgba = rgb << 8 | a;
            write!(f, "#{rgba:0>8X}")
        }
    }
}
impl ops::Add<Self> for Rgba {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Rgba {
            red: self.red + rhs.red,
            green: self.green + rhs.green,
            blue: self.blue + rhs.blue,
            alpha: self.alpha + rhs.alpha,
        }
    }
}
impl ops::AddAssign<Self> for Rgba {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}
impl ops::Sub<Self> for Rgba {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Rgba {
            red: self.red - rhs.red,
            green: self.green - rhs.green,
            blue: self.blue - rhs.blue,
            alpha: self.alpha - rhs.alpha,
        }
    }
}
impl ops::SubAssign<Self> for Rgba {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}
impl ops::Mul<Factor> for Rgba {
    type Output = Self;

    fn mul(self, rhs: Factor) -> Self::Output {
        Rgba {
            red: self.red * rhs,
            green: self.green * rhs,
            blue: self.blue * rhs,
            alpha: self.alpha * rhs,
        }
    }
}
impl ops::MulAssign<Factor> for Rgba {
    fn mul_assign(&mut self, rhs: Factor) {
        *self = *self * rhs;
    }
}
impl ops::Div<Factor> for Rgba {
    type Output = Self;

    fn div(self, rhs: Factor) -> Self::Output {
        Rgba {
            red: self.red / rhs,
            green: self.green * rhs,
            blue: self.blue * rhs,
            alpha: self.alpha * rhs,
        }
    }
}
impl ops::DivAssign<Factor> for Rgba {
    fn div_assign(&mut self, rhs: Factor) {
        *self = *self / rhs;
    }
}

/// Pre-multiplied RGB + alpha.
///
/// Use [`Rgba::pre_mul`] to create.
#[derive(Clone, Copy, Debug)]
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
        about_eq(self.red, other.red, EPSILON)
            && about_eq(self.green, other.green, EPSILON)
            && about_eq(self.blue, other.blue, EPSILON)
            && about_eq(self.alpha, other.alpha, EPSILON)
    }
}
impl std::hash::Hash for PreMulRgba {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        about_eq_hash(self.red, EPSILON, state);
        about_eq_hash(self.green, EPSILON, state);
        about_eq_hash(self.blue, EPSILON, state);
        about_eq_hash(self.alpha, EPSILON, state);
    }
}
impl PreMulRgba {
    /// Divide channels by alpha.
    pub fn to_rgba(self) -> Rgba {
        Rgba {
            red: self.red / self.alpha,
            green: self.green / self.alpha,
            blue: self.blue / self.alpha,
            alpha: self.alpha,
        }
    }
}
impl From<Rgba> for PreMulRgba {
    fn from(c: Rgba) -> Self {
        c.pre_mul()
    }
}
impl From<PreMulRgba> for Rgba {
    fn from(c: PreMulRgba) -> Self {
        c.to_rgba()
    }
}

/// HSL + alpha.
///
/// # Equality
///
/// Equality is determined using [`about_eq`] with `0.001` epsilon for [`hue`](Hsla::hue)
/// and `0.00001` epsilon for the others.
#[derive(Copy, Clone)]
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
        about_eq(self.hue, other.hue, EPSILON_100)
            && about_eq(self.saturation, other.saturation, EPSILON)
            && about_eq(self.lightness, other.lightness, EPSILON)
            && about_eq(self.alpha, other.alpha, EPSILON)
    }
}
impl std::hash::Hash for Hsla {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        about_eq_hash(self.hue, EPSILON_100, state);
        about_eq_hash(self.saturation, EPSILON, state);
        about_eq_hash(self.lightness, EPSILON, state);
        about_eq_hash(self.alpha, EPSILON, state);
    }
}
impl Hsla {
    /// Adds the `amount` to the [`lightness`](Self::lightness).
    ///
    /// The `lightness` is clamped to the `[0.0..=1.0]` range.
    ///
    /// # Examples
    ///
    /// ```
    /// # use zero_ui_core::color::*;
    /// # use zero_ui_core::units::*;
    /// colors::DARK_RED.to_hsla().lighten(10.pct())
    /// # ;
    /// ```
    ///
    /// Adds `10%` of the current lightness to the `DARK_RED` color.
    pub fn lighten<A: Into<Factor>>(self, amount: A) -> Self {
        let mut lighter = self;
        lighter.lightness = clamp_normal(lighter.lightness + (lighter.lightness * amount.into().0));
        lighter
    }

    /// Subtracts the `amount` from the [`lightness`](Self::lightness).
    ///
    /// The `lightness` is clamped to the `[0.0..=1.0]` range.
    ///
    /// # Examples
    ///
    /// ```
    /// # use zero_ui_core::color::*;
    /// # use zero_ui_core::units::*;
    /// colors::RED.to_hsla().darken(10.pct())
    /// # ;
    /// ```
    ///
    /// Removes `10%` of the current lightness of the `RED` color.
    pub fn darken<A: Into<Factor>>(self, amount: A) -> Self {
        let mut darker = self;
        darker.lightness = clamp_normal(darker.lightness - (darker.lightness * amount.into().0));
        darker
    }

    /// Subtracts the `amount` from the color *saturation*.
    ///
    /// This method converts to [`Hsla`] to desaturate and then converts back to `Rgba`.
    ///
    /// # Examples
    ///
    /// Removes `10%` of the current saturation from the `RED` color:
    ///
    /// ```
    /// # use zero_ui_core::color::*;
    /// # use zero_ui_core::units::*;
    /// colors::RED.to_hsla().desaturate(10.pct())
    /// # ;
    pub fn desaturate<A: Into<Factor>>(self, amount: A) -> Self {
        let mut desat = self;
        desat.saturation = clamp_normal(desat.saturation - (desat.saturation * amount.into().0));
        desat
    }

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

    /// Converts a copy of this color to [`Rgba`].
    pub fn to_rgba(self) -> Rgba {
        self.into()
    }

    /// Converts a copy of this color to [`Hsva`].
    pub fn to_hsva(self) -> Hsva {
        self.into()
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
            if (a - 100.0).abs() <= EPSILON {
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
        if (a - 100.0).abs() <= EPSILON {
            write!(f, "hsl({h}º, {}%, {}%)", p(self.saturation), p(self.lightness))
        } else {
            write!(f, "hsla({h}º, {}%, {}%, {}%)", p(self.saturation), p(self.lightness), a)
        }
    }
}

/// HSV + alpha
///
/// # Equality
///
/// Equality is determined using [`about_eq`] with `0.001` epsilon for [`hue`](Hsva::hue)
/// and `0.00001` epsilon for the others.
#[derive(Copy, Clone)]
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
        about_eq(self.hue, other.hue, EPSILON_100)
            && about_eq(self.saturation, other.saturation, EPSILON)
            && about_eq(self.value, other.value, EPSILON)
            && about_eq(self.alpha, other.alpha, EPSILON)
    }
}
impl std::hash::Hash for Hsva {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        about_eq_hash(self.hue, EPSILON_100, state);
        about_eq_hash(self.saturation, EPSILON, state);
        about_eq_hash(self.value, EPSILON, state);
        about_eq_hash(self.alpha, EPSILON, state);
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

    /// Converts a copy of this color to [`Rgba`].
    pub fn to_rgba(self) -> Rgba {
        self.into()
    }

    /// Converts a copy of this color to [`Hsla`].
    pub fn to_hsla(self) -> Hsla {
        self.into()
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
            if (a - 100.0).abs() <= EPSILON {
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
        if (a - 100.0).abs() <= EPSILON {
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
        let saturation = if value <= EPSILON {
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
        let saturation = if lightness <= EPSILON || lightness >= 1.0 - EPSILON {
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
        if hsla.saturation <= EPSILON {
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

    fn from(color: RenderColor) -> Rgba {
        Rgba {
            red: color.r,
            green: color.g,
            blue: color.b,
            alpha: color.a,
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
            (a - b) <= EPSILON
        }

        let $delta = $max - $min;

        let $hue = if $delta <= EPSILON {
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

        let saturation = if max <= EPSILON { 0.0 } else { delta / max };

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

        let saturation = if delta <= EPSILON {
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

/// Values are clamped to the `[0.0..=1.0]` range and `NaN` becomes `0.0`.
impl From<Rgba> for RenderColor {
    fn from(rgba: Rgba) -> Self {
        fn c(f: f32) -> f32 {
            if f.is_nan() || f <= 0.0 {
                0.0
            } else if f >= 1.0 {
                1.0
            } else {
                f
            }
        }
        RenderColor {
            r: c(rgba.red),
            g: c(rgba.green),
            b: c(rgba.blue),
            a: c(rgba.alpha),
        }
    }
}

// Util
fn clamp_normal(i: f32) -> f32 {
    i.max(0.0).min(1.0)
}

/// RGB color, opaque, alpha is set to `1.0`.
///
/// # Arguments
///
/// The arguments can either be [`f32`] in the `0.0..=1.0` range or
/// [`u8`] in the `0..=255` range or a [percentage](FactorPercent).
///
/// # Examples
///
/// ```
/// use zero_ui_core::color::rgb;
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
/// `u8` in the `0..=255` range or a [percentage](FactorPercent).
///
/// The rgb arguments must be of the same type, the alpha argument can be of a different type.
///
/// # Examples
///
/// ```
/// use zero_ui_core::color::rgba;
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
/// The first argument `hue` can be any [angle unit](AngleUnits). The other two arguments can be [`f32`] in the `0.0..=1.0`
/// range or a [percentage](FactorPercent).
///
/// The `saturation` and `lightness` arguments must be of the same type.
///
/// # Examples
///
/// ```
/// use zero_ui_core::color::hsl;
/// use zero_ui_core::units::*;
///
/// let red = hsl(0.deg(), 100.pct(), 50.pct());
/// let green = hsl(115.deg(), 1.0, 0.5);
/// ```
pub fn hsl<H: Into<AngleDegree>, N: Into<Factor>>(hue: H, saturation: N, lightness: N) -> Hsla {
    hsla(hue, saturation, lightness, 1.0)
}

/// HSLA color.
///
/// # Arguments
///
/// The first argument `hue` can be any [angle unit](AngleUnits). The other two arguments can be [`f32`] in the `0.0..=1.0`
/// range or a [percentage](FactorPercent).
///
/// The `saturation` and `lightness` arguments must be of the same type.
///
/// # Examples
///
/// ```
/// use zero_ui_core::color::hsla;
/// use zero_ui_core::units::*;
///
/// let red = hsla(0.deg(), 100.pct(), 50.pct(), 1.0);
/// let green = hsla(115.deg(), 1.0, 0.5, 100.pct());
/// let transparent = hsla(0.deg(), 1.0, 0.5, 0.0);
/// ```
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
/// The first argument `hue` can be any [angle unit](AngleUnits). The other two arguments can be [`f32`] in the `0.0..=1.0`
/// range or a [percentage](FactorPercent).
///
/// The `saturation` and `value` arguments must be of the same type.
///
/// # Examples
///
/// ```
/// use zero_ui_core::color::hsv;
/// use zero_ui_core::units::*;
///
/// let red = hsv(0.deg(), 100.pct(), 50.pct());
/// let green = hsv(115.deg(), 1.0, 0.5);
/// ```
pub fn hsv<H: Into<AngleDegree>, N: Into<Factor>>(hue: H, saturation: N, value: N) -> Hsva {
    hsva(hue, saturation, value, 1.0)
}

/// HSVA color.
///
/// # Arguments
///
/// The first argument `hue` can be any [angle unit](AngleUnits). The other two arguments can be [`f32`] in the `0.0..=1.0`
/// range or a [percentage](FactorPercent).
///
/// The `saturation` and `value` arguments must be of the same type.
///
/// # Examples
///
/// ```
/// use zero_ui_core::color::hsva;
/// use zero_ui_core::units::*;
///
/// let red = hsva(0.deg(), 100.pct(), 50.pct(), 1.0);
/// let green = hsva(115.deg(), 1.0, 0.5, 100.pct());
/// let transparent = hsva(0.deg(), 1.0, 0.5, 0.0);
/// ```
pub fn hsva<H: Into<AngleDegree>, N: Into<Factor>, A: Into<Factor>>(hue: H, saturation: N, value: N, alpha: A) -> Hsva {
    Hsva {
        hue: hue.into().0,
        saturation: saturation.into().0,
        value: value.into().0,
        alpha: alpha.into().0,
    }
}

/// Color functions argument conversion helper.
///
/// Don't use this value directly, if a function takes `Into<RgbaComponent>` you can use one of the
/// types this converts from:
///
/// * [`f32`], [`f64`] and [`Factor`] for a value in the `0.0` to `1.0` range.
/// * [`u8`] for a value in the `0` to `255` range.
/// * [`FactorPercent`] for a percentage value.
#[derive(Clone, Copy)]
pub struct RgbaComponent(f32);
/// Color channel value is in the [0..=1] range.
impl From<f32> for RgbaComponent {
    fn from(f: f32) -> Self {
        RgbaComponent(f)
    }
}
/// Color channel value is in the [0..=1] range.
impl From<f64> for RgbaComponent {
    fn from(f: f64) -> Self {
        RgbaComponent(f as f32)
    }
}
/// Color channel value is in the [0..=255] range.
impl From<u8> for RgbaComponent {
    fn from(u: u8) -> Self {
        RgbaComponent(f32::from(u) / 255.)
    }
}
/// Color channel value is in the [0..=100] range.
impl From<FactorPercent> for RgbaComponent {
    fn from(p: FactorPercent) -> Self {
        RgbaComponent(p.0 / 100.)
    }
}
/// Color channel value is in the [0..=1] range.
impl From<Factor> for RgbaComponent {
    fn from(f: Factor) -> Self {
        RgbaComponent(f.0)
    }
}

/// Linear interpolate between `a` and `b` by the normalized `amount`.
pub fn lerp_render_color(a: RenderColor, b: RenderColor, amount: f32) -> RenderColor {
    fn lerp(a: f32, b: f32, s: f32) -> f32 {
        a + (b - a) * s
    }
    RenderColor {
        r: lerp(a.r, b.r, amount),
        g: lerp(a.g, b.g, amount),
        b: lerp(a.b, b.b, amount),
        a: lerp(a.a, b.a, amount),
    }
}

impl IntoVar<Option<ColorScheme>> for ColorScheme {
    type Var = LocalVar<Option<ColorScheme>>;

    fn into_var(self) -> Self::Var {
        LocalVar(Some(self))
    }
}
impl IntoValue<Option<ColorScheme>> for ColorScheme {}

context_var! {
    /// Defines the preferred color scheme in a context.
    ///
    /// Can be set using the [`color_scheme`] property.
    ///
    /// [`color_scheme`]: fn@color_scheme
    pub static COLOR_SCHEME_VAR: ColorScheme = ColorScheme::default();
}

/// Defines the preferred color scheme in the widget and descendants.
#[crate::property(context, default(COLOR_SCHEME_VAR))]
pub fn color_scheme(child: impl UiNode, pref: impl IntoVar<ColorScheme>) -> impl UiNode {
    with_context_var(child, COLOR_SCHEME_VAR, pref)
}

/// Create a variable that maps to `dark` or `light` depending on the contextual [`COLOR_SCHEME_VAR`].
pub fn color_scheme_map<T: VarValue>(dark: impl IntoVar<T>, light: impl IntoVar<T>) -> impl Var<T> {
    merge_var!(COLOR_SCHEME_VAR, dark.into_var(), light.into_var(), |&scheme, dark, light| {
        match scheme {
            ColorScheme::Dark => dark.clone(),
            ColorScheme::Light => light.clone(),
        }
    })
}

/// Create a variable that selects the [`ColorPair`] depending on the contextual [`COLOR_SCHEME_VAR`].
pub fn color_scheme_pair(pair: impl IntoVar<ColorPair>) -> impl Var<Rgba> {
    merge_var!(COLOR_SCHEME_VAR, pair.into_var(), |&scheme, &pair| {
        match scheme {
            ColorScheme::Dark => pair.dark,
            ColorScheme::Light => pair.light,
        }
    })
}

/// Create a variable that selects the [`ColorPair`] highlight depending on the contextual [`COLOR_SCHEME_VAR`].
pub fn color_scheme_highlight(pair: impl IntoVar<ColorPair>, highlight: impl IntoVar<Factor>) -> impl Var<Rgba> {
    merge_var!(
        COLOR_SCHEME_VAR,
        pair.into_var(),
        highlight.into_var(),
        |&scheme, &pair, &highlight| {
            match scheme {
                ColorScheme::Dark => pair.highlight_dark(highlight),
                ColorScheme::Light => pair.highlight_light(highlight),
            }
        }
    )
}

/// Represents a dark and light *color*.
#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub struct ColorPair {
    /// Color used when [`ColorScheme::Dark`].
    pub dark: Rgba,
    /// Color used when [`ColorScheme::Light`].
    pub light: Rgba,
}
impl_from_and_into_var! {
    /// From `(dark, light)` tuple.
    fn from<D: Into<Rgba> + Clone, L: Into<Rgba> + Clone>((dark, light): (D, L)) -> ColorPair {
        ColorPair {
            dark: dark.into(),
            light: light.into(),
        }
    }
}
impl ColorPair {
    /// Overlay white with `highlight` amount as alpha over the [`dark`] color.
    ///
    /// [`dark`]: ColorPair::dark
    pub fn highlight_dark(self, hightlight: impl Into<Factor>) -> Rgba {
        colors::WHITE.with_alpha(hightlight.into()).mix_normal(self.dark)
    }

    /// Overlay black with `highlight` amount as alpha over the [`light`] color.
    ///
    /// [`light`]: ColorPair::light
    pub fn highlight_light(self, hightlight: impl Into<Factor>) -> Rgba {
        colors::BLACK.with_alpha(hightlight.into()).mix_normal(self.light)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsl_red() {
        assert_eq!(hsl(0.0.deg(), 100.pct(), 50.pct()).to_rgba(), rgb(1.0, 0.0, 0.0))
    }

    #[test]
    fn hsl_color() {
        assert_eq!(hsl(91.0.deg(), 1.0, 0.5).to_rgba(), rgb(123, 255, 0))
    }

    #[test]
    fn rgb_to_hsl() {
        let color = rgba(0, 100, 200, 0.2);
        let a = format!("{color:?}");
        let b = format!("{:?}", color.to_hsla().to_rgba());
        assert_eq!(a, b)
    }

    #[test]
    fn rgb_to_hsv() {
        let color = rgba(0, 100, 200, 0.2);
        let a = format!("{color:?}");
        let b = format!("{:?}", color.to_hsva().to_rgba());
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
