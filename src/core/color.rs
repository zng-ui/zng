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
pub fn rgb<C: Into<ColorFComponent>>(r: C, g: C, b: C) -> Color {
    rgba(r, g, b, 1.0)
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
pub fn rgba<C: Into<ColorFComponent>, A: Into<ColorFComponent>>(r: C, g: C, b: C, a: A) -> Color {
    Color::new(r.into().0, g.into().0, b.into().0, a.into().0)
}

/// [`rgb`] and [`rgba`] argument conversion helper.
pub struct ColorFComponent(pub f32);
impl From<f32> for ColorFComponent {
    fn from(f: f32) -> Self {
        ColorFComponent(f)
    }
}
impl From<u8> for ColorFComponent {
    fn from(u: u8) -> Self {
        ColorFComponent(f32::from(u) / 255.)
    }
}
