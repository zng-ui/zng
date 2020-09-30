//! Color types.

use super::{context::LayoutContext, render::FrameBinding, units::*};
use std::fmt;
use webrender::api::FilterOp;
pub use zero_ui_macros::hex_color as hex;

/// Webrender RGBA.
pub type RenderColor = webrender::api::ColorF;

/// RGB + alpha.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Rgba {
    /// [0.0..1.0]
    pub red: f32,
    /// [0.0..1.0]
    pub green: f32,
    /// [0.0..1.0]
    pub blue: f32,
    /// [0.0..1.0]
    pub alpha: f32,
}
impl Rgba {
    /// See [`rgba`] for a better constructor.
    pub fn new(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self { red, green, blue, alpha }
    }

    pub fn set_red<R: Into<RgbaComponent>>(&mut self, red: R) {
        self.red = clamp_normal(red.into().0)
    }

    pub fn set_green<G: Into<RgbaComponent>>(&mut self, green: G) {
        self.green = clamp_normal(green.into().0)
    }

    pub fn set_blue<B: Into<RgbaComponent>>(&mut self, blue: B) {
        self.blue = clamp_normal(blue.into().0)
    }

    pub fn set_alpha<A: Into<RgbaComponent>>(&mut self, alpha: A) {
        self.alpha = clamp_normal(alpha.into().0)
    }

    #[inline]
    pub fn to_hsla(self) -> Hsla {
        self.into()
    }

    #[inline]
    pub fn to_hsva(self) -> Hsva {
        self.into()
    }
}
impl fmt::Display for Rgba {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

/// HSL + alpha.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Hsla {
    /// [0.0..=360.0]
    pub hue: f32,
    /// [0.0..1.0]
    pub saturation: f32,
    /// [0.0..1.0]
    pub lightness: f32,
    /// [0.0..1.0]
    pub alpha: f32,
}
impl Hsla {
    pub fn lighten<A: Into<FactorNormal>>(self, ammount: A) -> Self {
        let mut lighter = self;
        lighter.lightness = clamp_normal(lighter.lightness + ammount.into().0);
        lighter
    }

    pub fn darken<A: Into<FactorNormal>>(self, ammount: A) -> Self {
        let mut darker = self;
        darker.lightness = clamp_normal(darker.lightness - ammount.into().0);
        darker
    }

    pub fn set_hue<H: Into<AngleDegree>>(&mut self, hue: H) {
        self.hue = hue.into().modulo().0
    }

    pub fn set_lightness<L: Into<FactorNormal>>(&mut self, lightness: L) {
        self.lightness = lightness.into().clamp_range().0;
    }

    pub fn set_saturation<L: Into<FactorNormal>>(&mut self, saturation: L) {
        self.saturation = saturation.into().clamp_range().0;
    }

    pub fn set_alpha<A: Into<FactorNormal>>(&mut self, alpha: A) {
        self.alpha = alpha.into().clamp_range().0
    }

    #[inline]
    pub fn to_rgba(self) -> Rgba {
        self.into()
    }

    #[inline]
    pub fn to_hsva(self) -> Hsva {
        self.into()
    }
}
impl fmt::Display for Hsla {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn p(n: f32) -> f32 {
            clamp_normal(n) * 100.0
        }
        let a = p(self.alpha);
        let h = AngleDegree(self.hue).modulo().0.round();
        if (a - 100.0).abs() <= f32::EPSILON {
            write!(f, "hsl({}º, {}%, {}%)", h, p(self.saturation), p(self.lightness))
        } else {
            write!(f, "hsla({}º, {}%, {}%, {}%)", h, p(self.saturation), p(self.lightness), a)
        }
    }
}

/// HSV + alpha
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Hsva {
    /// [0.0..=360.0]
    pub hue: f32,
    /// [0.0..1.0]
    pub saturation: f32,
    /// [0.0..1.0]
    pub value: f32,
    /// [0.0..1.0]
    pub alpha: f32,
}

impl Hsva {
    pub fn set_hue<H: Into<AngleDegree>>(&mut self, hue: H) {
        self.hue = hue.into().modulo().0
    }

    pub fn set_value<L: Into<FactorNormal>>(&mut self, value: L) {
        self.value = value.into().clamp_range().0;
    }

    pub fn set_saturation<L: Into<FactorNormal>>(&mut self, saturation: L) {
        self.saturation = saturation.into().clamp_range().0;
    }

    pub fn set_alpha<A: Into<FactorNormal>>(&mut self, alpha: A) {
        self.alpha = alpha.into().clamp_range().0
    }

    #[inline]
    pub fn to_rgba(self) -> Rgba {
        self.into()
    }

    #[inline]
    pub fn to_hsla(self) -> Hsla {
        self.into()
    }
}

