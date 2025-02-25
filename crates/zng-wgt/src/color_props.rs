use zng_color::{
    COLOR_SCHEME_VAR,
    colors::{ACCENT_COLOR_VAR, BASE_COLOR_VAR},
};

use crate::prelude::*;

/// Defines the preferred color scheme in the widget and descendants.
///
/// Sets the [`COLOR_SCHEME_VAR`].
#[property(CONTEXT, default(COLOR_SCHEME_VAR))]
pub fn color_scheme(child: impl UiNode, pref: impl IntoVar<ColorScheme>) -> impl UiNode {
    with_context_var(child, COLOR_SCHEME_VAR, pref)
}

/// Defines the preferred accent color in the widget and descendants.
///
/// The is a distinct background/fill color that contrasts with the foreground text color.
///
/// Sets the [`COLOR_SCHEME_VAR`].
#[property(CONTEXT, default(ACCENT_COLOR_VAR))]
pub fn accent_color(child: impl UiNode, color: impl IntoVar<LightDark>) -> impl UiNode {
    with_context_var(child, ACCENT_COLOR_VAR, color)
}

/// Defines the seed color used by widgets to derive background, non active border.
///
/// Usually the color is used directly for background fill and highlighted for others.
#[property(CONTEXT, default(BASE_COLOR_VAR))]
pub fn base_color(child: impl UiNode, color: impl IntoVar<LightDark>) -> impl UiNode {
    with_context_var(child, BASE_COLOR_VAR, color)
}
