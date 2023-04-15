//! Tooltip widget, properties and nodes..

use crate::prelude::new_widget::*;

/// A tooltip container.
///
/// Can be set on the [`tooltip`] property.
///
/// [`tooltip`]: fn@crate::properties::tooltip
#[widget($crate::widgets::Tip {
    ($child:expr) => {
        child = $child;
    };
})]
pub struct Tip(StyleMix<FocusableMix<Container>>);
impl Tip {
    fn on_start(&mut self) {
        widget_set! {
            self;
            hit_test_mode = false;
            style_fn = STYLE_VAR;
        }
    }

    widget_impl! {
        /// If the tooltip can be interacted with the mouse.
        ///
        /// This is disabled by default.
        pub fn crate::properties::hit_test_mode(mode: impl IntoVar<HitTestMode>);
    }
}

context_var! {
    /// Tip style in a context.
    ///
    /// Is the [`DefaultStyle!`] by default.
    ///
    /// [`DefaultStyle!`]: struct@DefaultStyle
    pub static STYLE_VAR: StyleFn = StyleFn::new(|_| DefaultStyle!());

    /// Idle background dark and light color.
    pub static BASE_COLORS_VAR: ColorPair = (rgb(20, 20, 20), rgb(235, 235, 235));
}

/// Sets the [`BASE_COLORS_VAR`] that is used to compute all background and border colors in the tip style.
#[property(CONTEXT, default(BASE_COLORS_VAR))]
pub fn base_colors(child: impl UiNode, color: impl IntoVar<ColorPair>) -> impl UiNode {
    with_context_var(child, BASE_COLORS_VAR, color)
}

/// Sets the tip style in a context, the parent style is fully replaced.
#[property(CONTEXT, default(STYLE_VAR))]
pub fn replace_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    with_context_var(child, STYLE_VAR, style)
}

/// Extends the tip style in a context, the parent style is used, properties of the same name set in
/// `style` override the parent style.
#[property(CONTEXT, default(StyleFn::nil()))]
pub fn extend_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    style::with_style_extension(child, STYLE_VAR, style)
}

/// Tip default style.
#[widget($crate::widgets::tip::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn on_start(&mut self) {
        widget_set! {
            self;
            crate::properties::padding = (2, 4);
            crate::properties::corner_radius = 3;
            crate::properties::background_color = color_scheme_pair(BASE_COLORS_VAR);
            crate::widgets::text::font_size = 10.pt();
            crate::properties::border = {
                widths: 1.px(),
                sides: color_scheme_highlight(BASE_COLORS_VAR, 0.5).map_into()
            };
        }
    }
}
