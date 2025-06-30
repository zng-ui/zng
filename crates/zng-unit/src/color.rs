use std::{fmt, ops};

use crate::{EQ_GRANULARITY, Factor, FactorPercent, about_eq, about_eq_hash};

/// RGB + alpha.
///
/// # Equality
///
/// Equality is determined using [`about_eq`] with `0.00001` granularity.
///
/// [`about_eq`]: crate::about_eq
#[repr(C)]
#[derive(Default, Copy, Clone, serde::Serialize, serde::Deserialize)]
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
        about_eq(self.red, other.red, EQ_GRANULARITY)
            && about_eq(self.green, other.green, EQ_GRANULARITY)
            && about_eq(self.blue, other.blue, EQ_GRANULARITY)
            && about_eq(self.alpha, other.alpha, EQ_GRANULARITY)
    }
}
impl Eq for Rgba {}
impl std::hash::Hash for Rgba {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        about_eq_hash(self.red, EQ_GRANULARITY, state);
        about_eq_hash(self.green, EQ_GRANULARITY, state);
        about_eq_hash(self.blue, EQ_GRANULARITY, state);
        about_eq_hash(self.alpha, EQ_GRANULARITY, state);
    }
}
impl Rgba {
    /// New from RGB of a the same type and A that can be of a different type.
    pub fn new<C: Into<RgbaComponent>, A: Into<RgbaComponent>>(red: C, green: C, blue: C, alpha: A) -> Rgba {
        Rgba {
            red: red.into().0,
            green: green.into().0,
            blue: blue.into().0,
            alpha: alpha.into().0,
        }
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
            let rgba = (rgb << 8) | a;
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

// Util
fn clamp_normal(i: f32) -> f32 {
    i.clamp(0.0, 1.0)
}

/// Color functions argument conversion helper.
///
/// Don't use this value directly, if a function takes `Into<RgbaComponent>` you can use one of the
/// types this converts from:
///
/// * `f32`, `f64` and [`Factor`] for a value in the `0.0` to `1.0` range.
/// * `u8` for a value in the `0` to `255` range.
/// * [`FactorPercent`] for a percentage value.
///
/// [`Factor`]: crate::Factor
/// [`FactorPercent`]: crate::FactorPercent
#[derive(Clone, Copy)]
pub struct RgbaComponent(pub f32);
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
