use crate::prelude::new_widget::*;

/// A clickable inline element.
#[widget($crate::widgets::link)]
pub mod link {
    #[doc(inline)]
    pub use super::vis;

    inherit!(crate::widgets::button);

    #[doc(no_inline)]
    pub use crate::widgets::text::underline_skip;

    properties! {
        /// Link style.
        ///
        /// Set to [`vis::STYLE_VAR`] by default, setting this property directly completely replaces the link style,
        /// see [`vis::replace_style`] and [`vis::extend_style`] for other ways of modifying the link style.
        style_gen = vis::STYLE_VAR;
    }
}

/// Link style, visual properties and context vars.
pub mod vis {
    use super::*;

    context_var! {
        /// Link style in a context.
        ///
        /// Is the [`default_style!`] by default.
        ///
        /// [`default_style!`]: mod@default_style
        pub static STYLE_VAR: StyleGenerator = StyleGenerator::new(|_, _| default_style!());
    }

    /// Sets the link style in a context, the parent style is fully replaced.
    #[property(CONTEXT, default(STYLE_VAR))]
    pub fn replace_style(child: impl UiNode, style: impl IntoVar<StyleGenerator>) -> impl UiNode {
        with_context_var(child, STYLE_VAR, style)
    }

    /// Extends the button style in a context, the parent style is used, properties of the same name set in
    /// `style` override the parent style.
    #[property(CONTEXT, default(StyleGenerator::nil()))]
    pub fn extend_style(child: impl UiNode, style: impl IntoVar<StyleGenerator>) -> impl UiNode {
        style_mixin::with_style_extension(child, STYLE_VAR, style)
    }

    /// Link default style.
    #[widget($crate::widgets::link::vis::default_style)]
    pub mod default_style {
        use super::*;

        use crate::widgets::text;

        #[doc(no_inline)]
        pub use text::underline_skip;

        inherit!(style);

        properties! {
            /// Link text color.
            pub text::txt_color = color_scheme_map(colors::LIGHT_BLUE, colors::BLUE);

            /// Link cursor.
            pub crate::properties::cursor = CursorIcon::Hand;

            /// When the pointer device is over this link.
            when *#is_cap_hovered {
                text::underline = 1, LineStyle::Solid;
            }
            
            /// When the pointer device is pressed on this link.
            when *#is_pressed {
                text::txt_color = color_scheme_map(colors::YELLOW, colors::BROWN);
            }

            /// When the button is disabled.
            when *#is_disabled {
                saturate = false;
                child_opacity = 50.pct();
                cursor = CursorIcon::NotAllowed;
            }
        }
    }
}
