use crate::prelude::new_widget::*;

/// A clickable element.
#[widget($crate::widgets::button)]
pub mod button {
    use super::*;

    #[doc(inline)]
    pub use super::vis;

    inherit!(container);
    inherit!(focusable_mixin);
    inherit!(style_mixin);

    properties! {
        /// Button click event.
        ///
        /// # Examples
        ///
        /// ```
        /// # use zero_ui::prelude::*;
        /// # let _scope = App::blank();
        /// #
        /// button! {
        ///     on_click = hn!(|ctx, args: &ClickArgs| {
        ///         assert!(args.is_primary());
        ///         println!("button {:?} clicked!", ctx.path.widget_id());
        ///     });
        ///     child = text("Click Me!");
        /// }
        /// # ;
        /// ```
        pub crate::properties::events::gesture::on_click;

        /// If pointer interaction with other widgets is blocked while the button is pressed.
        pub crate::properties::capture_mouse = true;

        /// Button style.
        ///
        /// Set to [`vis::STYLE_VAR`] by default, setting this property directly completely replaces the button style,
        /// see [`vis::replace_style`] and [`vis::extend_style`] for other ways of modifying the button style.
        style_gen = vis::STYLE_VAR;
    }
}

/// Button style, visual properties and context vars.
pub mod vis {
    use super::*;

    context_var! {
        /// Button style in a context.
        ///
        /// Is the [`default_style!`] by default.
        ///
        /// [`default_style!`]: mod@default_style
        pub static STYLE_VAR: StyleGenerator = StyleGenerator::new(|_, _| default_style!());

        /// Idle background dark and light color.
        pub static BASE_COLORS_VAR: ColorPair = (rgb(0.18, 0.18, 0.18), rgb(0.82, 0.82, 0.82));
    }

    /// Sets the [`BASE_COLORS_VAR`] that is used to compute all background and border colors in the button style.
    #[property(CONTEXT, default(BASE_COLORS_VAR))]
    pub fn base_colors(child: impl UiNode, color: impl IntoVar<ColorPair>) -> impl UiNode {
        with_context_var(child, BASE_COLORS_VAR, color)
    }

    /// Sets the button style in a context, the parent style is fully replaced.
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

    /// Create a [`color_scheme_highlight`] of `0.08`.
    pub fn color_scheme_hovered(pair: impl IntoVar<ColorPair>) -> impl Var<Rgba> {
        color_scheme_highlight(pair, 0.08)
    }

    /// Create a [`color_scheme_highlight`] of `0.16`.
    pub fn color_scheme_pressed(pair: impl IntoVar<ColorPair>) -> impl Var<Rgba> {
        color_scheme_highlight(pair, 0.16)
    }

    /// Button default style.
    #[widget($crate::widgets::button::vis::default_style)]
    pub mod default_style {
        use super::*;

        inherit!(style);

        properties! {
            /// Button padding.
            ///
            /// Is `(7, 15)` by default.
            pub crate::properties::padding = (7, 15);

            /// Button corner radius.
            ///
            /// Is `4` by default.
            pub crate::properties::corner_radius = 4;

            /// Button content align.
            pub crate::properties::child_align as content_align = Align::CENTER;

            /// Button base dark and light colors.
            ///
            /// All other button style colors are derived from this pair.
            pub super::base_colors;

            /// Button background.
            #[easing(300.ms())]
            pub crate::properties::background_color = color_scheme_pair(BASE_COLORS_VAR);

            /// Button border.
            ///
            /// Is widths `1`.
            #[easing(300.ms())]
            pub crate::properties::border = {
                widths: 1,
                sides: color_scheme_pair(BASE_COLORS_VAR).map_into()
            };

            /// When the pointer device is over this button.
            when *#is_cap_hovered {
                background_color = color_scheme_hovered(BASE_COLORS_VAR);
                border = {
                    widths: 1,
                    sides: color_scheme_pressed(BASE_COLORS_VAR).map_into(),
                };
            }

            /// When the button is pressed in a way that press release will cause a button click.
            when *#is_pressed  {
                background_color = color_scheme_pressed(BASE_COLORS_VAR);
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
