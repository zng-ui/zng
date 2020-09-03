use super::units::*;

pub type Color = webrender::api::ColorF;

/// Opaque RGB color.
///
/// # Arguments
///
/// The arguments can either be `f32` in the `0.0..=1.0` range or
/// `u8` in the `0..=255` range.
///
/// # Example
/// ```
/// use zero_ui::core::color::rgb;
///
/// let red = rgb(1.0, 0.0, 0.0);
/// let green = rgb(0, 255, 0);
/// ```
pub fn rgb<C: Into<RgbaComponent>>(red: C, green: C, blue: C) -> Color {
    rgba(red, green, blue, 1.0)
}

/// RGBA color.
///
/// # Arguments
///
/// The arguments can either be floating pointer in the `0.0..=1.0` range or
/// integers in the `0..=255` range.
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
pub fn rgba<C: Into<RgbaComponent>, A: Into<RgbaComponent>>(red: C, green: C, blue: C, alpha: A) -> Color {
    Color::new(red.into().0, green.into().0, blue.into().0, alpha.into().0)
}

pub fn hsl<H: Into<AngleDegree>, N: Into<FactorNormal>>(hue: H, saturation: N, lightness: N) -> Color {
    hsla(hue, saturation, lightness, 1.0)
}

pub fn hsla<H: Into<AngleDegree>, N: Into<FactorNormal>, A: Into<FactorNormal>>(hue: H, saturation: N, lightness: N, alpha: A) -> Color {
    let saturation = saturation.into().0;
    let lightness = lightness.into().0;
    let alpha = alpha.into().0;

    if saturation <= f32::EPSILON {
        return rgba(lightness, lightness, lightness, alpha);
    }

    let hue = hue.into().0;
    let c = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let hp = hue / 60.0;
    let x = c * (1.0 - ((hp % 2.0) - 1.0).abs());
    let rgb = if hp <= 1.0 {
        [c, x, 0.0]
    } else if hp <= 2.0 {
        [x, c, 0.0]
    } else if hp <= 3.0 {
        [0.0, c, x]
    } else if hp <= 4.0 {
        [0.0, x, c]
    } else if hp <= 5.0 {
        [x, 0.0, c]
    } else if hp <= 6.0 {
        [c, 0.0, x]
    } else {
        [0.0, 0.0, 0.0]
    };
    let m = lightness - c * 0.5;

    let f = |i: usize| ((rgb[i] + m) * 255.0).round() / 255.0;

    rgba(f(0), f(1), f(2), alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsl_red() {
        assert_eq!(hsl(0.0.deg(), 100.pct(), 50.pct()), rgb(1.0, 0.0, 0.0))
    }

    #[test]
    fn hsl_color() {
        assert_eq!(hsl(91.0.deg(), 1.0, 0.5), rgb(123, 255, 0))
    }
}

/// [`rgb`] and [`rgba`] argument conversion helper.
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
