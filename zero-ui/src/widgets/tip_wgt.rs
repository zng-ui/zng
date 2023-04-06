use crate::prelude::new_widget::*;

/// A tooltip container.
/// 
/// Can be set on the [`tooltip`] property.
/// 
/// [`tooltip`]: fn@crate::properties::tooltip;
#[widget($crate::widgets::tip {
    ($child:expr) => {
        child = $child;
    };
})]
pub mod tip {
    use super::*;

    #[doc(inline)]
    pub use super::vis;

    inherit!(container);
    inherit!(focusable_mixin);
    inherit!(style_mixin);

    properties! {
        /// If the tooltip can be interacted with the mouse.
        /// 
        /// This is disabled by default.
        pub crate::properties::hit_test_mode = false;

        /// Tooltip style.
        ///
        /// Set to [`vis::STYLE_VAR`] by default, setting this property directly completely replaces the tip style,
        /// see [`vis::replace_style`] and [`vis::extend_style`] for other ways of modifying the button style.
        style_gen = vis::STYLE_VAR;
    }
}

/// Tip style, visual properties and context vars.
pub mod vis {
    use super::*;

    context_var! {
        /// Tip style in a context.
        ///
        /// Is the [`default_style!`] by default.
        ///
        /// [`default_style!`]: mod@default_style
        pub static STYLE_VAR: StyleGenerator = StyleGenerator::new(|_| default_style!());

        /// Idle background dark and light color.
        pub static BASE_COLORS_VAR: ColorPair = (rgba(0, 0, 0, 0.8), rgba(255, 255, 255, 0.8));
    }

    /// Sets the [`BASE_COLORS_VAR`] that is used to compute all background and border colors in the tip style.
    #[property(CONTEXT, default(BASE_COLORS_VAR))]
    pub fn base_colors(child: impl UiNode, color: impl IntoVar<ColorPair>) -> impl UiNode {
        with_context_var(child, BASE_COLORS_VAR, color)
    }

    /// Sets the tip style in a context, the parent style is fully replaced.
    #[property(CONTEXT, default(STYLE_VAR))]
    pub fn replace_style(child: impl UiNode, style: impl IntoVar<StyleGenerator>) -> impl UiNode {
        with_context_var(child, STYLE_VAR, style)
    }

    /// Extends the tip style in a context, the parent style is used, properties of the same name set in
    /// `style` override the parent style.
    #[property(CONTEXT, default(StyleGenerator::nil()))]
    pub fn extend_style(child: impl UiNode, style: impl IntoVar<StyleGenerator>) -> impl UiNode {
        style_mixin::with_style_extension(child, STYLE_VAR, style)
    }

    /// Tip default style.
    #[widget($crate::widgets::tip::vis::default_style)]
    pub mod default_style {
        use super::*;

        inherit!(style);

        properties! {
            /// Tip padding.
            ///
            /// Is `(2, 4)` by default.
            pub crate::properties::padding = (2, 4);

            /// Tip corner radius.
            ///
            /// Is `3` by default.
            pub crate::properties::corner_radius = 3;

            /// Tip base dark and light colors.
            ///
            /// All other tip style colors are derived from this pair.
            pub super::base_colors;

            /// Tip background.
            pub crate::properties::background_color = color_scheme_pair(BASE_COLORS_VAR);

            /// Tip border.
            ///
            /// Is widths `1`.
            pub crate::properties::border = {
                widths: 1.px(),
                sides: color_scheme_highlight(BASE_COLORS_VAR, 0.2).map_into()
            };

            /// Tip shadow.
            /// 
            /// Is 
            pub crate::properties::filters::drop_shadow = {
                offset: (0, 0),
                blur_radius: 2,
                color: colors::BLACK,
            };
        }
    }
}
