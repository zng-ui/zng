use super::types::Angle;

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
/// use zero_ui::core::types::rgb;
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
/// use zero_ui::core::types::rgba;
///
/// let half_red = rgba(255, 0, 0, 0.5);
/// let green = rgba(0.0, 1.0, 0.0, 1.0);
/// let transparent = rgba(0, 0, 0, 0);
/// ```
pub fn rgba<C: Into<RgbaComponent>, A: Into<RgbaComponent>>(red: C, green: C, blue: C, alpha: A) -> Color {
    Color::new(red.into().0, green.into().0, blue.into().0, alpha.into().0)
}

pub fn hsl(hue: Angle, saturation: f32, lightness: f32) -> Color {
    hsla(hue, saturation, lightness, 1.0)
}

pub fn hsla<A: Into<RgbaComponent>>(hue: Angle, saturation: f32, lightness: f32, alpha: A) -> Color {
    if saturation <= f32::EPSILON {
        // greyscale
        rgba(lightness, lightness, lightness, alpha)
    } else {
        let hue = hue.to_degrees() / 360.0;
        let q = if lightness < 0.5 {
            lightness * (1.0 + saturation)
        } else {
            lightness + saturation - lightness * saturation
        };
        let p = 2.0 * lightness - q;

        let hue_to_rgb = |mut t: f32| {
            if t < 0.0 {
                t += 1.0;
            } else if t > 1.0 {
                t -= 1.0;
            }

            if t < 1.0 / 6.0 {
                p + (q - p) * 6.0 * t
            } else if t < 0.5 {
                q
            } else if t < 2.0 / 3.0 {
                p + (q - p) * 6.0 * (2.0 / 3.0 - t)
            } else {
                p
            }
        };

        rgba(hue_to_rgb(hue + 1.0/3.0), hue_to_rgb(hue), hue_to_rgb(hue - 1.0/3.0), alpha)
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    use crate::core::types::Units;
    #[test]
    fn hsl_red() {
        assert_eq!(hsl(0.0.deg(), 1.0, 0.5), rgb(1.0, 0.0, 0.0) )
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