impl fmt::Display for Hsva {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn p(n: f32) -> f32 {
            clamp_normal(n) * 100.0
        }
        let a = p(self.alpha);
        let h = AngleDegree(self.hue).modulo().0.round();
        if (a - 100.0).abs() <= f32::EPSILON {
            write!(f, "hsv({}º, {}%, {}%)", h, p(self.saturation), p(self.value))
        } else {
            write!(f, "hsva({}º, {}%, {}%, {}%)", h, p(self.saturation), p(self.value), a)
        }
    }
}
impl_from_and_into_var! {
    fn from(hsla: Hsla) -> Hsva {{
        let lightness = clamp_normal(hsla.lightness);
        let saturation = clamp_normal(hsla.saturation);

        let value = lightness + saturation * lightness.min(1.0 - lightness);
        let saturation = if value <= f32::EPSILON {
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
    }}

    fn from(hsva: Hsva) -> Hsla {{
        let saturation = clamp_normal(hsva.saturation);
        let value = clamp_normal(hsva.value);

        let lightness = value * (1.0 - saturation / 2.0);
        let saturation = if lightness <= f32::EPSILON || lightness >= 1.0 - f32::EPSILON {
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
    }}

    fn from(hsva: Hsva) -> Rgba {{
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
    }}

    fn from(hsla: Hsla) -> Rgba {{
        if hsla.saturation <= f32::EPSILON {
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
    }}
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
            (a - b) <= f32::EPSILON
        }

        let $delta = $max - $min;

        let $hue = if $delta <= f32::EPSILON {
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
    fn from(rgba: Rgba) -> Hsva {{
        cylindrical_color!(rgba -> (min, max, delta, hue));

        let saturation = if max <= f32::EPSILON { 0.0 } else { delta / max };

        let value = max;

        Hsva {
            hue,
            saturation,
            value,
            alpha: rgba.alpha,
        }
    }}

    fn from(rgba: Rgba) -> Hsla {{
        cylindrical_color!(rgba -> (min, max, delta, hue));

        let lightness = (max + min) / 2.0;

        let saturation = if delta <= f32::EPSILON {
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
    }}
}

impl From<Rgba> for RenderColor {
    fn from(rgba: Rgba) -> Self {
        RenderColor {
            r: rgba.red,
            g: rgba.green,
            b: rgba.blue,
            a: rgba.alpha,
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
/// # Example
/// ```
/// use zero_ui::core::color::rgb;
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
/// # Example
/// ```
/// use zero_ui::core::color::rgba;
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
/// # Example
///
/// ```
/// use zero_ui::core::color::hsl;
/// use zero_ui::core::units::*;
///
/// let red = hsl(0.deg(), 100.pct(), 50.pct());
/// let green = hsl(115.deg(), 1.0, 0.5);
/// ```
pub fn hsl<H: Into<AngleDegree>, N: Into<FactorNormal>>(hue: H, saturation: N, lightness: N) -> Hsla {
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
/// # Example
///
/// ```
/// use zero_ui::core::color::hsla;
/// use zero_ui::core::units::*;
///
/// let red = hsla(0.deg(), 100.pct(), 50.pct(), 1.0);
/// let green = hsla(115.deg(), 1.0, 0.5, 100.pct());
/// let transparent = hsla(0.deg(), 1.0, 0.5, 0.0);
/// ```
pub fn hsla<H: Into<AngleDegree>, N: Into<FactorNormal>, A: Into<FactorNormal>>(hue: H, saturation: N, lightness: N, alpha: A) -> Hsla {
    Hsla {
        hue: hue.into().0,
        saturation: saturation.into().0,
        lightness: lightness.into().0,
        alpha: alpha.into().0,
    }
}

pub fn hsv<H: Into<AngleDegree>, N: Into<FactorNormal>>(hue: H, saturation: N, value: N) -> Hsva {
    hsva(hue, saturation, value, 1.0)
}

pub fn hsva<H: Into<AngleDegree>, N: Into<FactorNormal>, A: Into<FactorNormal>>(hue: H, saturation: N, value: N, alpha: A) -> Hsva {
    Hsva {
        hue: hue.into().0,
        saturation: saturation.into().0,
        value: value.into().0,
        alpha: alpha.into().0,
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
        let a = color.to_string();
        let b = color.to_hsla().to_rgba().to_string();
        assert_eq!(a, b)
    }

    #[test]
    fn rgb_to_hsvl() {
        let color = rgba(0, 100, 200, 0.2);
        let a = color.to_string();
        let b = color.to_hsva().to_rgba().to_string();
        assert_eq!(a, b)
    }

    // #[test]
    // fn rgb_to_hsv_all() {
    //     for r in 0..=255 {
    //         println!("{}", r);
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

/// [`rgb`](rgb()) and [`rgba`] argument conversion helper.
pub struct RgbaComponent(pub f32);
impl From<f32> for RgbaComponent {
    fn from(f: f32) -> Self {
        RgbaComponent(f)
    }
}
impl From<u8> for RgbaComponent {
    fn from(u: u8) -> Self {
        RgbaComponent(f32::from(u) / 255.)
    }
}
impl From<FactorPercent> for RgbaComponent {
    fn from(p: FactorPercent) -> Self {
        RgbaComponent(p.0 / 100.)
    }
}

#[derive(Clone, Default, Debug)]
pub struct Filter {
    filters: Vec<FilterData>,
}
impl Filter {
    fn op(mut self, op: FilterOp) -> Self {
        self.filters.push(FilterData::Op(op));
        self
    }

    pub fn to_render(&self, available_size: LayoutSize, ctx: &LayoutContext) -> RenderFilter {
        self.filters
            .iter()
            .map(|f| match f {
                FilterData::Op(op) => *op,
                FilterData::Blur(l) => FilterOp::Blur(l.to_layout(LayoutLength::new(available_size.width), ctx).get()),
            })
            .collect()
    }

    pub fn opacity<A: Into<FactorNormal>>(self, alpha: A) -> Self {
        let alpha_value = alpha.into().0;
        self.op(FilterOp::Opacity(FrameBinding::Value(alpha_value), alpha_value))
    }

    pub fn invert<A: Into<FactorNormal>>(self, amount: A) -> Self {
        self.op(FilterOp::Invert(amount.into().0))
    }

    pub fn blur<R: Into<Length>>(mut self, radius: R) -> Self {
        self.filters.push(FilterData::Blur(radius.into()));
        self
    }

    pub fn sepia<A: Into<FactorNormal>>(self, amount: A) -> Self {
        self.op(FilterOp::Sepia(amount.into().0))
    }
}
pub type RenderFilter = Vec<FilterOp>;

#[derive(Clone, Debug)]
enum FilterData {
    Op(FilterOp),
    Blur(Length),
}

pub fn opacity<A: Into<FactorNormal>>(alpha: A) -> Filter {
    Filter::default().opacity(alpha)
}
pub fn invert<A: Into<FactorNormal>>(amount: A) -> Filter {
    Filter::default().invert(amount)
}
pub fn blur<R: Into<Length>>(radius: R) -> Filter {
    Filter::default().blur(radius)
}
pub fn sepia<A: Into<FactorNormal>>(amount: A) -> Filter {
    Filter::default().sepia(amount)
}

/// Named web colors
pub mod web_colors {
    use super::Rgba;

    macro_rules! rgb {
        ($r:literal, $g:literal, $b:literal) => {
            Rgba {
                red: $r as f32 / 255.,
                green: $g as f32 / 255.,
                blue: $b as f32 / 255.,
                alpha: 1.0,
            }
        };
    }

    /// Lavender (`#E6E6FA`)
    ///
    /// `rgb(230, 230, 250)`
    pub const LAVENDER: Rgba = rgb!(230, 230, 250);

    /// Thistle (`#D8BFD8`)
    ///
    /// `rgb(216, 191, 216)`
    pub const THISTLE: Rgba = rgb!(216, 191, 216);

    /// Plum (`#DDA0DD`)
    ///
    /// `rgb(221, 160, 221)`
    pub const PLUM: Rgba = rgb!(221, 160, 221);

    /// Violet (`#EE82EE`)
    ///
    /// `rgb(238, 130, 238)`
    pub const VIOLET: Rgba = rgb!(238, 130, 238);

    /// Orchid (`#DA70D6`)
    ///
    /// `rgb(218, 112, 214)`
    pub const ORCHID: Rgba = rgb!(218, 112, 214);

    /// Fuchsia (`#FF00FF`)
    ///
    /// `rgb(255, 0, 255)`
    pub const FUCHSIA: Rgba = rgb!(255, 0, 255);

    /// Magenta (`#FF00FF`)
    ///
    /// `rgb(255, 0, 255)`
    pub const MAGENTA: Rgba = rgb!(255, 0, 255);

    /// Medium Orchid (`#BA55D3`)
    ///
    /// `rgb(186, 85, 211)`
    pub const MEDIUM_ORCHID: Rgba = rgb!(186, 85, 211);

    /// Medium Purple (`#9370DB`)
    ///
    /// `rgb(147, 112, 219)`
    pub const MEDIUM_PURPLE: Rgba = rgb!(147, 112, 219);

    /// Blue Violet (`#8A2BE2`)
    ///
    /// `rgb(138, 43, 226)`
    pub const BLUE_VIOLET: Rgba = rgb!(138, 43, 226);

    /// Dark Violet (`#9400D3`)
    ///
    /// `rgb(148, 0, 211)`
    pub const DARK_VIOLET: Rgba = rgb!(148, 0, 211);

    /// Dark Orchid (`#9932CC`)
    ///
    /// `rgb(153, 50, 204)`
    pub const DARK_ORCHID: Rgba = rgb!(153, 50, 204);

    /// Dark Magenta (`#8B008B`)
    ///
    /// `rgb(139, 0, 139)`
    pub const DARK_MAGENTA: Rgba = rgb!(139, 0, 139);

    /// Purple (`#800080`)
    ///
    /// `rgb(128, 0, 128)`
    pub const PURPLE: Rgba = rgb!(128, 0, 128);

    /// Indigo (`#4B0082`)
    ///
    /// `rgb(75, 0, 130)`
    pub const INDIGO: Rgba = rgb!(75, 0, 130);

    /// Dark Slate Blue (`#483D8B`)
    ///
    /// `rgb(72, 61, 139)`
    pub const DARK_SLATE_BLUE: Rgba = rgb!(72, 61, 139);

    /// Slate Blue (`#6A5ACD`)
    ///
    /// `rgb(106, 90, 205)`
    pub const SLATE_BLUE: Rgba = rgb!(106, 90, 205);

    /// Medium Slate Blue (`#7B68EE`)
    ///
    /// `rgb(123, 104, 238)`
    pub const MEDIUM_SLATE_BLUE: Rgba = rgb!(123, 104, 238);

    /// Pink (`#FFC0CB`)
    ///
    /// `rgb(255, 192, 203)`
    pub const PINK: Rgba = rgb!(255, 192, 203);

    /// Light Pink (`#FFB6C1`)
    ///
    /// `rgb(255, 182, 193)`
    pub const LIGHT_PINK: Rgba = rgb!(255, 182, 193);

    /// Hot Pink (`#FF69B4`)
    ///
    /// `rgb(255, 105, 180)`
    pub const HOT_PINK: Rgba = rgb!(255, 105, 180);

    /// Deep Pink (`#FF1493`)
    ///
    /// `rgb(255, 20, 147)`
    pub const DEEP_PINK: Rgba = rgb!(255, 20, 147);

    /// Pale Violet Red (`#DB7093`)
    ///
    /// `rgb(219, 112, 147)`
    pub const PALE_VIOLET_RED: Rgba = rgb!(219, 112, 147);

    /// Medium Violet Red (`#C71585`)
    ///
    /// `rgb(199, 21, 133)`
    pub const MEDIUM_VIOLET_RED: Rgba = rgb!(199, 21, 133);

    /// Light Salmon (`#FFA07A`)
    ///
    /// `rgb(255, 160, 122)`
    pub const LIGHT_SALMON: Rgba = rgb!(255, 160, 122);

    /// Salmon (`#FA8072`)
    ///
    /// `rgb(250, 128, 114)`
    pub const SALMON: Rgba = rgb!(250, 128, 114);

    /// Dark Salmon (`#E9967A`)
    ///
    /// `rgb(233, 150, 122)`
    pub const DARK_SALMON: Rgba = rgb!(233, 150, 122);

    /// Light Coral (`#F08080`)
    ///
    /// `rgb(240, 128, 128)`
    pub const LIGHT_CORAL: Rgba = rgb!(240, 128, 128);

    /// Indian Red (`#CD5C5C`)
    ///
    /// `rgb(205, 92, 92)`
    pub const INDIAN_RED: Rgba = rgb!(205, 92, 92);

    /// Crimson (`#DC143C`)
    ///
    /// `rgb(220, 20, 60)`
    pub const CRIMSON: Rgba = rgb!(220, 20, 60);

    /// Fire Brick (`#B22222`)
    ///
    /// `rgb(178, 34, 34)`
    pub const FIRE_BRICK: Rgba = rgb!(178, 34, 34);

    /// Dark Red (`#8B0000`)
    ///
    /// `rgb(139, 0, 0)`
    pub const DARK_RED: Rgba = rgb!(139, 0, 0);

    /// Red (`#FF0000`)
    ///
    /// `rgb(255, 0, 0)`
    pub const RED: Rgba = rgb!(255, 0, 0);

    /// Orange Red (`#FF4500`)
    ///
    /// `rgb(255, 69, 0)`
    pub const ORANGE_RED: Rgba = rgb!(255, 69, 0);

    /// Tomato (`#FF6347`)
    ///
    /// `rgb(255, 99, 71)`
    pub const TOMATO: Rgba = rgb!(255, 99, 71);

    /// Coral (`#FF7F50`)
    ///
    /// `rgb(255, 127, 80)`
    pub const CORAL: Rgba = rgb!(255, 127, 80);

    /// Dark Orange (`#FF8C00`)
    ///
    /// `rgb(255, 140, 0)`
    pub const DARK_ORANGE: Rgba = rgb!(255, 140, 0);

    /// Orange (`#FFA500`)
    ///
    /// `rgb(255, 165, 0)`
    pub const ORANGE: Rgba = rgb!(255, 165, 0);

    /// Yellow (`#FFFF00`)
    ///
    /// `rgb(255, 255, 0)`
    pub const YELLOW: Rgba = rgb!(255, 255, 0);

    /// Light Yellow (`#FFFFE0`)
    ///
    /// `rgb(255, 255, 224)`
    pub const LIGHT_YELLOW: Rgba = rgb!(255, 255, 224);

    /// Lemon Chiffon (`#FFFACD`)
    ///
    /// `rgb(255, 250, 205)`
    pub const LEMON_CHIFFON: Rgba = rgb!(255, 250, 205);

    /// Light Goldenrod Yellow (`#FAFAD2`)
    ///
    /// `rgb(250, 250, 210)`
    pub const LIGHT_GOLDENROD_YELLOW: Rgba = rgb!(250, 250, 210);

    /// Papaya Whip (`#FFEFD5`)
    ///
    /// `rgb(255, 239, 213)`
    pub const PAPAYA_WHIP: Rgba = rgb!(255, 239, 213);

    /// Moccasin (`#FFE4B5`)
    ///
    /// `rgb(255, 228, 181)`
    pub const MOCCASIN: Rgba = rgb!(255, 228, 181);

    /// Peach Puff (`#FFDAB9`)
    ///
    /// `rgb(255, 218, 185)`
    pub const PEACH_PUFF: Rgba = rgb!(255, 218, 185);

    /// Pale Goldenrod (`#EEE8AA`)
    ///
    /// `rgb(238, 232, 170)`
    pub const PALE_GOLDENROD: Rgba = rgb!(238, 232, 170);

    /// Khaki (`#F0E68C`)
    ///
    /// `rgb(240, 230, 140)`
    pub const KHAKI: Rgba = rgb!(240, 230, 140);

    /// Dark Khaki (`#BDB76B`)
    ///
    /// `rgb(189, 183, 107)`
    pub const DARK_KHAKI: Rgba = rgb!(189, 183, 107);

    /// Gold (`#FFD700`)
    ///
    /// `rgb(255, 215, 0)`
    pub const GOLD: Rgba = rgb!(255, 215, 0);

    /// Cornsilk (`#FFF8DC`)
    ///
    /// `rgb(255, 248, 220)`
    pub const CORNSILK: Rgba = rgb!(255, 248, 220);

    /// Blanched Almond (`#FFEBCD`)
    ///
    /// `rgb(255, 235, 205)`
    pub const BLANCHED_ALMOND: Rgba = rgb!(255, 235, 205);

    /// Bisque (`#FFE4C4`)
    ///
    /// `rgb(255, 228, 196)`
    pub const BISQUE: Rgba = rgb!(255, 228, 196);

    /// Navajo White (`#FFDEAD`)
    ///
    /// `rgb(255, 222, 173)`
    pub const NAVAJO_WHITE: Rgba = rgb!(255, 222, 173);

    /// Wheat (`#F5DEB3`)
    ///
    /// `rgb(245, 222, 179)`
    pub const WHEAT: Rgba = rgb!(245, 222, 179);

    /// Burly Wood (`#DEB887`)
    ///
    /// `rgb(222, 184, 135)`
    pub const BURLY_WOOD: Rgba = rgb!(222, 184, 135);

    /// Tan (`#D2B48C`)
    ///
    /// `rgb(210, 180, 140)`
    pub const TAN: Rgba = rgb!(210, 180, 140);

    /// Rosy Brown (`#BC8F8F`)
    ///
    /// `rgb(188, 143, 143)`
    pub const ROSY_BROWN: Rgba = rgb!(188, 143, 143);

    /// Sandy Brown (`#F4A460`)
    ///
    /// `rgb(244, 164, 96)`
    pub const SANDY_BROWN: Rgba = rgb!(244, 164, 96);

    /// Goldenrod (`#DAA520`)
    ///
    /// `rgb(218, 165, 32)`
    pub const GOLDENROD: Rgba = rgb!(218, 165, 32);

    /// Dark Goldenrod (`#B8860B`)
    ///
    /// `rgb(184, 134, 11)`
    pub const DARK_GOLDENROD: Rgba = rgb!(184, 134, 11);

    /// Peru (`#CD853F`)
    ///
    /// `rgb(205, 133, 63)`
    pub const PERU: Rgba = rgb!(205, 133, 63);

    /// Chocolate (`#D2691E`)
    ///
    /// `rgb(210, 105, 30)`
    pub const CHOCOLATE: Rgba = rgb!(210, 105, 30);

    /// Saddle Brown (`#8B4513`)
    ///
    /// `rgb(139, 69, 19)`
    pub const SADDLE_BROWN: Rgba = rgb!(139, 69, 19);

    /// Sienna (`#A0522D`)
    ///
    /// `rgb(160, 82, 45)`
    pub const SIENNA: Rgba = rgb!(160, 82, 45);

    /// Brown (`#A52A2A`)
    ///
    /// `rgb(165, 42, 42)`
    pub const BROWN: Rgba = rgb!(165, 42, 42);

    /// Maroon (`#800000`)
    ///
    /// `rgb(128, 0, 0)`
    pub const MAROON: Rgba = rgb!(128, 0, 0);

    /// Dark Olive Green (`#556B2F`)
    ///
    /// `rgb(85, 107, 47)`
    pub const DARK_OLIVE_GREEN: Rgba = rgb!(85, 107, 47);

    /// Olive (`#808000`)
    ///
    /// `rgb(128, 128, 0)`
    pub const OLIVE: Rgba = rgb!(128, 128, 0);

    /// Olive Drab (`#6B8E23`)
    ///
    /// `rgb(107, 142, 35)`
    pub const OLIVE_DRAB: Rgba = rgb!(107, 142, 35);

    /// Yellow Green (`#9ACD32`)
    ///
    /// `rgb(154, 205, 50)`
    pub const YELLOW_GREEN: Rgba = rgb!(154, 205, 50);

    /// Lime Green (`#32CD32`)
    ///
    /// `rgb(50, 205, 50)`
    pub const LIME_GREEN: Rgba = rgb!(50, 205, 50);

    /// Lime (`#00FF00`)
    ///
    /// `rgb(0, 255, 0)`
    pub const LIME: Rgba = rgb!(0, 255, 0);

    /// Lawn Green (`#7CFC00`)
    ///
    /// `rgb(124, 252, 0)`
    pub const LAWN_GREEN: Rgba = rgb!(124, 252, 0);

    /// Chartreuse (`#7FFF00`)
    ///
    /// `rgb(127, 255, 0)`
    pub const CHARTREUSE: Rgba = rgb!(127, 255, 0);

    /// Green Yellow (`#ADFF2F`)
    ///
    /// `rgb(173, 255, 47)`
    pub const GREEN_YELLOW: Rgba = rgb!(173, 255, 47);

    /// Spring Green (`#00FF7F`)
    ///
    /// `rgb(0, 255, 127)`
    pub const SPRING_GREEN: Rgba = rgb!(0, 255, 127);

    /// Medium Spring Green (`#00FA9A`)
    ///
    /// `rgb(0, 250, 154)`
    pub const MEDIUM_SPRING_GREEN: Rgba = rgb!(0, 250, 154);

    /// Light Green (`#90EE90`)
    ///
    /// `rgb(144, 238, 144)`
    pub const LIGHT_GREEN: Rgba = rgb!(144, 238, 144);

    /// Pale Green (`#98FB98`)
    ///
    /// `rgb(152, 251, 152)`
    pub const PALE_GREEN: Rgba = rgb!(152, 251, 152);

    /// Dark Sea Green (`#8FBC8F`)
    ///
    /// `rgb(143, 188, 143)`
    pub const DARK_SEA_GREEN: Rgba = rgb!(143, 188, 143);

    /// Medium Sea Green (`#3CB371`)
    ///
    /// `rgb(60, 179, 113)`
    pub const MEDIUM_SEA_GREEN: Rgba = rgb!(60, 179, 113);

    /// Sea Green (`#2E8B57`)
    ///
    /// `rgb(46, 139, 87)`
    pub const SEA_GREEN: Rgba = rgb!(46, 139, 87);

    /// Forest Green (`#228B22`)
    ///
    /// `rgb(34, 139, 34)`
    pub const FOREST_GREEN: Rgba = rgb!(34, 139, 34);

    /// Green (`#008000`)
    ///
    /// `rgb(0, 128, 0)`
    pub const GREEN: Rgba = rgb!(0, 128, 0);

    /// Dark Green (`#006400`)
    ///
    /// `rgb(0, 100, 0)`
    pub const DARK_GREEN: Rgba = rgb!(0, 100, 0);

    /// Medium Aquamarine (`#66CDAA`)
    ///
    /// `rgb(102, 205, 170)`
    pub const MEDIUM_AQUAMARINE: Rgba = rgb!(102, 205, 170);

    /// Aqua (`#00FFFF`)
    ///
    /// `rgb(0, 255, 255)`
    pub const AQUA: Rgba = rgb!(0, 255, 255);

    /// Cyan (`#00FFFF`)
    ///
    /// `rgb(0, 255, 255)`
    pub const CYAN: Rgba = rgb!(0, 255, 255);

    /// Light Cyan (`#E0FFFF`)
    ///
    /// `rgb(224, 255, 255)`
    pub const LIGHT_CYAN: Rgba = rgb!(224, 255, 255);

    /// Pale Turquoise (`#AFEEEE`)
    ///
    /// `rgb(175, 238, 238)`
    pub const PALE_TURQUOISE: Rgba = rgb!(175, 238, 238);

    /// Aquamarine (`#7FFFD4`)
    ///
    /// `rgb(127, 255, 212)`
    pub const AQUAMARINE: Rgba = rgb!(127, 255, 212);

    /// Turquoise (`#40E0D0`)
    ///
    /// `rgb(64, 224, 208)`
    pub const TURQUOISE: Rgba = rgb!(64, 224, 208);

    /// Medium Turquoise (`#48D1CC`)
    ///
    /// `rgb(72, 209, 204)`
    pub const MEDIUM_TURQUOISE: Rgba = rgb!(72, 209, 204);

    /// Dark Turquoise (`#00CED1`)
    ///
    /// `rgb(0, 206, 209)`
    pub const DARK_TURQUOISE: Rgba = rgb!(0, 206, 209);

    /// Light Sea Green (`#20B2AA`)
    ///
    /// `rgb(32, 178, 170)`
    pub const LIGHT_SEA_GREEN: Rgba = rgb!(32, 178, 170);

    /// Cadet Blue (`#5F9EA0`)
    ///
    /// `rgb(95, 158, 160)`
    pub const CADET_BLUE: Rgba = rgb!(95, 158, 160);

    /// Dark Cyan (`#008B8B`)
    ///
    /// `rgb(0, 139, 139)`
    pub const DARK_CYAN: Rgba = rgb!(0, 139, 139);

    /// Teal (`#008080`)
    ///
    /// `rgb(0, 128, 128)`
    pub const TEAL: Rgba = rgb!(0, 128, 128);

    /// Light Steel Blue (`#B0C4DE`)
    ///
    /// `rgb(176, 196, 222)`
    pub const LIGHT_STEEL_BLUE: Rgba = rgb!(176, 196, 222);

    /// Powder Blue (`#B0E0E6`)
    ///
    /// `rgb(176, 224, 230)`
    pub const POWDER_BLUE: Rgba = rgb!(176, 224, 230);

    /// Light Blue (`#ADD8E6`)
    ///
    /// `rgb(173, 216, 230)`
    pub const LIGHT_BLUE: Rgba = rgb!(173, 216, 230);

    /// Sky Blue (`#87CEEB`)
    ///
    /// `rgb(135, 206, 235)`
    pub const SKY_BLUE: Rgba = rgb!(135, 206, 235);

    /// Light Sky Blue (`#87CEFA`)
    ///
    /// `rgb(135, 206, 250)`
    pub const LIGHT_SKY_BLUE: Rgba = rgb!(135, 206, 250);

    /// Deep Sky Blue (`#00BFFF`)
    ///
    /// `rgb(0, 191, 255)`
    pub const DEEP_SKY_BLUE: Rgba = rgb!(0, 191, 255);

    /// Dodger Blue (`#1E90FF`)
    ///
    /// `rgb(30, 144, 255)`
    pub const DODGER_BLUE: Rgba = rgb!(30, 144, 255);

    /// Cornflower Blue (`#6495ED`)
    ///
    /// `rgb(100, 149, 237)`
    pub const CORNFLOWER_BLUE: Rgba = rgb!(100, 149, 237);

    /// Steel Blue (`#4682B4`)
    ///
    /// `rgb(70, 130, 180)`
    pub const STEEL_BLUE: Rgba = rgb!(70, 130, 180);

    /// Royal Blue (`#4169E1`)
    ///
    /// `rgb(65, 105, 225)`
    pub const ROYAL_BLUE: Rgba = rgb!(65, 105, 225);

    /// Blue (`#0000FF`)
    ///
    /// `rgb(0, 0, 255)`
    pub const BLUE: Rgba = rgb!(0, 0, 255);

    /// Medium Blue (`#0000CD`)
    ///
    /// `rgb(0, 0, 205)`
    pub const MEDIUM_BLUE: Rgba = rgb!(0, 0, 205);

    /// Dark Blue (`#00008B`)
    ///
    /// `rgb(0, 0, 139)`
    pub const DARK_BLUE: Rgba = rgb!(0, 0, 139);

    /// Navy (`#000080`)
    ///
    /// `rgb(0, 0, 128)`
    pub const NAVY: Rgba = rgb!(0, 0, 128);

    /// Midnight Blue (`#191970`)
    ///
    /// `rgb(25, 25, 112)`
    pub const MIDNIGHT_BLUE: Rgba = rgb!(25, 25, 112);

    /// White (`#FFFFFF`)
    ///
    /// `rgb(255, 255, 255)`
    pub const WHITE: Rgba = rgb!(255, 255, 255);

    /// Snow (`#FFFAFA`)
    ///
    /// `rgb(255, 250, 250)`
    pub const SNOW: Rgba = rgb!(255, 250, 250);

    /// Honeydew (`#F0FFF0`)
    ///
    /// `rgb(240, 255, 240)`
    pub const HONEYDEW: Rgba = rgb!(240, 255, 240);

    /// Mint Cream (`#F5FFFA`)
    ///
    /// `rgb(245, 255, 250)`
    pub const MINT_CREAM: Rgba = rgb!(245, 255, 250);

    /// Azure (`#F0FFFF`)
    ///
    /// `rgb(240, 255, 255)`
    pub const AZURE: Rgba = rgb!(240, 255, 255);

    /// Alice Blue (`#F0F8FF`)
    ///
    /// `rgb(240, 248, 255)`
    pub const ALICE_BLUE: Rgba = rgb!(240, 248, 255);

    /// Ghost White (`#F8F8FF`)
    ///
    /// `rgb(248, 248, 255)`
    pub const GHOST_WHITE: Rgba = rgb!(248, 248, 255);

    /// White Smoke (`#F5F5F5`)
    ///
    /// `rgb(245, 245, 245)`
    pub const WHITE_SMOKE: Rgba = rgb!(245, 245, 245);

    /// Seashell (`#FFF5EE`)
    ///
    /// `rgb(255, 245, 238)`
    pub const SEASHELL: Rgba = rgb!(255, 245, 238);

    /// Beige (`#F5F5DC`)
    ///
    /// `rgb(245, 245, 220)`
    pub const BEIGE: Rgba = rgb!(245, 245, 220);

    /// Old Lace (`#FDF5E6`)
    ///
    /// `rgb(253, 245, 230)`
    pub const OLD_LACE: Rgba = rgb!(253, 245, 230);

    /// Floral White (`#FFFAF0`)
    ///
    /// `rgb(255, 250, 240)`
    pub const FLORAL_WHITE: Rgba = rgb!(255, 250, 240);

    /// Ivory (`#FFFFF0`)
    ///
    /// `rgb(255, 255, 240)`
    pub const IVORY: Rgba = rgb!(255, 255, 240);

    /// Antique White (`#FAEBD7`)
    ///
    /// `rgb(250, 235, 215)`
    pub const ANTIQUE_WHITE: Rgba = rgb!(250, 235, 215);

    /// Linen (`#FAF0E6`)
    ///
    /// `rgb(250, 240, 230)`
    pub const LINEN: Rgba = rgb!(250, 240, 230);

    /// Lavender Blush (`#FFF0F5`)
    ///
    /// `rgb(255, 240, 245)`
    pub const LAVENDER_BLUSH: Rgba = rgb!(255, 240, 245);

    /// Misty Rose (`#FFE4E1`)
    ///
    /// `rgb(255, 228, 225)`
    pub const MISTY_ROSE: Rgba = rgb!(255, 228, 225);

    /// Gainsboro (`#DCDCDC`)
    ///
    /// `rgb(220, 220, 220)`
    pub const GAINSBORO: Rgba = rgb!(220, 220, 220);

    /// Light Gray (`#D3D3D3`)
    ///
    /// `rgb(211, 211, 211)`
    pub const LIGHT_GRAY: Rgba = rgb!(211, 211, 211);

    /// Silver (`#C0C0C0`)
    ///
    /// `rgb(192, 192, 192)`
    pub const SILVER: Rgba = rgb!(192, 192, 192);

    /// Dark Gray (`#A9A9A9`)
    ///
    /// `rgb(169, 169, 169)`
    pub const DARK_GRAY: Rgba = rgb!(169, 169, 169);

    /// Gray (`#808080`)
    ///
    /// `rgb(128, 128, 128)`
    pub const GRAY: Rgba = rgb!(128, 128, 128);

    /// Dim Gray (`#696969`)
    ///
    /// `rgb(105, 105, 105)`
    pub const DIM_GRAY: Rgba = rgb!(105, 105, 105);

    /// Light Slate Gray (`#778899`)
    ///
    /// `rgb(119, 136, 153)`
    pub const LIGHT_SLATE_GRAY: Rgba = rgb!(119, 136, 153);

    /// Slate Gray (`#708090`)
    ///
    /// `rgb(112, 128, 144)`
    pub const SLATE_GRAY: Rgba = rgb!(112, 128, 144);

    /// Dark Slate Gray (`#2F4F4F`)
    ///
    /// `rgb(47, 79, 79)`
    pub const DARK_SLATE_GRAY: Rgba = rgb!(47, 79, 79);

    /// Black (`#000000`)
    ///
    /// `rgb(0, 0, 0)`
    pub const BLACK: Rgba = rgb!(0, 0, 0);
}

#[test]
fn test_hex_color() {
    fn f(n: u8) -> f32 {
        n as f32 / 255.0
    }
    assert_eq!(Rgba::new(f(0x11), f(0x22), f(0x33), f(0x44)), hex!(0x11223344));

    assert_eq!(web_colors::BLACK, hex!(0x00_00_00_FF));
    assert_eq!(web_colors::WHITE, hex!(0xFF_FF_FF_FF));
    assert_eq!(web_colors::WHITE, hex!(0xFF_FF_FF));
    assert_eq!(web_colors::WHITE, hex!(0xFFFFFF));
    assert_eq!(web_colors::WHITE, hex!(#FFFFFF));
    assert_eq!(web_colors::WHITE, hex!(FFFFFF));
    assert_eq!(web_colors::WHITE, hex!(0xFFFF));
    assert_eq!(web_colors::BLACK, hex!(0x000));
    assert_eq!(web_colors::BLACK, hex!(#000));
    assert_eq!(web_colors::BLACK, hex!(000));
}
