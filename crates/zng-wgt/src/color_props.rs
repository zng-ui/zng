use zng_color::{
    COLOR_SCHEME_VAR,
    colors::{ACCENT_COLOR_VAR, BASE_COLOR_VAR},
};

use crate::prelude::*;

/// Defines the preferred color scheme in the widget and descendants.
///
/// Sets the [`COLOR_SCHEME_VAR`].
#[property(CONTEXT, default(COLOR_SCHEME_VAR))]
pub fn color_scheme(child: impl IntoUiNode, pref: impl IntoVar<ColorScheme>) -> UiNode {
    with_context_var(child, COLOR_SCHEME_VAR, pref)
}

/// Defines the preferred accent color in the widget and descendants.
///
/// This is a a high saturation color used to highlight important UI elements, like the focused text input.
///
/// Sets the [`ACCENT_COLOR_VAR`].
#[property(CONTEXT, default(ACCENT_COLOR_VAR))]
pub fn accent_color(child: impl IntoUiNode, color: impl IntoVar<LightDark>) -> UiNode {
    with_context_var(child, ACCENT_COLOR_VAR, color)
}

/// Defines the seed color used by widgets to derive background, non active border.
///
/// Usually the color is used directly for background fill and highlighted for others.
/// 
/// Sets the [`BASE_COLOR_VAR`].
#[property(CONTEXT, default(BASE_COLOR_VAR))]
pub fn base_color(child: impl IntoUiNode, color: impl IntoVar<LightDark>) -> UiNode {
    with_context_var(child, BASE_COLOR_VAR, color)
}
