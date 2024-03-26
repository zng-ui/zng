use zero_ui_color::COLOR_SCHEME_VAR;

use crate::prelude::*;

/// Defines the preferred color scheme in the widget and descendants.
#[property(CONTEXT, default(COLOR_SCHEME_VAR))]
pub fn color_scheme(child: impl UiNode, pref: impl IntoVar<ColorScheme>) -> impl UiNode {
    with_context_var(child, COLOR_SCHEME_VAR, pref)
}
