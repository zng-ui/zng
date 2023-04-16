//! Link widget, properties and nodes..

use crate::prelude::new_widget::*;

/// A clickable inline element.
#[widget($crate::widgets::Link)]
pub struct Link(crate::widgets::Button);
impl Link {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            style_fn = STYLE_VAR;
        }
    }
}

context_var! {
    /// Link style in a context.
    ///
    /// Is the [`DefaultStyle!`] by default.
    ///
    /// [`DefaultStyle!`]: struct@DefaultStyle
    pub static STYLE_VAR: StyleFn = StyleFn::new(|_| DefaultStyle!());
}

/// Sets the link style in a context, the parent style is fully replaced.
#[property(CONTEXT, default(STYLE_VAR))]
pub fn replace_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    with_context_var(child, STYLE_VAR, style)
}

/// Extends the button style in a context, the parent style is used, properties of the same name set in
/// `style` override the parent style.
#[property(CONTEXT, default(StyleFn::nil()))]
pub fn extend_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    style::with_style_extension(child, STYLE_VAR, style)
}

/// Link default style.
#[widget($crate::widgets::link::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            text::txt_color = color_scheme_map(colors::LIGHT_BLUE, colors::BLUE);
            crate::properties::cursor = CursorIcon::Hand;

            when *#is_cap_hovered {
                text::underline = 1, LineStyle::Solid;
            }

            when *#is_pressed {
                text::txt_color = color_scheme_map(colors::YELLOW, colors::BROWN);
            }

            when *#is_disabled {
                saturate = false;
                child_opacity = 50.pct();
                cursor = CursorIcon::NotAllowed;
            }
        }
    }
}
